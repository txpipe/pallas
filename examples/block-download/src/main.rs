use pallas::network::{
    miniprotocols::{
        blockfetch,
        handshake::{self, n2n::VersionTable},
        Point, MAINNET_MAGIC, PROTOCOL_N2N_BLOCK_FETCH, PROTOCOL_N2N_HANDSHAKE,
    },
    multiplexer::{bearers::Bearer, StdPlexer},
};

fn main() {
    env_logger::init();

    let bearer = Bearer::connect_tcp("relays-new.cardano-mainnet.iohk.io:3001").unwrap();

    let mut plexer = StdPlexer::new(bearer);
    let handshake = plexer.use_client_channel(PROTOCOL_N2N_HANDSHAKE);
    let blockfetch = plexer.use_client_channel(PROTOCOL_N2N_BLOCK_FETCH);

    plexer.muxer.spawn();
    plexer.demuxer.spawn();

    let versions = VersionTable::v4_and_above(MAINNET_MAGIC);
    let mut hs_client = handshake::N2NClient::new(handshake);
    let handshake = hs_client.handshake(versions).unwrap();

    assert!(matches!(handshake, handshake::Confirmation::Accepted(..)));

    let point = Point::Specific(
        49159253,
        hex::decode("d034a2d0e4c3076f57368ed59319010c265718f0923057f8ff914a3b6bfd1314").unwrap(),
    );

    let mut bf_client = blockfetch::Client::new(blockfetch);

    let block = bf_client.fetch_single(point).unwrap();

    println!("downloaded block of size: {}", block.len());
    println!("{}", hex::encode(&block));
}
