use pallas::network::{
    miniprotocols::{blockfetch, chainsync, handshake, Point, MAINNET_MAGIC},
    multiplexer::{bearers::Bearer, StdChannel, StdPlexer},
};

#[derive(Debug)]
struct LoggingObserver;

fn do_handshake(channel: StdChannel) {
    let mut client = handshake::N2NClient::new(channel);

    let confirmation = client
        .handshake(handshake::n2n::VersionTable::v7_and_above(MAINNET_MAGIC))
        .unwrap();

    match confirmation {
        handshake::Confirmation::Accepted(v, _) => {
            log::info!("hand-shake accepted, using version {}", v)
        }
        handshake::Confirmation::Rejected(x) => {
            log::info!("hand-shake rejected with reason {:?}", x)
        }
        handshake::Confirmation::QueryReply(v) => {
            log::info!("hand-shake query reply {:?}", v)
        }
    }
}

fn do_blockfetch(channel: StdChannel) {
    let range = (
        Point::Specific(
            43847831,
            hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45")
                .unwrap(),
        ),
        Point::Specific(
            43847844,
            hex::decode("ff8d558a3d5a0e058beb3d94d26a567f75cd7d09ff5485aa0d0ebc38b61378d4")
                .unwrap(),
        ),
    );

    let mut client = blockfetch::Client::new(channel);

    let blocks = client.fetch_range(range).unwrap();

    for block in blocks {
        log::info!("received block of size: {}", block.len());
    }
}

fn do_chainsync(channel: StdChannel) {
    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let mut client = chainsync::N2NClient::new(channel);

    let (point, _) = client.find_intersect(known_points).unwrap();

    log::info!("intersected point is {:?}", point);

    for _ in 0..10 {
        let next = client.request_next().unwrap();

        match next {
            chainsync::NextResponse::RollForward(h, _) => {
                log::info!("rolling forward, header size: {}", h.cbor.len())
            }
            chainsync::NextResponse::RollBackward(x, _) => log::info!("rollback to {:?}", x),
            chainsync::NextResponse::Await => log::info!("tip of chaing reached"),
        };
    }
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    // setup a TCP socket to act as data bearer between our agents and the remote
    // relay.
    let bearer = Bearer::connect_tcp("relays-new.cardano-mainnet.iohk.io:3001").unwrap();

    // setup the multiplexer by specifying the bearer and the IDs of the
    // miniprotocols to use
    let mut plexer = StdPlexer::new(bearer);
    let channel0 = plexer.use_channel(0);
    let channel3 = plexer.use_channel(3);
    let channel2 = plexer.use_channel(2);

    plexer.muxer.spawn();
    plexer.demuxer.spawn();

    // execute the required handshake against the relay
    do_handshake(channel0);

    // fetch an arbitrary batch of block
    do_blockfetch(channel3);

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(channel2);
}
