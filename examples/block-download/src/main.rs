use pallas::network::{
    miniprotocols::{
        blockfetch,
        handshake::{self, n2n::VersionTable},
        Point, TESTNET_MAGIC,
    },
    multiplexer::{bearers::Bearer, StdPlexer},
};

fn main() {
    env_logger::init();

    let bearer = Bearer::connect_tcp("relays-new.cardano-testnet.iohkdev.io:3001").unwrap();

    let mut plexer = StdPlexer::new(bearer);
    let channel0 = plexer.use_channel(0);
    let channel3 = plexer.use_channel(3);

    plexer.muxer.spawn();
    plexer.demuxer.spawn();

    let versions = VersionTable::v4_and_above(TESTNET_MAGIC);
    let mut hs_client = handshake::N2NClient::new(channel0);
    let handshake = hs_client.handshake(versions).unwrap();

    assert!(matches!(handshake, handshake::Confirmation::Accepted(..)));

    let point = Point::Specific(
        63528597,
        hex::decode("3f3d81c7b88f0fa28867541c5fea8794125cccf6d6c9ee0037a1dbb064130dfd").unwrap(),
    );

    let mut bf_client = blockfetch::Client::new(channel3);

    let block = bf_client.fetch_single(point).unwrap();

    println!("downloaded block of size: {}", block.len());
    println!("{}", hex::encode(&block));
}
