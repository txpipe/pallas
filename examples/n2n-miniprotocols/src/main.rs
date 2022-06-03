use net2::TcpStreamExt;

use pallas::network::{
    miniprotocols::{blockfetch, chainsync, handshake, run_agent, Point, MAINNET_MAGIC},
    multiplexer::{spawn_demuxer, spawn_muxer, use_channel, StdChannel, StdPlexer},
};

use std::net::TcpStream;

#[derive(Debug)]
struct LoggingObserver;

impl blockfetch::Observer for LoggingObserver {
    fn on_block_received(&mut self, body: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        log::trace!("block received: {}", hex::encode(&body));
        Ok(())
    }
}

impl chainsync::Observer<chainsync::HeaderContent> for LoggingObserver {
    fn on_roll_forward(
        &mut self,
        _content: chainsync::HeaderContent,
        tip: &chainsync::Tip,
    ) -> Result<chainsync::Continuation, Box<dyn std::error::Error>> {
        log::debug!("asked to roll forward, tip at {:?}", tip);

        Ok(chainsync::Continuation::Proceed)
    }

    fn on_intersect_found(
        &mut self,
        point: &Point,
        tip: &chainsync::Tip,
    ) -> Result<chainsync::Continuation, Box<dyn std::error::Error>> {
        log::debug!("intersect was found {:?} (tip: {:?})", point, tip);

        Ok(chainsync::Continuation::Proceed)
    }

    fn on_rollback(
        &mut self,
        point: &Point,
    ) -> Result<chainsync::Continuation, Box<dyn std::error::Error>> {
        log::debug!("asked to roll back {:?}", point);

        Ok(chainsync::Continuation::Proceed)
    }

    fn on_tip_reached(&mut self) -> Result<chainsync::Continuation, Box<dyn std::error::Error>> {
        log::debug!("tip was reached");

        Ok(chainsync::Continuation::Proceed)
    }
}

fn do_handshake(mut channel: StdChannel) {
    let versions = handshake::n2n::VersionTable::v4_and_above(MAINNET_MAGIC);
    let _last = run_agent(handshake::Initiator::initial(versions), &mut channel).unwrap();
}

fn do_blockfetch(mut channel: StdChannel) {
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

    let agent = run_agent(
        blockfetch::BatchClient::initial(range, LoggingObserver {}),
        &mut channel,
    );

    println!("{:?}", agent);
}

fn do_chainsync(mut channel: StdChannel) {
    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let agent = run_agent(
        chainsync::Consumer::<chainsync::HeaderContent, _>::initial(
            Some(known_points),
            LoggingObserver {},
        ),
        &mut channel,
    );

    println!("{:?}", agent);
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    // setup a TCP socket to act as data bearer between our agents and the remote
    // relay.
    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();
    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    // setup the multiplexer by specifying the bearer and the IDs of the
    // miniprotocols to use
    let mut plexer = StdPlexer::new(bearer);
    let channel0 = use_channel(&mut plexer, 0);
    let channel3 = use_channel(&mut plexer, 3);
    let channel2 = use_channel(&mut plexer, 2);

    spawn_muxer(plexer.muxer);
    spawn_demuxer(plexer.demuxer);

    // execute the required handshake against the relay
    do_handshake(channel0);

    // fetch an arbitrary batch of block
    do_blockfetch(channel3);

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(channel2);
}
