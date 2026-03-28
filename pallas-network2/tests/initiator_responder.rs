mod harness;

use harness::{InitiatorNode, ResponderNode};
use pallas_network2::behavior::{InitiatorCommand, InitiatorEvent};
use pallas_network2::protocol::Point;

#[tokio::test]
async fn handshake_completes() {
    let (addr, responder) = ResponderNode::bind().await.spawn();
    let mut initiator = InitiatorNode::connect_to(addr);

    let events = initiator.wait_for_handshake().await;

    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::PeerInitialized(..))));

    responder.abort();
}

#[tokio::test]
async fn chainsync_receives_headers() {
    let (addr, responder) = ResponderNode::bind().await.spawn();
    let mut initiator = InitiatorNode::connect_to(addr);

    initiator.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
    initiator.wait_for_intersection().await;

    // First header
    initiator.execute(InitiatorCommand::ContinueSync(initiator.peer_id()));
    let events = initiator.wait_for_header().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::BlockHeaderReceived(..))));

    // Second header
    initiator.execute(InitiatorCommand::ContinueSync(initiator.peer_id()));
    let events = initiator.wait_for_header().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::BlockHeaderReceived(..))));

    responder.abort();
}

#[tokio::test]
async fn blockfetch_receives_blocks() {
    let (addr, responder) = ResponderNode::bind().await.spawn();
    let mut initiator = InitiatorNode::connect_to(addr);

    initiator.wait_for_handshake().await;

    let range = (Point::Origin, Point::new(100, vec![0xAA; 32]));
    initiator.execute(InitiatorCommand::RequestBlocks(range));
    initiator.execute(InitiatorCommand::Housekeeping);

    let events = initiator.wait_for_block().await;

    assert!(events
        .iter()
        .filter(|e| matches!(e, InitiatorEvent::BlockBodyReceived(..)))
        .count()
        >= 1);

    responder.abort();
}

#[tokio::test]
async fn connection_sustained_over_time() {
    let (addr, responder) = ResponderNode::bind().await.spawn();
    let mut initiator = InitiatorNode::connect_to(addr);

    initiator.wait_for_handshake().await;

    // Keep the connection alive for 2 seconds, polling with periodic
    // housekeeping. Each housekeeping cycle exchanges keepalive messages
    // over the wire. If anything breaks, the peer gets disconnected.
    initiator.sustain(std::time::Duration::from_secs(2)).await;

    // Verify the connection is still usable after the sustained period.
    initiator.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
    initiator.wait_for_intersection().await;

    responder.abort();
}

#[tokio::test]
async fn full_protocol_flow() {
    let (addr, responder) = ResponderNode::bind().await.spawn();
    let mut initiator = InitiatorNode::connect_to(addr);

    // Phase 1: handshake + chainsync intersection
    initiator.execute(InitiatorCommand::StartSync(vec![Point::Origin]));

    let events = initiator.wait_for_intersection().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::PeerInitialized(..))));
    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::IntersectionFound(..))));

    // Phase 2: first header
    initiator.execute(InitiatorCommand::ContinueSync(initiator.peer_id()));
    let events = initiator.wait_for_header().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::BlockHeaderReceived(..))));

    // Phase 3: blockfetch
    let range = (Point::Origin, Point::new(100, vec![0xAA; 32]));
    initiator.execute(InitiatorCommand::RequestBlocks(range));
    initiator.execute(InitiatorCommand::Housekeeping);

    let events = initiator.wait_for_block().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::BlockBodyReceived(..))));

    // Phase 4: continue sync for more headers
    initiator.execute(InitiatorCommand::ContinueSync(initiator.peer_id()));
    let events = initiator.wait_for_header().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, InitiatorEvent::BlockHeaderReceived(..))));

    responder.abort();
}

#[tokio::test]
async fn multiple_initiators() {
    let (addr, responder) = ResponderNode::bind().await.spawn();

    let mut initiators: Vec<_> = (0..3).map(|_| InitiatorNode::connect_to(addr)).collect();

    for initiator in &mut initiators {
        initiator.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
    }

    for (i, initiator) in initiators.iter_mut().enumerate() {
        let events = initiator.wait_for_intersection().await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, InitiatorEvent::PeerInitialized(..))),
            "initiator {} should complete handshake",
            i
        );

        initiator.execute(InitiatorCommand::ContinueSync(initiator.peer_id()));
        let events = initiator.wait_for_header().await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, InitiatorEvent::BlockHeaderReceived(..))),
            "initiator {} should receive a header",
            i
        );
    }

    responder.abort();
}
