//! A client facade's handshake has no deadline of its own, so a caller bounds
//! it with a timeout. When that fires mid-handshake, no task of the connection
//! keeps running and the connection closes.

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use pallas_network::facades::{Error, PeerClient};
use pallas_network::miniprotocols::MAINNET_MAGIC;
use pallas_network::multiplexer::Header;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::TcpListener;

/// How long the caller lets the handshake run before giving up on it.
const CANCEL_AFTER: Duration = Duration::from_millis(200);

/// How long the cancelled call gets to close the connection and stop its tasks.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(5);

/// Read the client's proposal from `stream` and never answer it.
async fn read_proposal<S>(mut stream: S) -> S
where
    S: AsyncRead + Unpin,
{
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await.unwrap();
    let header = Header::from(&header[..]);
    let mut proposal = vec![0u8; header.payload_len as usize];
    stream.read_exact(&mut proposal).await.unwrap();

    stream
}

/// Run a client `call` under a short timeout against a responder accepting
/// through `accept` that reads the proposal and never answers. Checks that the
/// timeout fired, that the client closed the connection and that no task of
/// the connection outlives the cancelled call.
async fn cancel_mid_handshake<S, T>(
    accept: impl Future<Output = S>,
    call: impl Future<Output = Result<T, Error>>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let tasks = tokio::runtime::Handle::current().metrics();
    let before = tasks.num_alive_tasks();

    let (outcome, mut stream) = tokio::join!(tokio::time::timeout(CANCEL_AFTER, call), async {
        read_proposal(accept.await).await
    });
    assert!(outcome.is_err(), "the call returned without a reply");

    // End of stream or a reset both mean the client closed the connection.
    let mut next = [0u8; 1];
    let read = tokio::time::timeout(SETTLE_TIMEOUT, stream.read(&mut next))
        .await
        .expect("the cancelled call keeps the connection open");
    assert!(
        !matches!(read, Ok(n) if n > 0),
        "the client wrote after its proposal"
    );

    // An aborted task is gone once the runtime polls it again.
    tokio::time::timeout(SETTLE_TIMEOUT, async {
        while tasks.num_alive_tasks() != before {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("a task of the connection outlives the cancelled call");
}

async fn tcp_listener() -> TcpListener {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap()
}

#[tokio::test]
async fn peer_client_connect_stops_when_cancelled() {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    cancel_mid_handshake(
        async { listener.accept().await.unwrap().0 },
        PeerClient::connect(addr, MAINNET_MAGIC),
    )
    .await;
}

#[tokio::test]
async fn peer_client_query_stops_when_cancelled() {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    cancel_mid_handshake(
        async { listener.accept().await.unwrap().0 },
        PeerClient::handshake_query(addr, MAINNET_MAGIC),
    )
    .await;
}

#[cfg(unix)]
mod unix {
    use super::*;

    use pallas_network::facades::{DmqClient, NodeClient};
    use pallas_network::multiplexer::Bearer;
    use tokio::net::UnixListener;

    /// Bind a Unix listener on a fresh socket path for `name`.
    fn listener(name: &str) -> (UnixListener, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("pallas-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        (listener, path)
    }

    #[tokio::test]
    async fn node_client_connect_stops_when_cancelled() {
        let (listener, path) = listener("cancel-node");

        cancel_mid_handshake(
            async { listener.accept().await.unwrap().0 },
            NodeClient::connect(&path, MAINNET_MAGIC),
        )
        .await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn node_client_query_stops_when_cancelled() {
        let (listener, path) = listener("cancel-node-query");

        cancel_mid_handshake(async { listener.accept().await.unwrap().0 }, async {
            let bearer = Bearer::connect_unix(&path).await.unwrap();
            NodeClient::handshake_query(bearer, MAINNET_MAGIC).await
        })
        .await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn dmq_client_connect_stops_when_cancelled() {
        let (listener, path) = listener("cancel-dmq");

        cancel_mid_handshake(
            async { listener.accept().await.unwrap().0 },
            DmqClient::connect(&path, MAINNET_MAGIC),
        )
        .await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn dmq_client_query_stops_when_cancelled() {
        let (listener, path) = listener("cancel-dmq-query");

        cancel_mid_handshake(async { listener.accept().await.unwrap().0 }, async {
            let bearer = Bearer::connect_unix(&path).await.unwrap();
            DmqClient::handshake_query(bearer, MAINNET_MAGIC).await
        })
        .await;

        let _ = std::fs::remove_file(&path);
    }
}
