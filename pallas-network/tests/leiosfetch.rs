#![cfg(feature = "leios")]
use pallas_network::{
    facades::{PeerClient, PeerServer},
    miniprotocols::leiosfetch::{self, ClientRequest},
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use tokio::net::TcpListener;

#[cfg(unix)]
#[tokio::test]
pub async fn leiosfetch_server_and_client_happy_path() {
    use tracing::debug;

    tracing_subscriber::fmt::init();

    let block_hash: leiosfetch::Hash =
        hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap();

    let endorser_block: leiosfetch::EndorserBlock = vec![];

    let rb_header: leiosfetch::Header =
        hex::decode("eade0000eade0000eade0000eade0000eade0000eade0000eade0000eade0000").unwrap();

    let block_txs_hash: leiosfetch::Hash =
        hex::decode("bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0").unwrap();

    let block_slot: leiosfetch::Slot = 123456789;
    let _block_txs_slot: leiosfetch::Slot = 222222222;

    let vote_issuer_id: leiosfetch::Hash =
        hex::decode("beedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeed").unwrap();

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30003))
        .await
        .unwrap();

    let server = tokio::spawn({
        let _header = rb_header.clone();
        let block_hash = block_hash.clone();
        let block = endorser_block.clone();
        let _block_txs_hash = block_txs_hash.clone();
        let _vote_issuer_id = vote_issuer_id.clone();

        async move {
            // server setup

            let mut peer_server = PeerServer::accept(&listener, 0).await.unwrap();

            let server_lf = peer_server.leiosfetch();

            // server receives `BlockRequest` from client
            debug!("server waiting for block request");
            assert_eq!(
                server_lf.recv_while_idle().await.unwrap().unwrap(),
                ClientRequest::BlockRequest(block_slot, block_hash),
            );
            assert_eq!(*server_lf.state(), leiosfetch::State::Block);

            // Server sends EB
            server_lf.send_block(block).await.unwrap();
            assert_eq!(*server_lf.state(), leiosfetch::State::Idle);

            // Server receives Done message from client
            assert!(server_lf.recv_while_idle().await.unwrap().is_none());
            assert_eq!(*server_lf.state(), leiosfetch::State::Done);
        }
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // client setup
        let mut client_to_server_conn = PeerClient::connect("localhost:30003", 0).await.unwrap();

        let client_lf = client_to_server_conn.leiosfetch();

        // client sends `BlockRequest`, receives block announcement
        client_lf
            .send_block_request(block_slot, block_hash)
            .await
            .unwrap();
        assert_eq!(client_lf.recv_block().await.unwrap(), endorser_block);
        assert_eq!(*client_lf.state(), leiosfetch::State::Idle);

        // client sends Done
        client_lf.send_done().await.unwrap();
        assert!(client_lf.is_done())
    });

    tokio::try_join!(client, server).unwrap();
}
