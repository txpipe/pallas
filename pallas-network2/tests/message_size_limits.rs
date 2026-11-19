//! A peer that sends a message larger than its channel's size limit, or bytes
//! that are not a message, is dropped.

// Only the responder node of the shared harness is used here.
#[allow(dead_code, unused_imports)]
mod harness;

use std::time::Duration;

use futures::StreamExt;
use pallas_codec::minicbor;
use pallas_network2::behavior::AnyMessage;
use pallas_network2::interface::{TcpInterface, TcpListenerInterface};
use pallas_network2::protocol::handshake;
use pallas_network2::{Interface, InterfaceCommand, InterfaceEvent, Message, PeerId};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use harness::ResponderNode;

const TIMEOUT: Duration = Duration::from_secs(10);

/// One segment on `channel`, sent by the initiator side.
fn segment(channel: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0, 0, 0, 0];
    out.extend(channel.to_be_bytes());
    out.extend((payload.len() as u16).to_be_bytes());
    out.extend(payload);
    out
}

/// The start of a handshake proposal whose version map never ends.
fn never_ending_proposal() -> Vec<u8> {
    let mut start = vec![0x82, 0x00, 0xbb];
    start.extend([0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff]);
    start
}

/// One whole, valid entry of a version map: version 14 with its data.
fn version_entry() -> Vec<u8> {
    let data = handshake::n2n::VersionData::new(1, false, Some(0), Some(false));
    let mut out = minicbor::to_vec(14u64).unwrap();
    out.extend(minicbor::to_vec(data).unwrap());
    out
}

/// Write `segments` until the peer closes, then wait for it to close.
async fn assert_peer_closes(stream: &mut TcpStream, segments: impl Iterator<Item = Vec<u8>>) {
    for segment in segments {
        if stream.write_all(&segment).await.is_err() {
            break;
        }
    }

    let mut buf = [0u8; 1024];
    let closed = tokio::time::timeout(TIMEOUT, async {
        loop {
            match stream.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(_) => continue,
            }
        }
    })
    .await;

    assert!(closed.is_ok(), "the peer kept the connection open");
}

#[tokio::test]
async fn a_message_that_never_ends_drops_the_connection() {
    let (addr, responder) = ResponderNode::bind().await.spawn();
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let entries = version_entry().repeat(40);
    let segments = std::iter::once(segment(handshake::CHANNEL_ID, &never_ending_proposal())).chain(
        std::iter::repeat_n(segment(handshake::CHANNEL_ID, &entries), 200),
    );

    assert_peer_closes(&mut stream, segments).await;
    responder.abort();
}

#[tokio::test]
async fn a_malformed_message_drops_the_connection() {
    let (addr, responder) = ResponderNode::bind().await.spawn();
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let not_a_handshake = minicbor::to_vec((99u8, "nope")).unwrap();
    let segments = std::iter::once(segment(handshake::CHANNEL_ID, &not_a_handshake));

    assert_peer_closes(&mut stream, segments).await;
    responder.abort();
}

/// A handshake message of about 10,000 bytes, over the 5,760-byte limit.
fn oversized_refusal() -> Vec<u8> {
    AnyMessage::Handshake(handshake::Message::Refuse(
        handshake::RefuseReason::Refused(14, "x".repeat(10_000)),
    ))
    .payload()
}

/// The first event about a peer other than its connection.
async fn next_peer_event<I: Interface<AnyMessage>>(
    interface: &mut I,
) -> InterfaceEvent<AnyMessage> {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            match interface.next().await {
                Some(InterfaceEvent::Connected(_)) | Some(InterfaceEvent::Idle) => continue,
                Some(InterfaceEvent::Recv(_, msgs)) if msgs.is_empty() => continue,
                Some(event) => return event,
                None => panic!("interface ended"),
            }
        }
    })
    .await
    .expect("no event from the interface")
}

async fn listener_receives_oversized(disable: bool) -> InterfaceEvent<AnyMessage> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut interface = TcpListenerInterface::<AnyMessage>::new(listener);
    if disable {
        interface.disable_size_limits();
    }

    let mut stream = TcpStream::connect(addr).await.unwrap();
    let bytes = oversized_refusal();
    for chunk in bytes.chunks(4_000) {
        stream
            .write_all(&segment(handshake::CHANNEL_ID, chunk))
            .await
            .unwrap();
    }

    next_peer_event(&mut interface).await
}

async fn initiator_receives_oversized(disable: bool) -> InterfaceEvent<AnyMessage> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut interface = TcpInterface::<AnyMessage>::new();
    if disable {
        interface.disable_size_limits();
    }

    interface.dispatch(InterfaceCommand::Connect(PeerId {
        host: addr.ip().to_string(),
        port: addr.port(),
    }));

    let remote = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let bytes = oversized_refusal();
        for chunk in bytes.chunks(4_000) {
            // Responder mode bit set, as a remote node replies.
            let seg = segment(handshake::CHANNEL_ID | 0x8000, chunk);
            stream.write_all(&seg).await.unwrap();
        }
        stream
    });

    let event = next_peer_event(&mut interface).await;
    drop(remote.await.unwrap());
    event
}

fn assert_refused(event: InterfaceEvent<AnyMessage>) {
    match event {
        InterfaceEvent::Error(_, pallas_network2::InterfaceError::Other(msg)) => {
            assert!(
                msg.contains("exceeds the size limit of 5760 bytes"),
                "{msg}"
            );
        }
        other => panic!("expected an error, got {other:?}"),
    }
}

fn assert_delivered(event: InterfaceEvent<AnyMessage>) {
    match event {
        InterfaceEvent::Recv(_, msgs) => {
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0].payload(), oversized_refusal());
        }
        other => panic!("expected the message, got {other:?}"),
    }
}

#[tokio::test]
async fn oversized_message_is_refused_by_the_listener() {
    assert_refused(listener_receives_oversized(false).await);
}

#[tokio::test]
async fn listener_without_size_limits_accepts_an_oversized_message() {
    assert_delivered(listener_receives_oversized(true).await);
}

#[tokio::test]
async fn oversized_message_is_refused_by_the_initiator() {
    assert_refused(initiator_receives_oversized(false).await);
}

#[tokio::test]
async fn initiator_without_size_limits_accepts_an_oversized_message() {
    assert_delivered(initiator_receives_oversized(true).await);
}
