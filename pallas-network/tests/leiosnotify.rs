#![cfg(feature = "leios")]
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
    use tracing::debug;

    tracing_subscriber::fmt::init();

    let block_hash: leiosnotify::Hash =
        hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap();

    let rb_header: leiosnotify::Header =
        hex::decode("eade0000eade0000eade0000eade0000eade0000eade0000eade0000eade0000").unwrap();

    let block_txs_hash: leiosnotify::Hash =
        hex::decode("bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0").unwrap();

    let block_slot: leiosnotify::Slot = 123456789;
    let block_txs_slot: leiosnotify::Slot = 222222222;

    let vote_issuer_id: leiosnotify::Hash =
        hex::decode("beedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeedbeed").unwrap();

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30003))
        .await
        .unwrap();

    let server = tokio::spawn({
        let sent_header = rb_header.clone();
        let sent_block_hash = block_hash.clone();
        let sent_block_txs_hash = block_txs_hash.clone();
        let sent_vote_issuer_id = vote_issuer_id.clone();

        async move {
            // server setup

            let mut peer_server = PeerServer::accept(&listener, 0).await.unwrap();

            let server_ln = peer_server.leiosnotify();

            // server receives `RequestNext` from client
            debug!("server waiting for request next");
            server_ln.recv_request_next().await.unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Busy);

            // Server sends header
            server_ln
                .send_block_announcement(sent_header)
                .await
                .unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Idle);

            // server receives `RequestNext` from client
            debug!("server waiting for request next");
            server_ln.recv_request_next().await.unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Busy);

            // Server sends block offer
            server_ln
                .send_block_offer(block_slot, sent_block_hash)
                .await
                .unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Idle);

            // server receives `RequestNext` from client
            debug!("server waiting for request next");
            server_ln.recv_request_next().await.unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Busy);

            // Server sends txs offer
            server_ln
                .send_block_txs_offer(block_txs_slot, sent_block_txs_hash)
                .await
                .unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Idle);

            // server receives `RequestNext` from client
            debug!("server waiting for request next");
            server_ln.recv_request_next().await.unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Busy);

            // Server sends votes offer
            server_ln
                .send_vote_offer(vec![(block_slot, sent_vote_issuer_id)])
                .await
                .unwrap();
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

        // client sends `RequestNext`, receives block announcement
        client_ln.send_request_next().await.unwrap();
        assert_eq!(
            client_ln.recv_block_announcement().await.unwrap(),
            rb_header,
        );

        // client sends `RequestNext`, receives block offer
        client_ln.send_request_next().await.unwrap();
        assert_eq!(
            client_ln.recv_block_offer().await.unwrap(),
            (block_slot, block_hash),
        );

        // client sends `RequestNext`, receives tx offer
        client_ln.send_request_next().await.unwrap();
        assert_eq!(
            client_ln.recv_block_txs_offer().await.unwrap(),
            (block_txs_slot, block_txs_hash),
        );

        // client sends `RequestNext`, receives votes offer
        client_ln.send_request_next().await.unwrap();
        assert_eq!(
            client_ln.recv_vote_offer().await.unwrap(),
            vec![(block_slot, vote_issuer_id)],
        );

        // client sends Done
        client_ln.send_done().await.unwrap();
        assert!(client_ln.is_done())
    });

    tokio::try_join!(client, server).unwrap();
}

#[cfg(unix)]
#[tokio::test]
pub async fn leiosnotify_outbound_no_agency() {
    let rb_header: leiosnotify::Header =
        hex::decode("eade0000eade0000eade0000eade0000eade0000eade0000eade0000eade0000").unwrap();

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30004))
        .await
        .unwrap();

    let server = tokio::spawn({
        async move {
            // server setup

            let mut peer_server = PeerServer::accept(&listener, 0).await.unwrap();

            let server_ln = peer_server.leiosnotify();

            // server is Idle
            assert_eq!(*server_ln.state(), leiosnotify::State::Idle);

            // Server incorrectly tries to send message
            let res = server_ln.send_block_announcement(rb_header).await;

            match res {
                Err(leiosnotify::ServerError::AgencyIsTheirs) => {}
                Err(leiosnotify::ServerError::InvalidOutbound) => {
                    tracing::warn!("Expected ServerError `AgencyIsTheirs`, got `InvalidOutbound`")
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
                Ok(_) => panic!("Server has no agency"),
            }

            // Server receives Done message from client
            server_ln.recv_request_next().await.unwrap();
            assert_eq!(*server_ln.state(), leiosnotify::State::Done);
        }
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // client setup

        let mut client_to_server_conn = PeerClient::connect("localhost:30004", 0).await.unwrap();

        let client_ln = client_to_server_conn.leiosnotify();

        // client sends Done
        client_ln.send_done().await.unwrap();
        assert!(client_ln.is_done());

        // client sends `RequestNext` while not having agency
        let res = client_ln.send_request_next().await;

        match res {
            Err(leiosnotify::ClientError::ProtocolDone) => {}
            Err(leiosnotify::ClientError::InvalidOutbound) => {
                tracing::warn!("Expected ClientError `ProtocolDone`, got `InvalidOutbound`")
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
            Ok(_) => panic!("Client has no agency"),
        }
    });

    tokio::try_join!(client, server).unwrap();
}
