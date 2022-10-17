use pallas_miniprotocols::{
    chainsync::{self, NextResponse},
    handshake::{self, Confirmation},
    Point,
};
use pallas_multiplexer::{bearers::Bearer, StdPlexer};

#[test]
pub fn chainsync_happy_path() {
    env_logger::builder().init();

    let bearer = Bearer::connect_tcp("preview-node.world.dev.cardano.org:30002").unwrap();
    let mut plexer = StdPlexer::new(bearer);

    let channel0 = plexer.use_channel(0);
    let channel2 = plexer.use_channel(2);

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

    let KNOWN_POINT = Point::Specific(
        5953863,
        hex::decode("7e44cb1e230b686875ae6a256b95c9b4eea7c9e9a9d046b626ed69d4c1b9bfe1").unwrap(),
    );

    let mut client = chainsync::N2NClient::new(channel2);

    let (point, _) = client.find_intersect(vec![KNOWN_POINT.clone()]).unwrap();

    assert!(matches!(client.state(), chainsync::State::Idle));

    match point {
        Some(point) => assert_eq!(point, KNOWN_POINT.clone()),
        None => panic!("expected point"),
    }

    let next = client.request_next().unwrap();

    match next {
        NextResponse::RollBackward(point, _) => assert_eq!(point, KNOWN_POINT.clone()),
        _ => panic!("expected rollback"),
    }

    assert!(matches!(client.state(), chainsync::State::Idle));

    for _ in [0..10] {
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
