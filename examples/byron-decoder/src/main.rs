use net2::TcpStreamExt;

use pallas::{
    ledger::primitives::{byron::Block, Fragment},
    network::{
        miniprotocols::{
            blockfetch::{BatchClient, Observer},
            handshake::n2n::{Client, VersionTable},
            run_agent, Point, TESTNET_MAGIC,
        },
        multiplexer::Multiplexer,
    },
};

use std::net::TcpStream;

#[derive(Debug)]
struct BlockPrinter;

impl Observer for BlockPrinter {
    fn on_block_received(&self, body: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", hex::encode(&body));
        println!("----------");

        let block = Block::decode_fragment(body.as_slice()).unwrap();
        println!("{:?}", block);
        println!("===========\n\n");

        Ok(())
    }
}

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("relays-new.cardano-testnet.iohkdev.io:3001").unwrap();
    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &[0, 3]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::v4_and_above(TESTNET_MAGIC);
    let _last = run_agent(Client::initial(versions), &mut hs_channel).unwrap();

    let range = (
        Point::Specific(
            23470073,
            hex::decode("333b55ab6e013b8e4fdf19d05dbf33aa0d58a59a2b1b86d0c75f58ff76a9e565")
                .unwrap(),
        ),
        Point::Specific(
            51278306,
            hex::decode("936a8e8387d68e8497216d4cee8ec3810bae3902aba5c7b8ab911ad36984d6ad")
                .unwrap(),
        ),
    );

    let mut bf_channel = muxer.use_channel(3);
    let bf = BatchClient::initial(range, BlockPrinter {});
    let bf_last = run_agent(bf, &mut bf_channel);
    println!("{:?}", bf_last);
}
