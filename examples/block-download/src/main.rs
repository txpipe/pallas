use net2::TcpStreamExt;

use pallas::ledger::alonzo::*;
use pallas::ouroboros::network::blockfetch::{BatchClient, Observer};
use pallas::ouroboros::network::handshake::{
    n2n::{Client, VersionTable},
    MAINNET_MAGIC,
};
use pallas::ouroboros::network::machines::primitives::Point;
use pallas::ouroboros::network::machines::run_agent;
use pallas::ouroboros::network::multiplexer::Multiplexer;
use std::net::TcpStream;

#[derive(Debug)]
struct BlockPrinter;

impl Observer for BlockPrinter {
    fn on_block_received(&self, body: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", hex::encode(&body));
        println!("----------");
        BlockWrapper::decode_fragment(&body[..])?;
        Ok(())
    }
}

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();
    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 3]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::v4_and_above(MAINNET_MAGIC);
    let last = run_agent(Client::initial(versions), &mut hs_channel).unwrap();

    let range = (
        Point(
            26249860,
            hex::decode("915386f44ad3a7fccee949c9d3fe43f5a20459c7401f990e1cc7d52c10be1fd6")
                .unwrap(),
        ),
        Point(
            26250057,
            hex::decode("5fec758c8aaff4a7683c27b075dc3984d8d982839cc56470a682d1411c9f8198")
                .unwrap(),
        ),
    );

    let mut bf_channel = muxer.use_channel(3);
    let bf = BatchClient::initial(range, BlockPrinter {});
    let bf_last = run_agent(bf, &mut bf_channel);
    println!("{:?}", bf_last);
}
