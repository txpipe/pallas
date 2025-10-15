// use pallas_codec::utils::AnyCbor;
// use pallas_crypto::hash::Hash;
use pallas_network::{
    facades::{PeerClient, PeerServer},
    miniprotocols::leiosnotify,
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use tokio::net::TcpListener;

#[cfg(unix)]
#[tokio::test]
pub async fn leiosnotify_server_and_client_happy_path() {
    use tracing::info;

    tracing_subscriber::fmt::init();
    
    let _block_hash: leiosnotify::Hash = hex::decode(
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    ).unwrap();

    let rb_header: leiosnotify::Header = hex::decode(
        "eade0000eade0000eade0000eade0000eade0000eade0000eade0000eade0000"
    ).unwrap();

    let _block_txs_hash: leiosnotify::Hash = hex::decode(
        "bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0"
    ).unwrap();

    let _block_slot: leiosnotify::Slot = 123456789;
    let _block_txs_slot: leiosnotify::Slot = 222222222;
    
    let _vote_issuer_id: leiosnotify::Hash = hex::decode(
        "beedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeed"
    ).unwrap();
            
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30003))
        .await
        .unwrap();

    let server = tokio::spawn({
        let sent_header = rb_header.clone();
        async move {
            // server setup

            let mut peer_server = PeerServer::accept(&listener, 0).await.unwrap();

            let server_ln = peer_server.leiosnotify();

            // server receives share request from client

            info!("server waiting for share request");

            server_ln.recv_request_next().await.unwrap();

            assert_eq!(*server_ln.state(), leiosnotify::State::Busy);

            // Server sends peer addresses

            server_ln.send_block_announcement(sent_header).await.unwrap();

            assert_eq!(*server_ln.state(), leiosnotify::State::Idle);

            // Server receives Done message from client

            server_ln.recv_request_next().await.unwrap();

            assert_eq!(*server_ln.state(), leiosnotify::State::Done);
        }
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // client setup

        let mut client_to_server_conn = PeerClient::connect("localhost:30003", 0).await.unwrap();

        let client_ln = client_to_server_conn.leiosnotify();

        // client sends peers request, receives peer addresses

        client_ln.send_request_next().await.unwrap();

        assert_eq!(
            client_ln.recv_block_announcement().await.unwrap(),
            rb_header,
        );

        // client sends Done

        client_ln.send_done().await.unwrap();

        assert!(client_ln.is_done())
    });

    tokio::try_join!(client, server).unwrap();
}
