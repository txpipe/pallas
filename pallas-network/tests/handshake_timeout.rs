//! A server facade whose peer connects but never completes the handshake must
//! give up at the deadline, close the connection and leave no task running.
//! The deadline starts once the peer connects, and `accept` keeps no deadline.

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};

use pallas_network::facades::{DEFAULT_HANDSHAKE_TIMEOUT, Error, PeerClient, PeerServer};
use pallas_network::miniprotocols::MAINNET_MAGIC;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};

/// Handshake deadline given to `accept_with_timeout` in these tests.
const DEADLINE: Duration = Duration::from_millis(200);

/// Slack allowed past the deadline before `accept_with_timeout` must return.
const MARGIN: Duration = Duration::from_secs(2);

/// How long the initiator waits to see the server close the connection.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// Deadline for the tests where the initiator connects late but then completes
/// the handshake; long enough for a handshake on a loaded machine.
const LATE_DEADLINE: Duration = Duration::from_millis(500);

/// How long those initiators wait before connecting, well past the deadline.
const LATE_CONNECT: Duration = Duration::from_millis(1500);

/// Await `accept` while its initiator stays connected and silent, letting the
/// paused clock run far past [`DEFAULT_HANDSHAKE_TIMEOUT`]. Panics if `accept`
/// returns, which only a deadline could make it do.
async fn assert_still_waiting<T>(accept: impl Future<Output = Result<T, Error>>) {
    let outcome = tokio::time::timeout(DEFAULT_HANDSHAKE_TIMEOUT * 6, accept).await;

    if let Ok(accepted) = outcome {
        let ended = match accepted {
            Ok(_) => "completed".to_string(),
            Err(err) => format!("failed with {err:?}"),
        };
        panic!("accept {ended} against a silent initiator; it must keep waiting");
    }
}

/// Run `accept` against an initiator that connects through `connect` and then
/// sends nothing. Checks that `accept` times out within the deadline plus a
/// margin, that no task of the connection outlives it, and that the server
/// closed the connection.
async fn assert_silent_peer_times_out<S, T>(
    connect: impl Future<Output = S>,
    accept: impl Future<Output = Result<T, Error>>,
) where
    S: AsyncRead + Unpin,
{
    let tasks = tokio::runtime::Handle::current().metrics();
    let before = tasks.num_alive_tasks();

    let timed = async {
        let accepted = accept.await;
        (accepted, Instant::now())
    };
    let connected = async {
        let stream = connect.await;
        (stream, Instant::now())
    };

    let ((accepted, returned), (mut stream, connected_at)) = tokio::join!(timed, connected);

    match accepted {
        Err(Error::ConnectFailure(err)) => {
            assert_eq!(err.kind(), std::io::ErrorKind::TimedOut)
        }
        Err(other) => panic!("expected a handshake timeout, got {other:?}"),
        Ok(_) => panic!("expected a handshake timeout, the handshake completed"),
    }

    let waited = returned.saturating_duration_since(connected_at);
    assert!(
        waited < DEADLINE + MARGIN,
        "accept returned {waited:?} after the connection, past the deadline"
    );

    assert_eq!(
        tasks.num_alive_tasks(),
        before,
        "a task of the timed-out connection outlives accept"
    );

    // End of stream or a reset both mean the server closed the connection.
    let mut next = [0u8; 1];
    let read = tokio::time::timeout(CLOSE_TIMEOUT, stream.read(&mut next))
        .await
        .expect("the server keeps the connection open after the handshake deadline");
    assert!(
        !matches!(read, Ok(n) if n > 0),
        "the server wrote to a peer that never proposed versions"
    );
}

async fn tcp_listener() -> TcpListener {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap()
}

#[tokio::test]
async fn peer_server_times_out_a_silent_initiator() {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    assert_silent_peer_times_out(
        async { TcpStream::connect(addr).await.unwrap() },
        PeerServer::accept_with_timeout(&listener, MAINNET_MAGIC, DEADLINE),
    )
    .await;
}

#[tokio::test]
async fn peer_server_accepts_a_handshake_within_the_deadline() {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    let (server, client) = tokio::join!(
        PeerServer::accept_with_timeout(&listener, MAINNET_MAGIC, CLOSE_TIMEOUT),
        PeerClient::connect(addr, MAINNET_MAGIC),
    );

    let server = server.expect("the server accepts a timely handshake");
    assert!(server.accepted_version().is_some());

    client.unwrap().abort().await;
    server.abort().await;
}

#[tokio::test(start_paused = true)]
#[allow(deprecated)]
async fn peer_server_accept_has_no_handshake_deadline() {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();
    let _silent = TcpStream::connect(addr).await.unwrap();

    assert_still_waiting(PeerServer::accept(&listener, MAINNET_MAGIC)).await;
}

#[tokio::test]
async fn peer_server_deadline_starts_once_the_peer_connects() {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    let (server, client) = tokio::join!(
        PeerServer::accept_with_timeout(&listener, MAINNET_MAGIC, LATE_DEADLINE),
        async {
            tokio::time::sleep(LATE_CONNECT).await;
            tokio::time::timeout(CLOSE_TIMEOUT, PeerClient::connect(addr, MAINNET_MAGIC)).await
        },
    );

    let server = server.expect("the deadline does not run while no peer is connected");
    assert!(server.accepted_version().is_some());

    client.unwrap().unwrap().abort().await;
    server.abort().await;
}

#[cfg(unix)]
mod unix {
    use super::*;

    use pallas_network::facades::{DmqServer, NodeClient, NodeServer};
    use tokio::net::{UnixListener, UnixStream};

    /// Bind a Unix listener on a fresh socket path for `name`.
    fn listener(name: &str) -> (UnixListener, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("pallas-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        (listener, path)
    }

    #[tokio::test]
    async fn node_server_times_out_a_silent_initiator() {
        let (listener, path) = listener("hs-timeout-node");

        assert_silent_peer_times_out(
            async { UnixStream::connect(&path).await.unwrap() },
            NodeServer::accept_with_timeout(&listener, MAINNET_MAGIC, DEADLINE),
        )
        .await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn dmq_server_times_out_a_silent_initiator() {
        let (listener, path) = listener("hs-timeout-dmq");

        assert_silent_peer_times_out(
            async { UnixStream::connect(&path).await.unwrap() },
            DmqServer::accept_with_timeout(&listener, MAINNET_MAGIC, DEADLINE),
        )
        .await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test(start_paused = true)]
    #[allow(deprecated)]
    async fn node_server_accept_has_no_handshake_deadline() {
        let (listener, path) = listener("hs-no-deadline-node");
        let _silent = UnixStream::connect(&path).await.unwrap();

        assert_still_waiting(NodeServer::accept(&listener, MAINNET_MAGIC)).await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test(start_paused = true)]
    #[allow(deprecated)]
    async fn dmq_server_accept_has_no_handshake_deadline() {
        let (listener, path) = listener("hs-no-deadline-dmq");
        let _silent = UnixStream::connect(&path).await.unwrap();

        assert_still_waiting(DmqServer::accept(&listener, MAINNET_MAGIC)).await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn node_server_deadline_starts_once_the_peer_connects() {
        let (listener, path) = listener("hs-late-node");

        let (server, client) = tokio::join!(
            NodeServer::accept_with_timeout(&listener, MAINNET_MAGIC, LATE_DEADLINE),
            async {
                tokio::time::sleep(LATE_CONNECT).await;
                tokio::time::timeout(CLOSE_TIMEOUT, NodeClient::connect(&path, MAINNET_MAGIC)).await
            },
        );

        let server = server.expect("the deadline does not run while no peer is connected");
        assert!(server.accepted_version().is_some());

        client.unwrap().unwrap().abort().await;
        server.abort().await;
        let _ = std::fs::remove_file(&path);
    }
}
