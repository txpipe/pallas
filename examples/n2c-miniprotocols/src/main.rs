use pallas::network::{
    miniprotocols::{chainsync, handshake, localstate, run_agent, Point, MAINNET_MAGIC},
    multiplexer::{threads, Channel, Multiplexer},
};

use std::os::unix::net::UnixStream;

#[derive(Debug)]
struct LoggingObserver;

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

fn do_handshake(mut channel: Channel) {
    let versions = handshake::n2c::VersionTable::v1_and_above(MAINNET_MAGIC);
    let _last = run_agent(handshake::Initiator::initial(versions), &mut channel).unwrap();
}

fn do_localstate_query(mut channel: Channel) {
    let agent = run_agent(
        localstate::OneShotClient::<localstate::queries::QueryV10>::initial(
            None,
            localstate::queries::RequestV10::GetChainPoint,
        ),
        &mut channel,
    );

    log::info!("state query result: {:?}", agent);
}

fn do_chainsync(mut channel: Channel) {
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

    // we connect to the unix socket of the local node. Make sure you have the right
    // path for your environment
    let bearer = UnixStream::connect("/tmp/node.socket").unwrap();

    // setup the multiplexer by specifying the bearer and the IDs of the
    // miniprotocols to use
    let mut plexer = Multiplexer::new(bearer);
    let channel0 = plexer.use_channel(0);
    let channel7 = plexer.use_channel(7);
    let channel5 = plexer.use_channel(5);

    threads::spawn_muxer(plexer.muxer);
    threads::spawn_demuxer(plexer.demuxer);

    // execute the required handshake against the relay
    do_handshake(channel0);

    // execute an arbitrary "Local State" query against the node
    do_localstate_query(channel7);

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(channel5);
}
