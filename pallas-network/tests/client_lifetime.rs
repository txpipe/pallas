//! A connected client facade closes its connection once it is dropped: no task
//! of the connection keeps running and the peer sees the connection end. An
//! agent moved out of the client keeps the connection open while it exists.

use std::fmt::Debug;
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use pallas_codec::minicbor;
use pallas_network::facades::{Error, PeerClient};
use pallas_network::miniprotocols::MAINNET_MAGIC;
use pallas_network::miniprotocols::handshake::{Message, n2n};
use pallas_network::multiplexer::Header;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::runtime::RuntimeMetrics;

/// Protocol id of a handshake segment sent by the responder.
const HANDSHAKE_RESPONSE: u16 = 0x8000;

/// How long the connect call and the responder get to finish together.
const CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the responder waits to see the client close the connection, and
/// the test waits for the connection's tasks to end.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a connection still in use is watched for staying open.
const OPEN_WINDOW: Duration = Duration::from_millis(300);

async fn read_segment<S>(stream: &mut S) -> std::io::Result<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await?;
    let header = Header::from(&header[..]);
    let mut payload = vec![0u8; header.payload_len as usize];
    stream.read_exact(&mut payload).await?;

    Ok(payload)
}

/// Read the client's proposal from `stream` and accept its highest version.
async fn accept_handshake<S, D>(stream: &mut S)
where
    S: AsyncRead + AsyncWrite + Unpin,
    D: Debug + Clone,
    Message<D>: for<'b> minicbor::Decode<'b, ()> + minicbor::Encode<()>,
{
    let proposal = read_segment(stream).await.unwrap();
    let Message::<D>::Propose(table) = minicbor::decode(&proposal).unwrap() else {
        panic!("the client did not propose versions");
    };
    let (version, data) = table.values.into_iter().max_by_key(|(v, _)| *v).unwrap();

    let payload = minicbor::to_vec(Message::Accept(version, data)).unwrap();
    let header: [u8; 8] = Header {
        protocol: HANDSHAKE_RESPONSE,
        timestamp: 0,
        payload_len: payload.len() as u16,
    }
    .into();
    stream.write_all(&header).await.unwrap();
    stream.write_all(&payload).await.unwrap();
}

/// Read and discard what the client sends until it closes the connection. End
/// of stream or a reset both mean it did.
async fn until_closed<S>(stream: &mut S)
where
    S: AsyncRead + Unpin,
{
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf).await {
        if n == 0 {
            break;
        }
    }
}

/// Connect a client through `connect` to a responder accepting through
/// `accept` that completes the handshake. Returns the client, the responder's
/// end of the connection and the runtime's task count before connecting.
async fn connected<S, C, D>(
    accept: impl Future<Output = S>,
    connect: impl Future<Output = Result<C, Error>>,
) -> (C, S, usize)
where
    S: AsyncRead + AsyncWrite + Unpin,
    D: Debug + Clone,
    Message<D>: for<'b> minicbor::Decode<'b, ()> + minicbor::Encode<()>,
{
    let before = tasks().num_alive_tasks();

    let (client, stream) = tokio::time::timeout(CALL_TIMEOUT, async {
        tokio::join!(connect, async {
            let mut stream = accept.await;
            accept_handshake::<_, D>(&mut stream).await;
            stream
        })
    })
    .await
    .expect("the connect call does not return after the handshake");

    let client = client.expect("the handshake is accepted");
    assert!(
        tasks().num_alive_tasks() > before,
        "the connection runs no task"
    );

    (client, stream, before)
}

fn tasks() -> RuntimeMetrics {
    tokio::runtime::Handle::current().metrics()
}

/// Check that the client closed the connection and that every task it started
/// ends, giving both [`CLOSE_TIMEOUT`].
async fn assert_closed<S>(stream: &mut S, before: usize)
where
    S: AsyncRead + Unpin,
{
    tokio::time::timeout(CLOSE_TIMEOUT, until_closed(stream))
        .await
        .expect("the client keeps the connection open");

    tokio::time::timeout(CLOSE_TIMEOUT, async {
        while tasks().num_alive_tasks() > before {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("a task of the connection outlives the client");
}

/// Check that the connection stays open for [`OPEN_WINDOW`].
async fn assert_open<S>(stream: &mut S)
where
    S: AsyncRead + Unpin,
{
    let closed = tokio::time::timeout(OPEN_WINDOW, until_closed(stream)).await;
    assert!(closed.is_err(), "the connection closed while in use");
}

/// Check that a `PeerClient` connection stays open for [`OPEN_WINDOW`] with
/// its muxer, demuxer and keep-alive loop all still running.
async fn assert_in_use<S>(stream: &mut S, before: usize)
where
    S: AsyncRead + Unpin,
{
    assert_open(stream).await;
    assert_eq!(
        tasks().num_alive_tasks(),
        before + 3,
        "the keep-alive loop stopped while an agent is in use"
    );
}

async fn tcp_listener() -> TcpListener {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap()
}

async fn peer_client() -> (PeerClient, tokio::net::TcpStream, usize) {
    let listener = tcp_listener().await;
    let addr = listener.local_addr().unwrap();

    connected::<_, _, n2n::VersionData>(
        async { listener.accept().await.unwrap().0 },
        PeerClient::connect(addr, MAINNET_MAGIC),
    )
    .await
}

#[tokio::test]
async fn dropped_peer_client_closes_its_connection() {
    let (client, mut stream, before) = peer_client().await;

    drop(client);

    assert_closed(&mut stream, before).await;
}

#[tokio::test]
async fn peer_client_agent_moved_out_keeps_the_connection_open() {
    let (client, mut stream, before) = peer_client().await;

    let PeerClient {
        plexer,
        keepalive,
        chainsync,
        blockfetch,
        txsubmission,
        peersharing,
    } = client;
    drop((plexer, keepalive, blockfetch, txsubmission, peersharing));
    assert_open(&mut stream).await;

    drop(chainsync);
    assert_closed(&mut stream, before).await;
}

/// Connect a `PeerClient`, let `keep` take what it holds on to and drop the
/// rest, and check that this keeps the keep-alive loop running until it is
/// dropped too.
async fn assert_keeps_the_keepalive<T>(keep: impl FnOnce(PeerClient) -> T) {
    let (client, mut stream, before) = peer_client().await;

    let kept = keep(client);
    assert_in_use(&mut stream, before).await;

    drop(kept);
    assert_closed(&mut stream, before).await;
}

#[tokio::test]
async fn peer_client_in_use_keeps_its_keepalive_running() {
    assert_keeps_the_keepalive(|client| client).await;
}

#[tokio::test]
async fn any_peer_client_agent_keeps_the_keepalive_running() {
    assert_keeps_the_keepalive(|client| client.chainsync).await;
    assert_keeps_the_keepalive(|client| client.blockfetch).await;
    assert_keeps_the_keepalive(|client| client.txsubmission).await;
    assert_keeps_the_keepalive(|client| client.peersharing).await;
}

#[tokio::test]
async fn peer_client_abort_leaves_no_task() {
    let (client, mut stream, before) = peer_client().await;

    tokio::time::timeout(CLOSE_TIMEOUT, client.abort())
        .await
        .expect("abort does not return");
    assert_eq!(
        tasks().num_alive_tasks(),
        before,
        "a task of the connection outlives abort"
    );

    assert_closed(&mut stream, before).await;
}

#[cfg(unix)]
mod unix {
    use super::*;

    use pallas_network::facades::{DmqClient, NodeClient};
    use pallas_network::miniprotocols::handshake::n2c;
    use tokio::net::UnixListener;

    /// Bind a Unix listener on a fresh socket path for `name`.
    fn listener(name: &str) -> (UnixListener, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("pallas-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        (listener, path)
    }

    #[tokio::test]
    async fn dropped_node_client_closes_its_connection() {
        let (listener, path) = listener("life-node");

        let (client, mut stream, before) = connected::<_, _, n2c::VersionData>(
            async { listener.accept().await.unwrap().0 },
            NodeClient::connect(&path, MAINNET_MAGIC),
        )
        .await;
        let _ = std::fs::remove_file(&path);

        drop(client);

        assert_closed(&mut stream, before).await;
    }

    #[tokio::test]
    async fn dropped_dmq_client_closes_its_connection() {
        let (listener, path) = listener("life-dmq");

        let (client, mut stream, before) = connected::<_, _, n2c::VersionData>(
            async { listener.accept().await.unwrap().0 },
            DmqClient::connect(&path, MAINNET_MAGIC),
        )
        .await;
        let _ = std::fs::remove_file(&path);

        drop(client);

        assert_closed(&mut stream, before).await;
    }
}
