//! The receive side bounds how large an inbound message may grow and finds a
//! message's end in time linear in its bytes.
//!
//! The agents that face other nodes cap each inbound message at the size of
//! its protocol state; the node-to-client agents do not. Every limited agent
//! can turn its cap off. Reassembly scans each inbound byte once, so a message
//! sent as many tiny segments is absorbed in time proportional to its size.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};

use pallas_codec::minicbor;
use pallas_codec::utils::AnyCbor;
use pallas_network::facades::{Error, PeerServer};
use pallas_network::miniprotocols::blockfetch;
use pallas_network::miniprotocols::chainsync::{self, BlockContent, HeaderContent, Tip};
use pallas_network::miniprotocols::handshake::{self, n2n};
use pallas_network::miniprotocols::localstate;
use pallas_network::miniprotocols::peersharing::{self, PeerAddress};
use pallas_network::miniprotocols::txsubmission::{self, EraTxBody, EraTxId, TxIdAndSize};
use pallas_network::miniprotocols::{
    MAINNET_MAGIC, PROTOCOL_N2C_CHAIN_SYNC, PROTOCOL_N2C_STATE_QUERY, PROTOCOL_N2N_BLOCK_FETCH,
    PROTOCOL_N2N_CHAIN_SYNC, PROTOCOL_N2N_HANDSHAKE, PROTOCOL_N2N_KEEP_ALIVE,
    PROTOCOL_N2N_PEER_SHARING, PROTOCOL_N2N_TX_SUBMISSION, Point, keepalive,
};
use pallas_network::multiplexer::{
    AgentChannel, Bearer, ChannelBuffer, Header, MAX_SEGMENT_PAYLOAD_LENGTH, Plexer, RunningPlexer,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const HEADER_LEN: usize = 8;

/// Keeps the plexer tasks of a loopback pair alive for the test's duration.
struct Link {
    _client: RunningPlexer,
    _server: RunningPlexer,
}

/// Connect two plexers over loopback TCP and subscribe `protocol` on each,
/// returning the initiator channel, the responder channel and a guard.
async fn loopback(protocol: u16) -> (AgentChannel, AgentChannel, Link) {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let (initiator, responder) =
        tokio::join!(Bearer::connect_tcp(addr), Bearer::accept_tcp(&listener));
    let mut client_plexer = Plexer::new(initiator.unwrap());
    let mut server_plexer = Plexer::new(responder.unwrap().0);

    let client_channel = client_plexer.subscribe_client(protocol);
    let server_channel = server_plexer.subscribe_server(protocol);

    let link = Link {
        _client: client_plexer.spawn(),
        _server: server_plexer.spawn(),
    };

    (client_channel, server_channel, link)
}

/// Encoded byte length of a handshake message.
fn encoded_len<M: minicbor::Encode<()>>(msg: &M) -> usize {
    let mut bytes = vec![];
    minicbor::encode(msg, &mut bytes).unwrap();
    bytes.len()
}

/// A handshake proposal whose encoded size brackets `target` bytes: `entries`
/// identical version entries under distinct version numbers.
fn proposal_of(entries: u64) -> handshake::Message<n2n::VersionData> {
    let values = (0..entries)
        .map(|i| {
            (
                1000 + i,
                n2n::VersionData::new(MAINNET_MAGIC, true, None, None),
            )
        })
        .collect();
    handshake::Message::Propose(handshake::VersionTable { values })
}

#[tokio::test]
async fn handshake_receive_bounds_the_proposal_size() {
    let limit = handshake::MESSAGE_SIZE_LIMIT;

    // A proposal comfortably under the limit, and one just over it.
    let under = proposal_of(200);
    assert!(encoded_len(&under) < limit, "fixture should be under limit");
    let mut over_entries = 200;
    while encoded_len(&proposal_of(over_entries)) <= limit {
        over_entries += 20;
    }
    let over = proposal_of(over_entries);

    // Under the limit: the responder receives the proposal.
    {
        let (client, server, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;
        let mut client = handshake::N2NClient::new(client);
        let mut server = handshake::N2NServer::new(server);

        let (_, received) = tokio::join!(
            async {
                client.send_message(&under).await.unwrap();
            },
            server.receive_proposed_versions(),
        );
        assert!(received.is_ok(), "a proposal under the limit is received");
    }

    // Over the limit: the responder refuses it with the documented error.
    {
        let (client, server, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;
        let mut client = handshake::N2NClient::new(client);
        let mut server = handshake::N2NServer::new(server);

        let (_, received) = tokio::join!(
            async {
                let _ = client.send_message(&over).await;
            },
            server.receive_proposed_versions(),
        );
        assert_size_limit_error(received.unwrap_err(), limit);
    }

    // Over the limit but with limits disabled: the responder receives it.
    {
        let (client, server, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;
        let mut client = handshake::N2NClient::new(client);
        let mut server = handshake::N2NServer::new(server);
        server.disable_size_limits();

        let (_, received) = tokio::join!(
            async {
                client.send_message(&over).await.unwrap();
            },
            server.receive_proposed_versions(),
        );
        assert!(
            received.is_ok(),
            "the opt-out accepts an oversized proposal"
        );
    }
}

/// A single intersection point carrying a `payload`-byte hash.
fn point_of(payload: usize) -> Point {
    Point::Specific(42, vec![0xab; payload])
}

#[tokio::test]
async fn chainsync_receive_bounds_the_message_size() {
    let limit = chainsync::MESSAGE_SIZE_LIMIT;

    // One big point stays under the limit; two go over it.
    let under = chainsync::Message::<HeaderContent>::FindIntersect(vec![point_of(limit - 2_000)]);
    assert!(encoded_len(&under) < limit);
    let over = chainsync::Message::<HeaderContent>::FindIntersect(vec![point_of(limit - 2_000); 2]);
    assert!(encoded_len(&over) > limit);

    for (message, disable, accepted) in [
        (&under, false, true),
        (&over, false, false),
        (&over, true, true),
    ] {
        let (client, server, _link) = loopback(PROTOCOL_N2N_CHAIN_SYNC).await;
        let mut client = chainsync::N2NClient::new(client);
        let mut server = chainsync::N2NServer::new(server);
        if disable {
            server.disable_size_limits();
        }

        let (_, received) = tokio::join!(
            async {
                let _ = client.send_message(message).await;
            },
            server.recv_while_idle(),
        );

        if accepted {
            assert!(received.is_ok(), "message within budget is received");
        } else {
            assert_size_limit_error(received.unwrap_err(), limit);
        }
    }
}

#[tokio::test]
async fn blockfetch_receive_bounds_a_streamed_block() {
    let limit = blockfetch::STREAMING_SIZE_LIMIT;

    for (body_len, disable, accepted) in [
        (limit - 50_000, false, true),
        (limit + 50_000, false, false),
        (limit + 50_000, true, true),
    ] {
        let (client, server, _link) = loopback(PROTOCOL_N2N_BLOCK_FETCH).await;
        let mut client = blockfetch::Client::new(client);
        let mut server = blockfetch::Server::new(server);
        if disable {
            client.disable_size_limits();
        }

        let point = Point::Specific(1, vec![0; 4]);

        // Drive both agents into the streaming state.
        let (sent, request) = tokio::join!(
            client.request_range((point.clone(), point.clone())),
            drive_server_to_streaming(&mut server),
        );
        sent.unwrap().unwrap();
        request.unwrap();

        // Stream one block and receive it.
        let (_, received) = tokio::join!(
            async {
                let _ = server.send_block(vec![0u8; body_len]).await;
            },
            client.recv_while_streaming(),
        );

        if accepted {
            assert_eq!(received.unwrap().unwrap().len(), body_len);
        } else {
            assert_size_limit_error(received.unwrap_err(), limit);
        }
    }
}

#[tokio::test]
async fn node_to_client_receive_is_unbounded() {
    // A state-query result larger than the node-to-node streaming cap still
    // decodes, because node-to-client agents receive without limits.
    let payload = blockfetch::STREAMING_SIZE_LIMIT + 1_000_000;
    let mut big = vec![0x5a]; // CBOR byte string, 4-byte length follows.
    big.extend((payload as u32).to_be_bytes());
    big.extend(std::iter::repeat_n(0u8, payload));
    let result = AnyCbor::from_raw_bytes(big);

    let (client, server, _link) = loopback(PROTOCOL_N2C_STATE_QUERY).await;
    let mut client = localstate::Client::new(client);
    let mut server = localstate::Server::new(server);

    let query = AnyCbor::from_raw_bytes(vec![0x00]);
    let (client_result, _) = tokio::join!(
        async {
            client.acquire(None).await.unwrap();
            client.query_any(query).await
        },
        async {
            server.recv_while_idle().await.unwrap();
            server.send_acquired().await.unwrap();
            server.recv_while_acquired().await.unwrap();
            server.send_result(result).await.unwrap();
        },
    );

    assert_eq!(client_result.unwrap().raw_bytes().len(), payload + 5);
}

#[tokio::test]
async fn unlimited_receive_is_linear_in_message_size() {
    // A proposal whose version table never ends, sent as small segments of
    // whole entries.
    const TARGET: usize = 2 * 1024 * 1024;
    const SEGMENT: usize = 16;
    let prefix = never_ending_proposal_prefix();

    let (mut client, server, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;

    // The receiver has no limit: it never returns for this input, but it must
    // keep up with the bytes as they stream in.
    tokio::spawn(async move {
        let mut buffer = ChannelBuffer::new(server);
        let _ = buffer
            .recv_full_msg::<handshake::Message<n2n::VersionData>>()
            .await;
    });

    let start = Instant::now();
    let send = async {
        client.enqueue_chunk(prefix).await.unwrap();
        let filler = version_entries(SEGMENT);
        let mut sent = 0;
        while sent < TARGET {
            client.enqueue_chunk(filler.clone()).await.unwrap();
            sent += SEGMENT;
        }
    };

    // Backpressure makes this track the receiver's work. Linear reassembly
    // absorbs these bytes well within this bound; the previous per-segment
    // re-scan exceeded it by far.
    let bound = Duration::from_secs(20);
    assert!(
        tokio::time::timeout(bound, send).await.is_ok(),
        "absorbing {TARGET} bytes took over {bound:?}",
    );
    eprintln!("absorbed {TARGET} bytes in {:?}", start.elapsed());
}

// `accept` has no handshake deadline, so only the size limit can end this.
#[allow(deprecated)]
#[tokio::test]
async fn peer_server_refuses_an_oversized_proposal() {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let tasks = tokio::runtime::Handle::current().metrics();
    let before = tasks.num_alive_tasks();

    let (accepted, stream) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(PeerServer::accept(&listener, MAINNET_MAGIC), async {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            stream_never_ending_proposal(&mut stream).await;
            stream
        })
    })
    .await
    .expect("accept returns after refusing the proposal");

    let Err(Error::HandshakeProtocol(err)) = accepted else {
        panic!("the handshake fails with a protocol error");
    };
    assert_refused_within_one_segment(err, handshake::MESSAGE_SIZE_LIMIT, PROPOSAL_SEGMENT);
    assert_eq!(
        tasks.num_alive_tasks(),
        before,
        "no task of the refused connection outlives accept",
    );

    // The server closed the connection rather than keep buffering.
    let mut stream = stream;
    let mut byte = [0u8; 1];
    let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut byte))
        .await
        .expect("the server closes the connection");
    assert!(!matches!(read, Ok(n) if n > 0), "the server wrote back");
}

/// Segment size used to stream a never-ending proposal.
const PROPOSAL_SEGMENT: usize = 1024;

/// `[0, {` with a declared entry count of `u64::MAX`: a proposal whose version
/// table never ends.
fn never_ending_proposal_prefix() -> Vec<u8> {
    let mut prefix = vec![0x82, 0x00, 0xbb];
    prefix.extend([0xff; 8]);
    prefix
}

/// `len` bytes of whole version-table entries, each `0 => [0, false]`.
fn version_entries(len: usize) -> Vec<u8> {
    assert_eq!(len % 4, 0);
    [0x00, 0x82, 0x00, 0xf4].repeat(len / 4)
}

/// Write a proposal that never ends as many small segments, stopping once the
/// server closes the connection or well past the handshake limit.
async fn stream_never_ending_proposal(stream: &mut TcpStream) {
    if write_segment(stream, &never_ending_proposal_prefix())
        .await
        .is_err()
    {
        return;
    }

    let entries = version_entries(PROPOSAL_SEGMENT);
    for _ in 0..(64 * handshake::MESSAGE_SIZE_LIMIT / PROPOSAL_SEGMENT) {
        if write_segment(stream, &entries).await.is_err() {
            return;
        }
    }
}

/// Frame `payload` as one handshake segment and write it.
async fn write_segment(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let header: [u8; HEADER_LEN] = Header {
        protocol: PROTOCOL_N2N_HANDSHAKE,
        timestamp: 0,
        payload_len: payload.len() as u16,
    }
    .into();
    stream.write_all(&header).await?;
    stream.write_all(payload).await?;
    stream.flush().await
}

#[tokio::test]
async fn handshake_server_stops_buffering_a_never_ending_proposal() {
    let limit = handshake::MESSAGE_SIZE_LIMIT;
    let (mut client, server, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;
    let mut server = handshake::N2NServer::new(server);

    let send = async {
        client
            .enqueue_chunk(never_ending_proposal_prefix())
            .await
            .unwrap();
        let entries = version_entries(PROPOSAL_SEGMENT);
        for _ in 0..(64 * limit / PROPOSAL_SEGMENT) {
            client.enqueue_chunk(entries.clone()).await.unwrap();
        }
        std::future::pending::<()>().await;
    };

    let received = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::select! {
            received = server.receive_proposed_versions() => received,
            _ = send => unreachable!(),
        }
    })
    .await
    .expect("the receive stops on its own");

    assert_refused_within_one_segment(received.unwrap_err(), limit, PROPOSAL_SEGMENT);
}

/// Assert `err` is the size-limit error, raised once more than `limit` bytes
/// and at most one further `segment` were buffered.
fn assert_refused_within_one_segment(err: impl std::fmt::Debug, limit: usize, segment: usize) {
    assert_size_limit_error(&err, limit);
    let text = format!("{err:?}");
    let seen: usize = text
        .split_once("message of at least ")
        .and_then(|(_, rest)| rest.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no size seen in: {text}"));
    assert!(
        seen > limit && seen <= limit + segment,
        "refused at {seen} bytes, limit {limit}, segment {segment}",
    );
    assert!(segment <= MAX_SEGMENT_PAYLOAD_LENGTH);
}

/// Assert `err`'s string names the size limit and the bytes seen.
fn assert_size_limit_error(err: impl std::fmt::Debug, limit: usize) {
    let text = format!("{err:?}");
    assert!(
        text.contains(&format!("exceeds the size limit of {limit} bytes")),
        "unexpected error: {text}",
    );
}

/// Receive the client's range request and open the batch, leaving the server
/// in the streaming state.
async fn drive_server_to_streaming(
    server: &mut blockfetch::Server,
) -> Result<Option<blockfetch::BlockRequest>, blockfetch::ServerError> {
    let request = server.recv_while_idle().await?;
    server.send_start_batch().await?;
    Ok(request)
}

/// What a limited receive does with the message the peer sent.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Expect {
    /// Received.
    Accepted,
    /// Refused with the size-limit error.
    Refused,
    /// Past the size check, whatever the agent then makes of it: for states
    /// whose valid messages are too small to reach the limit.
    NotRefused,
}

/// The receives a limit test runs: under the limit, over it, and over it
/// with the opt-out.
fn cases<M>(under: M, over: M, over_opted_out: Expect) -> [(M, bool, Expect); 3]
where
    M: Clone,
{
    [
        (under, false, Expect::Accepted),
        (over.clone(), false, Expect::Refused),
        (over, true, over_opted_out),
    ]
}

/// Check a receive's result against what was expected of it.
fn check<T, E: std::fmt::Debug>(result: Result<T, E>, expect: Expect, limit: usize) {
    match expect {
        Expect::Accepted => {
            if let Err(err) = result {
                panic!("the message is not received: {err:?}");
            }
        }
        Expect::Refused => match result {
            Ok(_) => panic!("an oversized message is received"),
            Err(err) => assert_size_limit_error(err, limit),
        },
        Expect::NotRefused => {
            if let Err(err) = result {
                let text = format!("{err:?}");
                assert!(!text.contains("exceeds the size limit"), "refused: {text}");
            }
        }
    }
}

/// Bytes the peer sends: an encoded message, or raw CBOR.
#[derive(Clone)]
struct Wire(Vec<u8>);

fn wire<M: minicbor::Encode<()>>(msg: &M) -> Wire {
    let mut bytes = vec![];
    minicbor::encode(msg, &mut bytes).unwrap();
    Wire(bytes)
}

/// `[label, <byte string>]` of at least `len` bytes: a message larger than
/// any valid message of a protocol whose messages are all small.
fn oversized(label: u8, len: usize) -> Wire {
    let mut bytes = vec![0x82, label, 0x5a];
    bytes.extend((len as u32).to_be_bytes());
    bytes.extend(std::iter::repeat_n(0u8, len));
    Wire(bytes)
}

/// Send `wire` from the peer's channel, from a task of its own so a receive
/// that stops early does not hold it up.
fn send_from_peer(mut peer: AgentChannel, wire: Wire) {
    tokio::spawn(async move {
        for chunk in wire.0.chunks(MAX_SEGMENT_PAYLOAD_LENGTH) {
            if peer.enqueue_chunk(chunk.to_vec()).await.is_err() {
                break;
            }
        }
        peer
    });
}

/// A transaction id carrying `len` bytes.
fn tx_id(len: usize) -> EraTxId {
    EraTxId(6, vec![0xcd; len])
}

#[test]
fn limits_match_the_network_specification() {
    assert_eq!(handshake::MESSAGE_SIZE_LIMIT, 5_760);
    assert_eq!(chainsync::MESSAGE_SIZE_LIMIT, 65_535);
    assert_eq!(blockfetch::IDLE_SIZE_LIMIT, 65_535);
    assert_eq!(blockfetch::BUSY_SIZE_LIMIT, 65_535);
    assert_eq!(blockfetch::STREAMING_SIZE_LIMIT, 2_500_000);
    assert_eq!(txsubmission::INIT_SIZE_LIMIT, 65_535);
    assert_eq!(txsubmission::IDLE_SIZE_LIMIT, 65_535);
    assert_eq!(txsubmission::TX_IDS_SIZE_LIMIT, 2_500_000);
    assert_eq!(txsubmission::TXS_SIZE_LIMIT, 2_500_000);
    assert_eq!(keepalive::MESSAGE_SIZE_LIMIT, 65_535);
    assert_eq!(peersharing::MESSAGE_SIZE_LIMIT, 5_760);
}

#[tokio::test]
async fn handshake_client_bounds_the_reply_size() {
    let limit = handshake::MESSAGE_SIZE_LIMIT;
    let refusal = |n: u64| {
        handshake::Message::<n2n::VersionData>::Refuse(handshake::RefuseReason::VersionMismatch(
            (0..n).map(|i| 1_000_000_000_000 + i).collect(),
        ))
    };
    let (under, over) = (wire(&refusal(500)), wire(&refusal(700)));
    assert!(under.0.len() < limit && over.0.len() > limit);

    for (msg, disable, expect) in cases(under, over, Expect::Accepted) {
        let (client, peer, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;
        let mut client = handshake::N2NClient::new(client);
        if disable {
            client.disable_size_limits();
        }
        client
            .send_propose(handshake::VersionTable {
                values: [(13, n2n::VersionData::new(MAINNET_MAGIC, true, None, None))].into(),
            })
            .await
            .unwrap();

        send_from_peer(peer, msg);
        check(client.recv_message().await, expect, limit);
    }
}

#[tokio::test]
async fn chainsync_client_bounds_the_message_size() {
    let limit = chainsync::MESSAGE_SIZE_LIMIT;
    let found = |n: usize| {
        chainsync::Message::<HeaderContent>::IntersectFound(point_of(n), Tip(point_of(32), 1))
    };
    let (under, over) = (wire(&found(limit - 2_000)), wire(&found(limit + 2_000)));

    for (msg, disable, expect) in cases(under, over, Expect::Accepted) {
        let (client, peer, _link) = loopback(PROTOCOL_N2N_CHAIN_SYNC).await;
        let mut client = chainsync::N2NClient::new(client);
        if disable {
            client.disable_size_limits();
        }
        client
            .send_find_intersect(vec![point_of(32)])
            .await
            .unwrap();

        send_from_peer(peer, msg);
        check(client.recv_message().await, expect, limit);
    }
}

#[tokio::test]
async fn blockfetch_client_bounds_the_busy_reply() {
    let limit = blockfetch::BUSY_SIZE_LIMIT;
    let under = wire(&blockfetch::Message::StartBatch);
    let over = wire(&blockfetch::Message::Block {
        body: vec![0; limit + 1_000],
    });

    for (msg, disable, expect) in cases(under, over, Expect::NotRefused) {
        let (client, peer, _link) = loopback(PROTOCOL_N2N_BLOCK_FETCH).await;
        let mut client = blockfetch::Client::new(client);
        if disable {
            client.disable_size_limits();
        }
        let point = point_of(32);
        client
            .send_request_range((point.clone(), point))
            .await
            .unwrap();

        send_from_peer(peer, msg);
        check(client.recv_message().await, expect, limit);
    }
}

#[tokio::test]
async fn blockfetch_server_bounds_the_range_request() {
    let limit = blockfetch::IDLE_SIZE_LIMIT;
    let request = |n: usize| blockfetch::Message::RequestRange {
        range: (point_of(n), point_of(n)),
    };
    let (under, over) = (wire(&request(30_000)), wire(&request(34_000)));
    assert!(under.0.len() < limit && over.0.len() > limit);

    for (msg, disable, expect) in cases(under, over, Expect::Accepted) {
        let (peer, server, _link) = loopback(PROTOCOL_N2N_BLOCK_FETCH).await;
        let mut server = blockfetch::Server::new(server);
        if disable {
            server.disable_size_limits();
        }

        send_from_peer(peer, msg);
        check(server.recv_while_idle().await, expect, limit);
    }
}

type TxMessage = txsubmission::Message<EraTxId, EraTxBody>;

#[tokio::test]
async fn txsubmission_server_bounds_the_init_message() {
    let limit = txsubmission::INIT_SIZE_LIMIT;
    let under = wire(&TxMessage::Init);
    let over = wire(&TxMessage::ReplyTxs(vec![EraTxBody(6, vec![0; limit])]));

    for (msg, disable, expect) in cases(under, over, Expect::NotRefused) {
        let (peer, server, _link) = loopback(PROTOCOL_N2N_TX_SUBMISSION).await;
        let mut server = txsubmission::Server::new(server);
        if disable {
            server.disable_size_limits();
        }

        send_from_peer(peer, msg);
        check(server.wait_for_init().await, expect, limit);
    }
}

/// Wait for a transaction-submission server to receive `Init`, sent by
/// `peer`, and return the peer's channel.
async fn init_txsubmission(
    server: &mut txsubmission::Server,
    mut peer: AgentChannel,
) -> AgentChannel {
    peer.enqueue_chunk(wire(&TxMessage::Init).0).await.unwrap();
    server.wait_for_init().await.unwrap();
    peer
}

#[tokio::test]
async fn txsubmission_server_bounds_the_tx_ids_reply() {
    let limit = txsubmission::TX_IDS_SIZE_LIMIT;
    // Under this limit but over the idle one, so the state's own limit applies.
    let reply = |n: usize| TxMessage::ReplyTxIds(vec![TxIdAndSize(tx_id(n), 1)]);
    let (under, over) = (wire(&reply(100_000)), wire(&reply(limit)));

    for (msg, disable, expect) in cases(under, over, Expect::Accepted) {
        let (peer, server, _link) = loopback(PROTOCOL_N2N_TX_SUBMISSION).await;
        let mut server = txsubmission::Server::new(server);
        if disable {
            server.disable_size_limits();
        }
        let peer = init_txsubmission(&mut server, peer).await;
        server
            .acknowledge_and_request_tx_ids(true, 0, 1)
            .await
            .unwrap();

        send_from_peer(peer, msg);
        check(server.receive_next_reply().await, expect, limit);
    }
}

#[tokio::test]
async fn txsubmission_server_bounds_the_txs_reply() {
    let limit = txsubmission::TXS_SIZE_LIMIT;
    // Under this limit but over the idle one, so the state's own limit applies.
    let reply = |n: usize| TxMessage::ReplyTxs(vec![EraTxBody(6, vec![0; n])]);
    let (under, over) = (wire(&reply(100_000)), wire(&reply(limit)));

    for (msg, disable, expect) in cases(under, over, Expect::Accepted) {
        let (peer, server, _link) = loopback(PROTOCOL_N2N_TX_SUBMISSION).await;
        let mut server = txsubmission::Server::new(server);
        if disable {
            server.disable_size_limits();
        }
        let peer = init_txsubmission(&mut server, peer).await;
        server.request_txs(vec![tx_id(32)]).await.unwrap();

        send_from_peer(peer, msg);
        check(server.receive_next_reply().await, expect, limit);
    }
}

#[tokio::test]
async fn txsubmission_client_bounds_the_request() {
    let limit = txsubmission::IDLE_SIZE_LIMIT;
    let request = |n: usize| TxMessage::RequestTxs(vec![tx_id(n)]);
    let (under, over) = (wire(&request(limit - 1_000)), wire(&request(limit + 1_000)));

    for (msg, disable, expect) in cases(under, over, Expect::Accepted) {
        let (client, peer, _link) = loopback(PROTOCOL_N2N_TX_SUBMISSION).await;
        let mut client = txsubmission::Client::new(client);
        if disable {
            client.disable_size_limits();
        }
        client.send_init().await.unwrap();

        send_from_peer(peer, msg);
        check(client.next_request().await, expect, limit);
    }
}

#[tokio::test]
async fn keepalive_client_bounds_the_response() {
    let limit = keepalive::MESSAGE_SIZE_LIMIT;
    let under = wire(&keepalive::Message::ResponseKeepAlive(7));
    let over = oversized(1, limit);

    for (msg, disable, expect) in cases(under, over, Expect::NotRefused) {
        let (client, peer, _link) = loopback(PROTOCOL_N2N_KEEP_ALIVE).await;
        let mut client = keepalive::Client::new(client);
        if disable {
            client.disable_size_limits();
        }
        client.send_keepalive_request().await.unwrap();

        send_from_peer(peer, msg);
        check(client.recv_message().await, expect, limit);
    }
}

#[tokio::test]
async fn keepalive_server_bounds_the_request() {
    let limit = keepalive::MESSAGE_SIZE_LIMIT;
    let under = wire(&keepalive::Message::KeepAlive(7));
    let over = oversized(0, limit);

    for (msg, disable, expect) in cases(under, over, Expect::NotRefused) {
        let (peer, server, _link) = loopback(PROTOCOL_N2N_KEEP_ALIVE).await;
        let mut server = keepalive::Server::new(server);
        if disable {
            server.disable_size_limits();
        }

        send_from_peer(peer, msg);
        check(server.recv_message().await, expect, limit);
    }
}

fn share_peers(n: u8) -> peersharing::Message {
    peersharing::Message::SharePeers(
        (0..n)
            .map(|i| PeerAddress::V6(std::net::Ipv6Addr::from_bits(u128::MAX - i as u128), 3001))
            .collect(),
    )
}

#[tokio::test]
async fn peersharing_client_bounds_the_reply() {
    let limit = peersharing::MESSAGE_SIZE_LIMIT;
    let (under, over) = (wire(&share_peers(200)), wire(&share_peers(255)));
    assert!(under.0.len() < limit && over.0.len() > limit);

    for (msg, disable, expect) in cases(under, over, Expect::Accepted) {
        let (client, peer, _link) = loopback(PROTOCOL_N2N_PEER_SHARING).await;
        let mut client = peersharing::Client::new(client);
        if disable {
            client.disable_size_limits();
        }
        client.send_share_request(255).await.unwrap();

        send_from_peer(peer, msg);
        check(client.recv_peer_addresses().await, expect, limit);
    }
}

#[tokio::test]
async fn peersharing_server_bounds_the_request() {
    let limit = peersharing::MESSAGE_SIZE_LIMIT;
    let under = wire(&peersharing::Message::ShareRequest(10));
    let over = wire(&share_peers(255));

    for (msg, disable, expect) in cases(under, over, Expect::NotRefused) {
        let (peer, server, _link) = loopback(PROTOCOL_N2N_PEER_SHARING).await;
        let mut server = peersharing::Server::new(server);
        if disable {
            server.disable_size_limits();
        }

        send_from_peer(peer, msg);
        check(server.recv_share_request().await, expect, limit);
    }
}

#[tokio::test]
async fn node_to_client_chainsync_is_unbounded() {
    let big = 4 * chainsync::MESSAGE_SIZE_LIMIT;

    // The client receives a block larger than the node-to-node limit.
    let (client, peer, _link) = loopback(PROTOCOL_N2C_CHAIN_SYNC).await;
    let mut client = chainsync::N2CClient::new(client);
    client.send_request_next().await.unwrap();
    send_from_peer(
        peer,
        wire(&chainsync::Message::RollForward(
            BlockContent(vec![0; big]),
            Tip(point_of(32), 1),
        )),
    );
    check(client.recv_message().await, Expect::Accepted, big);

    // The server receives an intersection request larger than that limit.
    let (peer, server, _link) = loopback(PROTOCOL_N2C_CHAIN_SYNC).await;
    let mut server = chainsync::N2CServer::new(server);
    send_from_peer(
        peer,
        wire(&chainsync::Message::<BlockContent>::FindIntersect(vec![
            point_of(big),
        ])),
    );
    check(server.recv_while_idle().await, Expect::Accepted, big);
}

#[tokio::test]
async fn limited_receive_is_linear_when_a_message_spans_several_items() {
    // A tx-ids reply whose decoder reads past its own CBOR item into many
    // one-byte items after it, with its last byte never sent.
    const IDS: u32 = 400_000;
    const SEGMENT: usize = 16;
    let mut bytes = vec![0x82, 0x01, 0x9a];
    bytes.extend(IDS.to_be_bytes());
    bytes.extend([0x80, 0x80, 0x00, 0x40, 0x00].repeat(IDS as usize));
    bytes.pop();
    assert!(bytes.len() < txsubmission::TX_IDS_SIZE_LIMIT);

    let (mut client, server, _link) = loopback(PROTOCOL_N2N_TX_SUBMISSION).await;

    // The receive never returns for this input, but must keep up with it.
    tokio::spawn(async move {
        let mut buffer = ChannelBuffer::new(server);
        let _ = buffer
            .recv_full_msg_with_limit::<txsubmission::Message<EraTxId, EraTxBody>>(
                txsubmission::TX_IDS_SIZE_LIMIT,
            )
            .await;
    });

    let start = Instant::now();
    let send = async {
        for chunk in bytes.chunks(SEGMENT) {
            client.enqueue_chunk(chunk.to_vec()).await.unwrap();
        }
    };

    // Backpressure makes this track the receiver's work, which a decode of
    // the whole buffer per item would take far longer than this to finish.
    let bound = Duration::from_secs(20);
    assert!(
        tokio::time::timeout(bound, send).await.is_ok(),
        "absorbing {} bytes took over {bound:?}",
        bytes.len(),
    );
    eprintln!("absorbed {} bytes in {:?}", bytes.len(), start.elapsed());
}

#[tokio::test]
async fn limited_receive_is_linear_when_a_message_decodes_before_its_item_ends() {
    // A tx-ids reply under an array head declaring a third item, which opens
    // and never closes, so each short segment allows an early decode that
    // succeeds inside an item the scanner has not seen end.
    const IDS: u32 = 200_000;
    const SEGMENT: usize = 16;
    let mut head = vec![0x83, 0x01, 0x9a];
    head.extend(IDS.to_be_bytes());
    head.extend([0x82, 0x82, 0x00, 0x40, 0x00].repeat(IDS as usize));
    head.push(0x9f);
    assert_ne!(
        head.len() % MAX_SEGMENT_PAYLOAD_LENGTH,
        0,
        "ends on a short segment"
    );
    let tail = vec![0x00; txsubmission::TX_IDS_SIZE_LIMIT - head.len() - SEGMENT];

    let (mut client, server, _link) = loopback(PROTOCOL_N2N_TX_SUBMISSION).await;

    // The receive never returns for this input, but must keep up with it.
    tokio::spawn(async move {
        let mut buffer = ChannelBuffer::new(server);
        let _ = buffer
            .recv_full_msg_with_limit::<txsubmission::Message<EraTxId, EraTxBody>>(
                txsubmission::TX_IDS_SIZE_LIMIT,
            )
            .await;
    });

    let start = Instant::now();
    let send = async {
        for chunk in head.chunks(MAX_SEGMENT_PAYLOAD_LENGTH) {
            client.enqueue_chunk(chunk.to_vec()).await.unwrap();
        }
        for chunk in tail.chunks(SEGMENT) {
            client.enqueue_chunk(chunk.to_vec()).await.unwrap();
        }
    };

    // Backpressure makes this track the receiver's work, which a decode of
    // the whole buffer per segment would take far longer than this to finish.
    let bound = Duration::from_secs(20);
    assert!(
        tokio::time::timeout(bound, send).await.is_ok(),
        "absorbing {} bytes took over {bound:?}",
        head.len() + tail.len(),
    );
    eprintln!(
        "absorbed {} bytes in {:?}",
        head.len() + tail.len(),
        start.elapsed()
    );
}

/// A version table of NodeToNodeV_14 to V_17 on mainnet, with the extra
/// trailing field that V_17's version data carries.
fn version_table_with_extra_field() -> Vec<u8> {
    let magic = [0x1a, 0x2d, 0x96, 0x4a, 0x09];
    let mut table = vec![0xa4];
    for version in [0x0e, 0x0f, 0x10] {
        table.extend([version, 0x84]);
        table.extend(magic);
        table.extend([0xf4, 0x01, 0xf4]);
    }
    table.extend([0x11, 0x85]);
    table.extend(magic);
    table.extend([0xf4, 0x01, 0xf4, 0xf4]);
    table
}

#[tokio::test]
async fn handshake_accepts_version_data_with_extra_fields() {
    for opted_out in [false, true] {
        // The server receives a proposal carrying the extra field.
        {
            let (client, server, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;
            let mut server = handshake::N2NServer::new(server);
            if opted_out {
                server.disable_size_limits();
            }

            let mut propose = vec![0x82, 0x00];
            propose.extend(version_table_with_extra_field());
            send_from_peer(client, Wire(propose));

            let table = server.receive_proposed_versions().await.unwrap();
            assert_eq!(table.values.len(), 4);
            assert!(table.values.contains_key(&17));
        }

        // The client receives a query reply carrying it.
        {
            let (client, server, _link) = loopback(PROTOCOL_N2N_HANDSHAKE).await;
            let mut client = handshake::N2NClient::new(client);
            if opted_out {
                client.disable_size_limits();
            }

            let mut reply = vec![0x82, 0x03];
            reply.extend(version_table_with_extra_field());
            send_from_peer(server, Wire(reply));

            let versions = n2n::VersionTable::v7_and_above(MAINNET_MAGIC);
            match client.handshake(versions).await.unwrap() {
                handshake::Confirmation::QueryReply(table) => {
                    assert_eq!(table.values.len(), 4);
                    assert!(table.values.contains_key(&17));
                }
                other => panic!("unexpected confirmation: {other:?}"),
            }
        }
    }
}
