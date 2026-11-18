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
use pallas_network::miniprotocols::chainsync::{self, HeaderContent};
use pallas_network::miniprotocols::handshake::{self, n2n};
use pallas_network::miniprotocols::localstate;
use pallas_network::miniprotocols::{
    MAINNET_MAGIC, PROTOCOL_N2C_STATE_QUERY, PROTOCOL_N2N_BLOCK_FETCH, PROTOCOL_N2N_CHAIN_SYNC,
    PROTOCOL_N2N_HANDSHAKE, Point,
};
use pallas_network::multiplexer::{
    AgentChannel, Bearer, ChannelBuffer, Header, Plexer, RunningPlexer,
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
    // A message prefix that never completes: an array whose declared length is
    // never reached, followed by filler the old code re-scanned on every
    // segment.
    let mut prefix = vec![0x82, 0x00, 0x9b]; // [0, <array of u64::MAX items> ...
    prefix.extend([0xff; 8]);

    const TARGET: usize = 2 * 1024 * 1024;
    const SEGMENT: usize = 16;

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
    client.enqueue_chunk(prefix).await.unwrap();
    let filler = vec![0u8; SEGMENT];
    let mut sent = 0;
    while sent < TARGET {
        client.enqueue_chunk(filler.clone()).await.unwrap();
        sent += SEGMENT;
    }
    let elapsed = start.elapsed();

    // Backpressure makes this track the receiver's work. Linear reassembly
    // absorbs these bytes well within this bound; the previous per-segment
    // re-scan exceeded it by far.
    assert!(
        elapsed < Duration::from_secs(20),
        "absorbing {TARGET} bytes took {elapsed:?}",
    );
}

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

    assert!(
        matches!(accepted, Err(Error::HandshakeProtocol(_))),
        "the handshake fails with a protocol error",
    );
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

/// Write a handshake proposal prefix that never completes, then stream filler
/// well past the handshake limit, as 64 KiB segments.
async fn stream_never_ending_proposal(stream: &mut TcpStream) {
    let mut prefix = vec![0x82, 0x00, 0x9b];
    prefix.extend([0xff; 8]);
    write_segment(stream, &prefix).await;

    let filler = vec![0u8; 32 * 1024];
    let mut sent = 0;
    while sent < 4 * handshake::MESSAGE_SIZE_LIMIT {
        write_segment(stream, &filler).await;
        sent += filler.len();
    }
}

/// Frame `payload` as one handshake segment and write it.
async fn write_segment(stream: &mut TcpStream, payload: &[u8]) {
    let header: [u8; HEADER_LEN] = Header {
        protocol: PROTOCOL_N2N_HANDSHAKE,
        timestamp: 0,
        payload_len: payload.len() as u16,
    }
    .into();
    stream.write_all(&header).await.unwrap();
    stream.write_all(payload).await.unwrap();
    stream.flush().await.unwrap();
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
