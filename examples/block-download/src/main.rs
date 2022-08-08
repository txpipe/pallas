use pallas::network::{
    miniprotocols::{
        handshake::{n2n::VersionTable, Initiator},
        run_agent, Point, TESTNET_MAGIC,
    },
    multiplexer::{bearers::Bearer, StdPlexer},
};

use pallas::network::miniprotocols::blockfetch::{BatchClient, Observer};

#[derive(Debug)]
struct BlockPrinter;

impl Observer for BlockPrinter {
    fn on_block_received(&mut self, body: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", hex::encode(&body));
        println!("----------");
        Ok(())
    }
}

fn main() {
    env_logger::init();

    let bearer = Bearer::connect_tcp("relays-new.cardano-testnet.iohkdev.io:3001").unwrap();

    let mut plexer = StdPlexer::new(bearer);
    let mut channel0 = plexer.use_channel(0).into();
    let mut channel3 = plexer.use_channel(3).into();

    plexer.muxer.spawn();
    plexer.demuxer.spawn();

    let versions = VersionTable::v4_and_above(TESTNET_MAGIC);
    let _last = run_agent(Initiator::initial(versions), &mut channel0).unwrap();

    let range = (
        Point::Specific(
            63528597,
            hex::decode("3f3d81c7b88f0fa28867541c5fea8794125cccf6d6c9ee0037a1dbb064130dfd")
                .unwrap(),
        ),
        Point::Specific(
            63528597,
            hex::decode("3f3d81c7b88f0fa28867541c5fea8794125cccf6d6c9ee0037a1dbb064130dfd")
                .unwrap(),
        ),
    );

    let bf = BatchClient::initial(range, BlockPrinter {});
    let bf_last = run_agent(bf, &mut channel3);
    println!("{:?}", bf_last);
}
