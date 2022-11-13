use pallas_miniprotocols::{
    blockfetch,
    chainsync::{self, NextResponse},
    handshake::{self, Confirmation},
    Point,
};
use pallas_multiplexer::{bearers::Bearer, StdChannel, StdPlexer};

struct N2NChannels {
    channel2: StdChannel,
    channel3: StdChannel,
}

fn setup_n2n_connection() -> N2NChannels {
    let bearer = Bearer::connect_tcp("preview-node.world.dev.cardano.org:30002").unwrap();
    let mut plexer = StdPlexer::new(bearer);

    let channel0 = plexer.use_channel(0);
    let channel2 = plexer.use_channel(2);
    let channel3 = plexer.use_channel(3);

    plexer.muxer.spawn();
    plexer.demuxer.spawn();

    let mut client = handshake::N2NClient::new(channel0);

    let confirmation = client
        .handshake(handshake::n2n::VersionTable::v7_and_above(2))
        .unwrap();

    assert!(matches!(confirmation, Confirmation::Accepted(..)));

    if let Confirmation::Accepted(v, _) = confirmation {
        assert!(v >= 7);
    }

    N2NChannels { channel2, channel3 }
}

#[test]
#[ignore]
pub fn chainsync_history_happy_path() {
    let N2NChannels { channel2, .. } = setup_n2n_connection();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let mut client = chainsync::N2NClient::new(channel2);

    let (point, _) = client.find_intersect(vec![known_point.clone()]).unwrap();

    assert!(matches!(client.state(), chainsync::State::Idle));

    match point {
        Some(point) => assert_eq!(point, known_point.clone()),
        None => panic!("expected point"),
    }

    let next = client.request_next().unwrap();

    match next {
        NextResponse::RollBackward(point, _) => assert_eq!(point, known_point.clone()),
        _ => panic!("expected rollback"),
    }

    assert!(matches!(client.state(), chainsync::State::Idle));

    for _ in 0..10 {
        let next = client.request_next().unwrap();

        match next {
            NextResponse::RollForward(_, _) => (),
            _ => panic!("expected roll-forward"),
        }

        assert!(matches!(client.state(), chainsync::State::Idle));
    }

    client.send_done().unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[test]
#[ignore]
pub fn chainsync_tip_happy_path() {
    let N2NChannels { channel2, .. } = setup_n2n_connection();

    let mut client = chainsync::N2NClient::new(channel2);

    client.intersect_tip().unwrap();

    assert!(matches!(client.state(), chainsync::State::Idle));

    let next = client.request_next().unwrap();

    assert!(matches!(next, NextResponse::RollBackward(..)));

    let mut await_count = 0;

    for _ in 0..4 {
        let next = if client.has_agency() {
            client.request_next().unwrap()
        } else {
            await_count += 1;
            client.recv_while_must_reply().unwrap()
        };

        match next {
            NextResponse::RollForward(_, _) => (),
            NextResponse::Await => (),
            _ => panic!("expected roll-forward or await"),
        }
    }

    assert!(await_count > 0, "tip was never reached");

    client.send_done().unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[test]
#[ignore]
pub fn blockfetch_happy_path() {
    let N2NChannels { channel3, .. } = setup_n2n_connection();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let mut client = blockfetch::Client::new(channel3);

    let range_ok = client.request_range((known_point.clone(), known_point.clone()));

    assert!(matches!(client.state(), blockfetch::State::Streaming));

    assert!(matches!(range_ok, Ok(_)));

    for _ in 0..1 {
        let next = client.recv_while_streaming().unwrap();

        match next {
            Some(body) => assert_eq!(body.len(), 3251),
            _ => panic!("expected block body"),
        }

        assert!(matches!(client.state(), blockfetch::State::Streaming));
    }

    let next = client.recv_while_streaming().unwrap();

    assert!(matches!(next, None));

    client.send_done().unwrap();

    assert!(matches!(client.state(), blockfetch::State::Done));
}
