use net2::TcpStreamExt;
use pallas::codec::minicbor::bytes::ByteVec;
use pallas::codec::utils::CborWrap;
use std::net::TcpStream;

use pallas::codec::{DecodeOwned, Fragment};

use pallas::ledger::primitives::alonzo::network::{SubmitTx, SubmitTxRejection};
use pallas::network::miniprotocols::handshake::n2c::{Client, VersionTable};
use pallas::network::miniprotocols::localtxsubmission::{BatchClient, Message};
use pallas::network::miniprotocols::{run_agent, TESTNET_MAGIC};
use pallas::network::multiplexer::Multiplexer;

const TX: &str = &"84a60081825820e96bfea3feb098a906165a8cfa5aeefe3d5328d329605d15551fe745a21122c5000d80018182581d603e51e83bbcbc603a305bc7078ff2d671adc69507c747e0c1a3a917531a3b98256f021a0002a4910e800758203eda589740c3b110236bf6a26f45ee01fb835b25e2b4e052962a368ad9c06f18a1008182582068f67b87b242bde9ea03d92057958329802cb4b03b2a6687761387e743b0e3a65840385db9d25128a18956c8588623475129a61f27940613867bf7e769e7c87d92d50c74d23b0bb0f2b7a562c9a69bac6e2969b8c7805b5d1efbec403bd9a7082d03f5d90103a100a1190539a269636f6d706c6574656400646e616d656b68656c6c6f20776f726c64";

type AlonzoBatchClient = BatchClient<SubmitTx, SubmitTxRejection>;

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("localhost:3307").unwrap();
    // let bearer =
    // TcpStream::connect("relays-new.cardano-mainnet.iohk.io:3001").unwrap();

    bearer.set_nodelay(true).unwrap();
    bearer.set_keepalive_ms(Some(30_000u32)).unwrap();

    let mut muxer = Multiplexer::setup(bearer, &vec![0, 6]).unwrap();

    let mut hs_channel = muxer.use_channel(0);
    let versions = VersionTable::only_v10(TESTNET_MAGIC);
    let last = run_agent(Client::initial(versions), &mut hs_channel).unwrap();
    println!("{:?}", last);

    let tx = SubmitTx(CborWrap(ByteVec::from(hex::decode(TX).unwrap())));

    let mut ts_channel = muxer.use_channel(6);
    let ts = AlonzoBatchClient::initial(vec![tx]);
    let ts = run_agent(ts, &mut ts_channel).unwrap();

    println!("{:?}", ts);
}
