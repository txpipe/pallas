use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use pallas_network::facades::PeerClient;
use pallas_network::miniprotocols::blockfetch::BlockRequest;
use pallas_network::miniprotocols::handshake;
use pallas_network::miniprotocols::handshake::n2n::VersionData;
use pallas_network::miniprotocols::{
    blockfetch,
    chainsync::{self, NextResponse},
    Point,
};
use pallas_network::multiplexer::{Bearer, Plexer};
use tokio::net::TcpListener;

#[tokio::test]
#[ignore]
pub async fn chainsync_history_happy_path() {
    let mut peer = PeerClient::connect("preview-node.world.dev.cardano.org:30002", 2)
        .await
        .unwrap();

    let client = peer.chainsync();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let (point, _) = client
        .find_intersect(vec![known_point.clone()])
        .await
        .unwrap();

    println!("{:?}", point);

    assert!(matches!(client.state(), chainsync::State::Idle));

    match point {
        Some(point) => assert_eq!(point, known_point),
        None => panic!("expected point"),
    }

    let next = client.request_next().await.unwrap();

    match next {
        NextResponse::RollBackward(point, _) => assert_eq!(point, known_point),
        _ => panic!("expected rollback"),
    }

    assert!(matches!(client.state(), chainsync::State::Idle));

    for _ in 0..10 {
        let next = client.request_next().await.unwrap();

        match next {
            NextResponse::RollForward(_, _) => (),
            _ => panic!("expected roll-forward"),
        }

        assert!(matches!(client.state(), chainsync::State::Idle));
    }

    client.send_done().await.unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[tokio::test]
#[ignore]
pub async fn chainsync_tip_happy_path() {
    let mut peer = PeerClient::connect("preview-node.world.dev.cardano.org:30002", 2)
        .await
        .unwrap();

    let client = peer.chainsync();

    client.intersect_tip().await.unwrap();

    assert!(matches!(client.state(), chainsync::State::Idle));

    let next = client.request_next().await.unwrap();

    assert!(matches!(next, NextResponse::RollBackward(..)));

    let mut await_count = 0;

    for _ in 0..4 {
        let next = if client.has_agency() {
            client.request_next().await.unwrap()
        } else {
            await_count += 1;
            client.recv_while_must_reply().await.unwrap()
        };

        match next {
            NextResponse::RollForward(_, _) => (),
            NextResponse::Await => (),
            _ => panic!("expected roll-forward or await"),
        }
    }

    assert!(await_count > 0, "tip was never reached");

    client.send_done().await.unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[tokio::test]
#[ignore]
pub async fn blockfetch_happy_path() {
    let mut peer = PeerClient::connect("preview-node.world.dev.cardano.org:30002", 2)
        .await
        .unwrap();

    let client = peer.blockfetch();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let range_ok = client
        .request_range((known_point.clone(), known_point))
        .await;

    assert!(matches!(client.state(), blockfetch::State::Streaming));

    println!("streaming...");

    assert!(matches!(range_ok, Ok(_)));

    for _ in 0..1 {
        let next = client.recv_while_streaming().await.unwrap();

        match next {
            Some(body) => assert_eq!(body.len(), 3251),
            _ => panic!("expected block body"),
        }

        assert!(matches!(client.state(), blockfetch::State::Streaming));
    }

    let next = client.recv_while_streaming().await.unwrap();

    assert!(matches!(next, None));

    client.send_done().await.unwrap();

    assert!(matches!(client.state(), blockfetch::State::Done));
}

#[tokio::test]
#[ignore]
pub async fn blockfetch_server_and_client_happy_path() {
    let server = tokio::spawn(async move {
        // server setup

        let block_bodies = vec![
            hex::decode("deadbeefdeadbeef").unwrap(),
            hex::decode("c0ffeec0ffeec0ffee").unwrap(),
        ];

        let server_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30001))
            .await
            .unwrap();

        let (bearer, _) = Bearer::accept_tcp(server_listener).await.unwrap();

        let mut server_plexer = Plexer::new(bearer);

        let mut server_hs: handshake::Server<VersionData> =
            handshake::Server::new(server_plexer.subscribe_server(0));
        let mut server_bf = blockfetch::Server::new(server_plexer.subscribe_server(3));

        tokio::spawn(async move { server_plexer.run().await });

        server_hs.receive_proposed_versions().await.unwrap();
        server_hs
            .accept_version(10, VersionData::new(0, false))
            .await
            .unwrap();

        // server receives range from client, sends blocks

        let BlockRequest(range_request) = server_bf.recv_while_idle().await.unwrap().unwrap();

        assert_eq!(*server_bf.state(), blockfetch::State::Busy);

        println!("server received range request: {range_request:?}");
        println!("server responding with {} blocks", block_bodies.len());

        server_bf.send_block_range(block_bodies).await.unwrap();

        assert_eq!(*server_bf.state(), blockfetch::State::Idle);

        // server receives range from client, sends NoBlocks

        let BlockRequest(range_request) = server_bf.recv_while_idle().await.unwrap().unwrap();

        println!("server received range request: {range_request:?}");
        println!("server responding with no blocks (NoBlocks message)");

        server_bf.send_block_range(vec![]).await.unwrap();

        assert_eq!(*server_bf.state(), blockfetch::State::Idle);

        println!(
            "server received: {:?}",
            server_bf.recv_while_idle().await.unwrap()
        );

        assert_eq!(*server_bf.state(), blockfetch::State::Done);
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // client setup

        let mut client_to_server_conn = PeerClient::connect("localhost:30001", 0).await.unwrap();

        let client_bf = client_to_server_conn.blockfetch();

        // client sends request range

        let point = Point::Specific(
            1337,
            hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef")
                .unwrap(),
        );

        println!(
            "client sending request range: {:?}",
            (point.clone(), point.clone())
        );

        client_bf
            .send_request_range((point.clone(), point.clone()))
            .await
            .unwrap();

        println!(
            "client received: {:?}, now in {:?}",
            client_bf.recv_while_busy().await.unwrap(),
            client_bf.state()
        );

        // client receives blocks until idle

        while let Some(received_body) = client_bf.recv_while_streaming().await.unwrap() {
            println!("client received body: {}", hex::encode(received_body))
        }

        // client sends request range

        println!(
            "client sending request range: {:?}",
            (point.clone(), point.clone())
        );

        client_bf
            .send_request_range((point.clone(), point.clone()))
            .await
            .unwrap();

        // client sends done

        println!(
            "client received: {:?}",
            client_bf.recv_while_busy().await.unwrap()
        );

        client_bf.send_done().await.unwrap();
    });

    _ = tokio::join!(client, server);
}

// TODO: redo txsubmission client test
