use pallas::network::{
    miniprotocols::{chainsync, handshake, localstate, Point, MAINNET_MAGIC},
    multiplexer::{self, bearers::Bearer},
};

#[derive(Debug)]
struct LoggingObserver;

fn do_handshake(channel: multiplexer::StdChannel) {
    let mut client = handshake::N2CClient::new(channel);

    let confirmation = client
        .handshake(handshake::n2c::VersionTable::v1_and_above(MAINNET_MAGIC))
        .unwrap();

    match confirmation {
        handshake::Confirmation::Accepted(v, _) => {
            log::info!("hand-shake accepted, using version {}", v)
        }
        handshake::Confirmation::Rejected(x) => {
            log::info!("hand-shake rejected with reason {:?}", x)
        }
    }
}

fn do_localstate_query(channel: multiplexer::StdChannel) {
    let mut client = localstate::ClientV10::new(channel);
    client.acquire(None).unwrap();

    let result = client
        .query(localstate::queries::RequestV10::GetSystemStart)
        .unwrap();

    log::info!("system start result: {:?}", result);
}

fn do_chainsync(channel: multiplexer::StdChannel) {
    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let mut client = chainsync::N2CClient::new(channel);

    let (point, _) = client.find_intersect(known_points).unwrap();

    log::info!("intersected point is {:?}", point);

    for _ in 0..10 {
        let next = client.request_next().unwrap();

        match next {
            chainsync::NextResponse::RollForward(h, _) => {
                log::info!("rolling forward, block size: {}", h.len())
            }
            chainsync::NextResponse::RollBackward(x, _) => log::info!("rollback to {:?}", x),
            chainsync::NextResponse::Await => log::info!("tip of chain reached"),
        };
    }
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    #[cfg(target_family = "unix")]
    let bearer = Bearer::connect_unix("/tmp/node.socket").unwrap();

    #[cfg(not(target_family = "unix"))]
    panic!("can't use n2c unix socket on non-unix systems");

    // setup the multiplexer by specifying the bearer and the IDs of the
    // miniprotocols to use
    let mut plexer = multiplexer::StdPlexer::new(bearer);
    let channel0 = plexer.use_channel(0);
    let channel7 = plexer.use_channel(7);
    let channel5 = plexer.use_channel(5);

    plexer.muxer.spawn();
    plexer.demuxer.spawn();

    // execute the required handshake against the relay
    do_handshake(channel0);

    // execute an arbitrary "Local State" query against the node
    do_localstate_query(channel7);

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(channel5);
}
