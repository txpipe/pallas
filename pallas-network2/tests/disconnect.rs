//! Reproduces txpipe/pallas#868. These tests fail on purpose until it is fixed.

use std::time::Duration;

use futures::StreamExt;
use pallas_network2::{
    Interface, InterfaceCommand, InterfaceEvent, PeerId, behavior::AnyMessage,
    interface::TcpInterface, protocol::blockfetch,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};

const LONG_ENOUGH_FOR_AN_EXPECTED_EVENT: Duration = Duration::from_secs(2);
const LONG_ENOUGH_TO_RULE_OUT_AN_EVENT: Duration = Duration::from_millis(500);
const FEWER_BYTES_THAN_A_SEGMENT_HEADER: usize = 7;
const FAR_MORE_THAN_THE_SOCKET_BUFFERS: usize = 64 * 1024 * 1024;

struct RemotePeer {
    listener: TcpListener,
    id: PeerId,
}

impl RemotePeer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let id = PeerId {
            host: addr.ip().to_string(),
            port: addr.port(),
        };

        Self { listener, id }
    }

    async fn accepts_our_connection(&self) -> RemoteEndOfTheConnection {
        let (socket, _) = self.listener.accept().await.unwrap();
        RemoteEndOfTheConnection(socket)
    }

    async fn accepts_our_connection_but_never_reads_it(&self) -> RemoteEndOfTheConnection {
        self.accepts_our_connection().await
    }
}

struct RemoteEndOfTheConnection(TcpStream);

impl RemoteEndOfTheConnection {
    async fn must_no_longer_be_able_to_write_to_us(&mut self) {
        for _ in 0..FEWER_BYTES_THAN_A_SEGMENT_HEADER {
            let our_kernel_reset_the_connection = self.0.write_all(&[0]).await.is_err();

            if our_kernel_reset_the_connection {
                return;
            }

            sleep(Duration::from_millis(100)).await;
        }

        panic!("our read half is still open");
    }
}

async fn our_next_event_within(
    us: &mut TcpInterface<AnyMessage>,
    wait: Duration,
) -> Option<InterfaceEvent<AnyMessage>> {
    timeout(wait, us.next()).await.ok().flatten()
}

async fn we_connect_to(us: &mut TcpInterface<AnyMessage>, peer: &RemotePeer) {
    us.dispatch(InterfaceCommand::Connect(peer.id.clone()));

    match our_next_event_within(us, LONG_ENOUGH_FOR_AN_EXPECTED_EVENT).await {
        Some(InterfaceEvent::Connected(id)) => assert_eq!(id, peer.id),
        other => panic!("expected Connected, got {other:?}"),
    }
}

async fn we_disconnect_from(us: &mut TcpInterface<AnyMessage>, peer: &RemotePeer) {
    we_can_disconnect_from(us, peer).await;
}

async fn we_can_disconnect_from(us: &mut TcpInterface<AnyMessage>, peer: &RemotePeer) {
    us.dispatch(InterfaceCommand::Disconnect(peer.id.clone()));

    match our_next_event_within(us, LONG_ENOUGH_FOR_AN_EXPECTED_EVENT).await {
        Some(InterfaceEvent::Disconnected(id)) => assert_eq!(id, peer.id),
        None => panic!("Disconnect did not complete: no Disconnected event"),
        other => panic!("expected Disconnected, got {other:?}"),
    }
}

async fn we_send_more_than_the_socket_buffers_can_hold(
    us: &mut TcpInterface<AnyMessage>,
    peer: &RemotePeer,
) {
    let huge_block = AnyMessage::BlockFetch(blockfetch::Message::Block(vec![
        0;
        FAR_MORE_THAN_THE_SOCKET_BUFFERS
    ]));

    us.dispatch(InterfaceCommand::Send(peer.id.clone(), huge_block));

    we_must_stay_silent(us, "our send completed: it should be blocked").await;
}

async fn we_must_stay_silent(us: &mut TcpInterface<AnyMessage>, failure: &str) {
    if let Some(event) = our_next_event_within(us, LONG_ENOUGH_TO_RULE_OUT_AN_EVENT).await {
        panic!("{failure}: {event:?}");
    }
}

#[tokio::test]
async fn disconnect_closes_the_socket() {
    let peer = RemotePeer::start().await;
    let mut us = TcpInterface::<AnyMessage>::new();

    we_connect_to(&mut us, &peer).await;
    let mut remote_end = peer.accepts_our_connection().await;

    we_disconnect_from(&mut us, &peer).await;

    remote_end.must_no_longer_be_able_to_write_to_us().await;
}

#[tokio::test]
async fn disconnect_completes_while_a_send_is_blocked() {
    let peer = RemotePeer::start().await;
    let mut us = TcpInterface::<AnyMessage>::new();

    we_connect_to(&mut us, &peer).await;
    let _remote_end = peer.accepts_our_connection_but_never_reads_it().await;

    we_send_more_than_the_socket_buffers_can_hold(&mut us, &peer).await;

    we_can_disconnect_from(&mut us, &peer).await;
}
