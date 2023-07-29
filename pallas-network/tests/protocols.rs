use pallas_network::facades::PeerClient;
use pallas_network::miniprotocols::{
    blockfetch,
    chainsync::{self, NextResponse},
    Point,
};

#[tokio::test]
#[ignore]
pub async fn chainsync_history_happy_path() {
    let mut peer = PeerClient::connect("preview-node.world.dev.cardano.org:30002", 2)
        .await
        .unwrap();

    let client = peer.chainsync();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let (point, _) = client
        .find_intersect(vec![known_point.clone()])
        .await
        .unwrap();

    println!("{:?}", point);

    assert!(matches!(client.state(), chainsync::State::Idle));

    match point {
        Some(point) => assert_eq!(point, known_point),
        None => panic!("expected point"),
    }

    let next = client.request_next().await.unwrap();

    match next {
        NextResponse::RollBackward(point, _) => assert_eq!(point, known_point),
        _ => panic!("expected rollback"),
    }

    assert!(matches!(client.state(), chainsync::State::Idle));

    for _ in 0..10 {
        let next = client.request_next().await.unwrap();

        match next {
            NextResponse::RollForward(_, _) => (),
            _ => panic!("expected roll-forward"),
        }

        assert!(matches!(client.state(), chainsync::State::Idle));
    }

    client.send_done().await.unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[tokio::test]
#[ignore]
pub async fn chainsync_tip_happy_path() {
    let mut peer = PeerClient::connect("preview-node.world.dev.cardano.org:30002", 2)
        .await
        .unwrap();

    let client = peer.chainsync();

    client.intersect_tip().await.unwrap();

    assert!(matches!(client.state(), chainsync::State::Idle));

    let next = client.request_next().await.unwrap();

    assert!(matches!(next, NextResponse::RollBackward(..)));

    let mut await_count = 0;

    for _ in 0..4 {
        let next = if client.has_agency() {
            client.request_next().await.unwrap()
        } else {
            await_count += 1;
            client.recv_while_must_reply().await.unwrap()
        };

        match next {
            NextResponse::RollForward(_, _) => (),
            NextResponse::Await => (),
            _ => panic!("expected roll-forward or await"),
        }
    }

    assert!(await_count > 0, "tip was never reached");

    client.send_done().await.unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[tokio::test]
#[ignore]
pub async fn blockfetch_happy_path() {
    let mut peer = PeerClient::connect("preview-node.world.dev.cardano.org:30002", 2)
        .await
        .unwrap();

    let client = peer.blockfetch();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let range_ok = client
        .request_range((known_point.clone(), known_point))
        .await;

    assert!(matches!(client.state(), blockfetch::State::Streaming));

    println!("streaming...");

    assert!(matches!(range_ok, Ok(_)));

    for _ in 0..1 {
        let next = client.recv_while_streaming().await.unwrap();

        match next {
            Some(body) => assert_eq!(body.len(), 3251),
            _ => panic!("expected block body"),
        }

        assert!(matches!(client.state(), blockfetch::State::Streaming));
    }

    let next = client.recv_while_streaming().await.unwrap();

    assert!(matches!(next, None));

    client.send_done().await.unwrap();

    assert!(matches!(client.state(), blockfetch::State::Done));
}

// TODO: redo txsubmission client test
