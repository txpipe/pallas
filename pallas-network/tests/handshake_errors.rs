//! A server facade whose handshake fails must close the connection and leave
//! no task of it running when `accept` returns.

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use pallas_network::facades::{DEFAULT_HANDSHAKE_TIMEOUT, Error, PeerServer};
use pallas_network::miniprotocols::MAINNET_MAGIC;
use pallas_network::multiplexer::Header;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A handshake segment whose message tag is not a handshake message.
const MALFORMED_PROPOSAL: [u8; 3] = [0x82, 0x07, 0x00];

/// How long the initiator waits to see the server close the connection.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long connecting, the initiator and `accept` get to finish together.
const ACCEPT_TIMEOUT: Duration = Duration::from_secs(5);

/// What the initiator does once connected.
enum Initiator {
    SendMalformed,
    Disconnect,
}

/// Act as `initiator` on `stream`. Returns the stream when it stays open.
async fn initiate<S>(mut stream: S, initiator: Initiator) -> Option<S>
where
    S: AsyncWrite + Unpin,
{
    match initiator {
        Initiator::SendMalformed => {
            let header: [u8; 8] = Header {
                protocol: 0,
                timestamp: 0,
                payload_len: MALFORMED_PROPOSAL.len() as u16,
            }
            .into();
            stream.write_all(&header).await.unwrap();
            stream.write_all(&MALFORMED_PROPOSAL).await.unwrap();
            Some(stream)
        }
        Initiator::Disconnect => None,
    }
}

/// Run `accept` against an initiator connecting through `connect`. Checks that
/// `accept` fails with a handshake protocol error, that no task of the
/// connection outlives it, and that the server closed the connection.
async fn assert_handshake_error_closes<S, T>(
    connect: impl Future<Output = S>,
    accept: impl Future<Output = Result<T, Error>>,
    initiator: Initiator,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let tasks = tokio::runtime::Handle::current().metrics();
    let before = tasks.num_alive_tasks();

    // A stalled teardown fails the test instead of hanging it.
    let (accepted, stream) = tokio::time::timeout(ACCEPT_TIMEOUT, async {
        tokio::join!(accept, async { initiate(connect.await, initiator).await })
    })
    .await
    .expect("accept does not return after a handshake error");

    assert!(
        matches!(accepted, Err(Error::HandshakeProtocol(_))),
        "expected a handshake protocol error"
    );
    assert_eq!(
        tasks.num_alive_tasks(),
        before,
        "a task of the failed connection outlives accept"
    );

    if let Some(mut stream) = stream {
        // End of stream or a reset both mean the server closed the connection.
        let mut next = [0u8; 1];
        let read = tokio::time::timeout(CLOSE_TIMEOUT, stream.read(&mut next))
            .await
            .expect("the server keeps the connection open after a handshake error");
        assert!(
            !matches!(read, Ok(n) if n > 0),
            "the server wrote after a handshake error"
        );
    }
}

async fn peer_server_handshake_error(initiator: Initiator) {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    assert_handshake_error_closes(
        async { TcpStream::connect(addr).await.unwrap() },
        PeerServer::accept_with_timeout(&listener, MAINNET_MAGIC, DEFAULT_HANDSHAKE_TIMEOUT),
        initiator,
    )
    .await;
}

#[tokio::test]
async fn peer_server_closes_after_a_malformed_proposal() {
    peer_server_handshake_error(Initiator::SendMalformed).await;
}

#[tokio::test]
async fn peer_server_closes_when_the_initiator_disconnects() {
    peer_server_handshake_error(Initiator::Disconnect).await;
}

#[cfg(unix)]
mod unix {
    use super::*;

    use pallas_network::facades::{DmqServer, NodeServer};
    use tokio::net::{UnixListener, UnixStream};

    /// Bind a Unix listener on a fresh socket path for `name`.
    fn listener(name: &str) -> (UnixListener, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("pallas-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        (listener, path)
    }

    async fn node_server_handshake_error(name: &str, initiator: Initiator) {
        let (listener, path) = listener(name);

        assert_handshake_error_closes(
            async { UnixStream::connect(&path).await.unwrap() },
            NodeServer::accept_with_timeout(&listener, MAINNET_MAGIC, DEFAULT_HANDSHAKE_TIMEOUT),
            initiator,
        )
        .await;

        let _ = std::fs::remove_file(&path);
    }

    async fn dmq_server_handshake_error(name: &str, initiator: Initiator) {
        let (listener, path) = listener(name);

        assert_handshake_error_closes(
            async { UnixStream::connect(&path).await.unwrap() },
            DmqServer::accept_with_timeout(&listener, MAINNET_MAGIC, DEFAULT_HANDSHAKE_TIMEOUT),
            initiator,
        )
        .await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn node_server_closes_after_a_malformed_proposal() {
        node_server_handshake_error("hs-node-bad", Initiator::SendMalformed).await;
    }

    #[tokio::test]
    async fn node_server_closes_when_the_initiator_disconnects() {
        node_server_handshake_error("hs-node-eof", Initiator::Disconnect).await;
    }

    #[tokio::test]
    async fn dmq_server_closes_after_a_malformed_proposal() {
        dmq_server_handshake_error("hs-dmq-bad", Initiator::SendMalformed).await;
    }

    #[tokio::test]
    async fn dmq_server_closes_when_the_initiator_disconnects() {
        dmq_server_handshake_error("hs-dmq-eof", Initiator::Disconnect).await;
    }
}
