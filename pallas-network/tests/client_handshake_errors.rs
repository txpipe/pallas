//! A client facade must stop its plexer on every return after the handshake:
//! a malformed reply, a refusal, or a query reply leaves no task of the
//! connection running and closes it.

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use pallas_codec::minicbor;
use pallas_network::facades::{Error, PeerClient};
use pallas_network::miniprotocols::MAINNET_MAGIC;
use pallas_network::miniprotocols::handshake::{Message, RefuseReason, n2n};
use pallas_network::multiplexer::Header;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;

/// A handshake segment whose message tag is not a handshake message.
const MALFORMED_REPLY: [u8; 3] = [0x82, 0x07, 0x00];

/// Protocol id of a handshake segment sent by the responder.
const HANDSHAKE_RESPONSE: u16 = 0x8000;

/// How long the responder waits to see the client close the connection.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the client call and the responder get to finish together.
const CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// What the responder answers to the client's proposal.
#[derive(Clone, Copy)]
enum Answer {
    Malformed,
    Refuse,
    QueryReply,
}

impl Answer {
    fn payload(self) -> Vec<u8> {
        match self {
            Answer::Malformed => MALFORMED_REPLY.to_vec(),
            Answer::Refuse => {
                let reason = RefuseReason::VersionMismatch(vec![1]);
                minicbor::to_vec(Message::<n2n::VersionData>::Refuse(reason)).unwrap()
            }
            Answer::QueryReply => {
                let table = n2n::VersionTable::v7_and_above(MAINNET_MAGIC);
                minicbor::to_vec(Message::QueryReply(table)).unwrap()
            }
        }
    }
}

/// Read the client's proposal from `stream` and answer it with `answer`.
async fn respond<S>(mut stream: S, answer: Answer) -> S
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await.unwrap();
    let header = Header::from(&header[..]);
    let mut proposal = vec![0u8; header.payload_len as usize];
    stream.read_exact(&mut proposal).await.unwrap();

    let payload = answer.payload();
    let header: [u8; 8] = Header {
        protocol: HANDSHAKE_RESPONSE,
        timestamp: 0,
        payload_len: payload.len() as u16,
    }
    .into();
    stream.write_all(&header).await.unwrap();
    stream.write_all(&payload).await.unwrap();

    stream
}

/// Run a client `call` against a responder accepting through `accept` and
/// answering with `answer`. Checks that no task of the connection outlives the
/// call and that the client closed the connection, then returns the result.
async fn call_against<S, T>(
    accept: impl Future<Output = S>,
    call: impl Future<Output = Result<T, Error>>,
    answer: Answer,
) -> Result<T, Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let tasks = tokio::runtime::Handle::current().metrics();
    let before = tasks.num_alive_tasks();

    // A stalled teardown fails the test instead of hanging it.
    let (result, mut stream) = tokio::time::timeout(CALL_TIMEOUT, async {
        tokio::join!(call, async { respond(accept.await, answer).await })
    })
    .await
    .expect("the client call does not return after the reply");

    assert_eq!(
        tasks.num_alive_tasks(),
        before,
        "a task of the connection outlives the client call"
    );

    // End of stream or a reset both mean the client closed the connection.
    let mut next = [0u8; 1];
    let read = tokio::time::timeout(CLOSE_TIMEOUT, stream.read(&mut next))
        .await
        .expect("the client keeps the connection open");
    assert!(
        !matches!(read, Ok(n) if n > 0),
        "the client wrote after the handshake"
    );

    result
}

async fn tcp_listener() -> TcpListener {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap()
}

async fn peer_client_connect(answer: Answer) -> Result<PeerClient, Error> {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    call_against(
        async { listener.accept().await.unwrap().0 },
        PeerClient::connect(addr, MAINNET_MAGIC),
        answer,
    )
    .await
}

#[tokio::test]
async fn peer_client_connect_stops_after_a_malformed_reply() {
    let result = peer_client_connect(Answer::Malformed).await;
    assert!(matches!(result, Err(Error::HandshakeProtocol(_))));
}

#[tokio::test]
async fn peer_client_connect_stops_after_a_refusal() {
    let result = peer_client_connect(Answer::Refuse).await;
    assert!(matches!(result, Err(Error::IncompatibleVersion)));
}

async fn peer_client_query(answer: Answer) -> Result<n2n::VersionTable, Error> {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    call_against(
        async { listener.accept().await.unwrap().0 },
        PeerClient::handshake_query(addr, MAINNET_MAGIC),
        answer,
    )
    .await
}

#[tokio::test]
async fn peer_client_query_stops_after_a_query_reply() {
    let table = peer_client_query(Answer::QueryReply).await.unwrap();
    assert!(!table.values.is_empty());
}

#[tokio::test]
async fn peer_client_query_stops_after_a_malformed_reply() {
    let result = peer_client_query(Answer::Malformed).await;
    assert!(matches!(result, Err(Error::HandshakeProtocol(_))));
}

#[tokio::test]
async fn peer_client_query_stops_after_a_refusal() {
    let result = peer_client_query(Answer::Refuse).await;
    assert!(matches!(result, Err(Error::IncompatibleVersion)));
}

#[cfg(unix)]
mod unix {
    use super::*;

    use pallas_network::facades::{DmqClient, NodeClient};
    use tokio::net::UnixListener;

    /// Bind a Unix listener on a fresh socket path for `name`.
    fn listener(name: &str) -> (UnixListener, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("pallas-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        (listener, path)
    }

    async fn node_client_connect(name: &str, answer: Answer) -> Result<NodeClient, Error> {
        let (listener, path) = listener(name);

        let result = call_against(
            async { listener.accept().await.unwrap().0 },
            NodeClient::connect(&path, MAINNET_MAGIC),
            answer,
        )
        .await;

        let _ = std::fs::remove_file(&path);
        result
    }

    async fn dmq_client_connect(name: &str, answer: Answer) -> Result<DmqClient, Error> {
        let (listener, path) = listener(name);

        let result = call_against(
            async { listener.accept().await.unwrap().0 },
            DmqClient::connect(&path, MAINNET_MAGIC),
            answer,
        )
        .await;

        let _ = std::fs::remove_file(&path);
        result
    }

    #[tokio::test]
    async fn node_client_connect_stops_after_a_malformed_reply() {
        let result = node_client_connect("cl-node-bad", Answer::Malformed).await;
        assert!(matches!(result, Err(Error::HandshakeProtocol(_))));
    }

    #[tokio::test]
    async fn node_client_connect_stops_after_a_refusal() {
        let result = node_client_connect("cl-node-refuse", Answer::Refuse).await;
        assert!(matches!(result, Err(Error::IncompatibleVersion)));
    }

    #[tokio::test]
    async fn dmq_client_connect_stops_after_a_malformed_reply() {
        let result = dmq_client_connect("cl-dmq-bad", Answer::Malformed).await;
        assert!(matches!(result, Err(Error::HandshakeProtocol(_))));
    }

    #[tokio::test]
    async fn dmq_client_connect_stops_after_a_refusal() {
        let result = dmq_client_connect("cl-dmq-refuse", Answer::Refuse).await;
        assert!(matches!(result, Err(Error::IncompatibleVersion)));
    }
}
