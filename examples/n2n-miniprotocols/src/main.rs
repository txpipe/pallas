use pallas::network::{
    miniprotocols::{blockfetch, chainsync, handshake, run_agent, Point, MAINNET_MAGIC},
    multiplexer::{agents::ChannelBuffer, bearers::Bearer, StdChannel, StdPlexer},
};

#[derive(Debug)]
struct LoggingObserver {
    block_counter: u64,
}

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
        self.block_counter += 1;

        log::info!(
            "asked to roll forward, total blocks: {}",
            self.block_counter
        );

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

fn do_handshake(mut channel: ChannelBuffer<StdChannel>) {
    let versions = handshake::n2n::VersionTable::v4_and_above(MAINNET_MAGIC);
    let _last = run_agent(handshake::Initiator::initial(versions), &mut channel).unwrap();
}

fn do_blockfetch(mut channel: ChannelBuffer<StdChannel>) {
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
        blockfetch::BatchClient::initial(range, LoggingObserver { block_counter: 0 }),
        &mut channel,
    );

    println!("{:?}", agent);
}

fn do_chainsync(mut channel: ChannelBuffer<StdChannel>) {
    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45").unwrap(),
    )];

    let agent = run_agent(
        chainsync::Consumer::<chainsync::HeaderContent, _>::initial(
            Some(known_points),
            LoggingObserver { block_counter: 0 },
        ),
        &mut channel,
    );

    println!("{:?}", agent);
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
    let channel0 = plexer.use_channel(0).into();
    let channel3 = plexer.use_channel(3).into();
    let channel2 = plexer.use_channel(2).into();

    plexer.muxer.spawn(plexer.ingress_parking.clone());
    plexer.demuxer.spawn();

    // execute the required handshake against the relay
    do_handshake(channel0);

    // fetch an arbitrary batch of block
    do_blockfetch(channel3);

    // execute the chainsync flow from an arbitrary point in the chain
    do_chainsync(channel2);
}
