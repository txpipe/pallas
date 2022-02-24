use net2::TcpStreamExt;

use pallas::{
    ledger::primitives::{byron::Block, Fragment},
    network::{
        miniprotocols::{
            blockfetch::{BatchClient, Observer},
            handshake::n2n::{Client, VersionTable},
            run_agent, Point, MAINNET_MAGIC,
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

    let bearer = TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();
    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &[0, 3]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::v4_and_above(MAINNET_MAGIC);
    let _last = run_agent(Client::initial(versions), &mut hs_channel).unwrap();

    let range = (
        Point::Specific(
            3240000,
            hex::decode("b7096a881f77ced24bdd285758646c0e059545b54855bd3a2307ece518bd6317")
                .unwrap(),
        ),
        Point::Specific(
            4492794,
            hex::decode("5c196e7394ace0449ba5a51c919369699b13896e97432894b4f0354dce8670b6")
                .unwrap(),
        ),
    );

    let mut bf_channel = muxer.use_channel(3);
    let bf = BatchClient::initial(range, BlockPrinter {});
    let bf_last = run_agent(bf, &mut bf_channel);
    println!("{:?}", bf_last);
}
