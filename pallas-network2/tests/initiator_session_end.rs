#![cfg(feature = "unstable")]

use std::time::Duration;

use tokio::net::TcpListener;

use pallas_network2::behavior::responder::{ResponderBehavior, ResponderCommand, ResponderEvent};
use pallas_network2::behavior::{
    AnyMessage, DisconnectReason, InitiatorBehavior, InitiatorCommand, InitiatorEvent,
};
use pallas_network2::interface::{TcpInterface, TcpListenerInterface};
use pallas_network2::{Manager, PeerId};

type Initiator = Manager<TcpInterface<AnyMessage>, InitiatorBehavior, AnyMessage>;

const WINDOW: Duration = Duration::from_millis(300);

const SETTLE: Duration = Duration::from_millis(50);

const LIMIT: Duration = Duration::from_secs(10);

async fn bound() -> (TcpListener, PeerId) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let pid = PeerId {
        host: addr.ip().to_string(),
        port: addr.port(),
    };

    (listener, pid)
}

async fn observe(
    manager: &mut Initiator,
    pid: &PeerId,
    initialized: &mut bool,
    reasons: &mut Vec<DisconnectReason>,
) {
    match manager.poll_next().await {
        Some(InitiatorEvent::PeerInitialized(p, _)) if p == *pid => *initialized = true,
        Some(InitiatorEvent::PeerDisconnected(p, reason)) if p == *pid => reasons.push(reason),
        _ => {}
    }
}

async fn ends_until(
    manager: &mut Initiator,
    pid: &PeerId,
    until: impl Fn(bool, &[DisconnectReason]) -> bool,
    window: Duration,
) -> (bool, Vec<DisconnectReason>) {
    let mut initialized = false;
    let mut reasons = Vec::new();

    let _ = tokio::time::timeout(LIMIT, async {
        while !until(initialized, &reasons) {
            observe(manager, pid, &mut initialized, &mut reasons).await;
        }
    })
    .await;
    let _ = tokio::time::timeout(window, async {
        loop {
            observe(manager, pid, &mut initialized, &mut reasons).await;
        }
    })
    .await;

    (initialized, reasons)
}

fn on_init(initialized: bool, _: &[DisconnectReason]) -> bool {
    initialized
}

fn on_end(_: bool, reasons: &[DisconnectReason]) -> bool {
    !reasons.is_empty()
}

fn now(_: bool, _: &[DisconnectReason]) -> bool {
    true
}

#[tokio::test]
async fn a_demote_a_remote_close_and_a_refused_connect_emit_closed_errored_errored() {
    let (listener, demoted) = bound().await;
    let responder = tokio::spawn(async move {
        let mut responder = Manager::new(
            TcpListenerInterface::<AnyMessage>::new(listener),
            ResponderBehavior::default(),
        );
        loop {
            if let Some(ResponderEvent::PeerInitialized(..)) = responder.poll_next().await {
                responder.execute(ResponderCommand::Housekeeping);
            }
        }
    });
    let (remote, closed) = bound().await;
    let remote = tokio::spawn(async move {
        let _ = remote.accept().await;
    });
    let (unused, refused) = bound().await;
    drop(unused);
    let mut initiator = Initiator::new(TcpInterface::new(), InitiatorBehavior::default());

    initiator.execute(InitiatorCommand::IncludePeer(demoted.clone()));
    initiator.execute(InitiatorCommand::Housekeeping);
    let (initialized, mut local) = ends_until(&mut initiator, &demoted, on_init, WINDOW).await;
    // Promotes the peer to hot, which the demote needs before it closes the connection.
    initiator.execute(InitiatorCommand::Housekeeping);
    local.extend(ends_until(&mut initiator, &demoted, now, SETTLE).await.1);
    initiator.execute(InitiatorCommand::DemotePeer(demoted.clone()));
    initiator.execute(InitiatorCommand::Housekeeping);
    local.extend(ends_until(&mut initiator, &demoted, on_end, WINDOW).await.1);
    responder.abort();

    initiator.execute(InitiatorCommand::IncludePeer(closed.clone()));
    initiator.execute(InitiatorCommand::Housekeeping);
    let (_, remote_closed) = ends_until(&mut initiator, &closed, on_end, WINDOW).await;
    remote.abort();

    initiator.execute(InitiatorCommand::IncludePeer(refused.clone()));
    initiator.execute(InitiatorCommand::Housekeeping);
    let (_, refused_connect) = ends_until(&mut initiator, &refused, on_end, WINDOW).await;

    assert_eq!(
        (initialized, local, remote_closed, refused_connect),
        (
            true,
            vec![DisconnectReason::Closed],
            vec![DisconnectReason::Errored],
            vec![DisconnectReason::Errored]
        ),
        "initialized, then the reasons emitted for a local demote, a remote close and a refused connect"
    );
}
