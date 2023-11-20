use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use pallas_codec::utils::AnyCbor;
use pallas_network::facades::{NodeClient, PeerClient, PeerServer};
use pallas_network::miniprotocols::blockfetch::BlockRequest;
use pallas_network::miniprotocols::chainsync::{ClientRequest, HeaderContent, Tip};
use pallas_network::miniprotocols::handshake::n2n::VersionData;
use pallas_network::miniprotocols::localstate::ClientQueryRequest;
use pallas_network::miniprotocols::{
    blockfetch,
    chainsync::{self, NextResponse},
    Point,
};
use pallas_network::miniprotocols::{handshake, localstate};
use pallas_network::multiplexer::{Bearer, Plexer};
use std::path::Path;
use tokio::net::{TcpListener, UnixListener};

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

    println!("{point:?}");

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

    assert!(range_ok.is_ok());

    for _ in 0..1 {
        let next = client.recv_while_streaming().await.unwrap();

        match next {
            Some(body) => assert_eq!(body.len(), 3251),
            _ => panic!("expected block body"),
        }

        assert!(matches!(client.state(), blockfetch::State::Streaming));
    }

    let next = client.recv_while_streaming().await.unwrap();

    assert!(next.is_none());

    client.send_done().await.unwrap();

    assert!(matches!(client.state(), blockfetch::State::Done));
}

#[tokio::test]
#[ignore]
pub async fn blockfetch_server_and_client_happy_path() {
    let block_bodies = vec![
        hex::decode("deadbeefdeadbeef").unwrap(),
        hex::decode("c0ffeec0ffeec0ffee").unwrap(),
    ];

    let point = Point::Specific(
        1337,
        hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap(),
    );

    let server = tokio::spawn({
        let bodies = block_bodies.clone();
        let point = point.clone();
        async move {
            // server setup

            let server_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30001))
                .await
                .unwrap();

            let mut peer_server = PeerServer::accept(&server_listener, 0).await.unwrap();

            let server_bf = peer_server.blockfetch();

            // server receives range from client, sends blocks

            let BlockRequest(range_request) = server_bf.recv_while_idle().await.unwrap().unwrap();

            assert_eq!(range_request, (point.clone(), point.clone()));
            assert_eq!(*server_bf.state(), blockfetch::State::Busy);

            server_bf.send_block_range(bodies).await.unwrap();

            assert_eq!(*server_bf.state(), blockfetch::State::Idle);

            // server receives range from client, sends NoBlocks

            let BlockRequest(_) = server_bf.recv_while_idle().await.unwrap().unwrap();

            server_bf.send_block_range(vec![]).await.unwrap();

            assert_eq!(*server_bf.state(), blockfetch::State::Idle);

            assert!(server_bf.recv_while_idle().await.unwrap().is_none());

            assert_eq!(*server_bf.state(), blockfetch::State::Done);
        }
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // client setup

        let mut client_to_server_conn = PeerClient::connect("localhost:30001", 0).await.unwrap();

        let client_bf = client_to_server_conn.blockfetch();

        // client sends request range

        client_bf
            .send_request_range((point.clone(), point.clone()))
            .await
            .unwrap();

        assert!(client_bf.recv_while_busy().await.unwrap().is_some());

        // client receives blocks until idle

        let mut received_bodies = Vec::new();

        while let Some(received_body) = client_bf.recv_while_streaming().await.unwrap() {
            received_bodies.push(received_body)
        }

        assert_eq!(received_bodies, block_bodies);

        // client sends request range

        client_bf
            .send_request_range((point.clone(), point.clone()))
            .await
            .unwrap();

        // recv_while_busy returns None for NoBlocks message
        assert!(client_bf.recv_while_busy().await.unwrap().is_none());

        // client sends done

        client_bf.send_done().await.unwrap();
    });

    _ = tokio::join!(client, server);
}

#[tokio::test]
#[ignore]
pub async fn chainsync_server_and_client_happy_path_n2n() {
    let point1 = Point::Specific(1, vec![0x01]);
    let point2 = Point::Specific(2, vec![0x02]);

    let server = tokio::spawn({
        let point1 = point1.clone();
        let point2 = point2.clone();
        async move {
            // server setup

            let server_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30001))
                .await
                .unwrap();

            let (bearer, _) = Bearer::accept_tcp(&server_listener).await.unwrap();

            let mut server_plexer = Plexer::new(bearer);

            let mut server_hs: handshake::Server<VersionData> =
                handshake::Server::new(server_plexer.subscribe_server(0));
            let mut server_cs = chainsync::N2NServer::new(server_plexer.subscribe_server(2));

            tokio::spawn(async move { server_plexer.run().await });

            server_hs.receive_proposed_versions().await.unwrap();
            server_hs
                .accept_version(10, VersionData::new(0, false))
                .await
                .unwrap();

            // server receives find intersect from client, sends intersect point

            assert_eq!(*server_cs.state(), chainsync::State::Idle);

            let intersect_points = match server_cs.recv_while_idle().await.unwrap().unwrap() {
                ClientRequest::Intersect(points) => points,
                ClientRequest::RequestNext => panic!("unexpected message"),
            };

            assert_eq!(*server_cs.state(), chainsync::State::Intersect);
            assert_eq!(intersect_points, vec![point2.clone(), point1.clone()]);

            server_cs
                .send_intersect_found(point2.clone(), Tip(point2.clone(), 1337))
                .await
                .unwrap();

            assert_eq!(*server_cs.state(), chainsync::State::Idle);

            // server receives request next from client, sends rollbackwards

            match server_cs.recv_while_idle().await.unwrap().unwrap() {
                ClientRequest::RequestNext => (),
                ClientRequest::Intersect(_) => panic!("unexpected message"),
            };

            assert_eq!(*server_cs.state(), chainsync::State::CanAwait);

            server_cs
                .send_roll_backward(point2.clone(), Tip(point2.clone(), 1337))
                .await
                .unwrap();

            assert_eq!(*server_cs.state(), chainsync::State::Idle);

            // server receives request next from client, sends rollforwards

            match server_cs.recv_while_idle().await.unwrap().unwrap() {
                ClientRequest::RequestNext => (),
                ClientRequest::Intersect(_) => panic!("unexpected message"),
            };

            assert_eq!(*server_cs.state(), chainsync::State::CanAwait);

            let header2 = HeaderContent {
                variant: 1,
                byron_prefix: None,
                cbor: hex::decode("c0ffeec0ffeec0ffee").unwrap(),
            };

            server_cs
                .send_roll_forward(header2, Tip(point2.clone(), 1337))
                .await
                .unwrap();

            assert_eq!(*server_cs.state(), chainsync::State::Idle);

            // server receives request next from client, sends await reply
            // then rollforwards

            match server_cs.recv_while_idle().await.unwrap().unwrap() {
                ClientRequest::RequestNext => (),
                ClientRequest::Intersect(_) => panic!("unexpected message"),
            };

            assert_eq!(*server_cs.state(), chainsync::State::CanAwait);

            server_cs.send_await_reply().await.unwrap();

            assert_eq!(*server_cs.state(), chainsync::State::MustReply);

            let header1 = HeaderContent {
                variant: 1,
                byron_prefix: None,
                cbor: hex::decode("deadbeefdeadbeef").unwrap(),
            };

            server_cs
                .send_roll_forward(header1, Tip(point1.clone(), 123))
                .await
                .unwrap();

            assert_eq!(*server_cs.state(), chainsync::State::Idle);

            // server receives client done

            assert!(server_cs.recv_while_idle().await.unwrap().is_none());
            assert_eq!(*server_cs.state(), chainsync::State::Done);
        }
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;

        // client setup

        let mut client_to_server_conn = PeerClient::connect("localhost:30001", 0).await.unwrap();

        let client_cs = client_to_server_conn.chainsync();

        // client sends find intersect

        let intersect_response = client_cs
            .find_intersect(vec![point2.clone(), point1.clone()])
            .await
            .unwrap();

        assert_eq!(intersect_response.0, Some(point2.clone()));
        assert_eq!(intersect_response.1 .0, point2.clone());
        assert_eq!(intersect_response.1 .1, 1337);

        // client sends msg request next

        client_cs.send_request_next().await.unwrap();

        // client receives rollback

        match client_cs.recv_while_can_await().await.unwrap() {
            NextResponse::RollBackward(point, tip) => {
                assert_eq!(point, point2.clone());
                assert_eq!(tip.0, point2.clone());
                assert_eq!(tip.1, 1337);
            }
            _ => panic!("unexpected response"),
        }

        client_cs.send_request_next().await.unwrap();

        // client receives roll forward

        match client_cs.recv_while_can_await().await.unwrap() {
            NextResponse::RollForward(content, tip) => {
                assert_eq!(content.cbor, hex::decode("c0ffeec0ffeec0ffee").unwrap());
                assert_eq!(tip.0, point2.clone());
                assert_eq!(tip.1, 1337);
            }
            _ => panic!("unexpected response"),
        }

        // client sends msg request next

        client_cs.send_request_next().await.unwrap();

        // client receives await

        match client_cs.recv_while_can_await().await.unwrap() {
            NextResponse::Await => (),
            _ => panic!("unexpected response"),
        }

        match client_cs.recv_while_must_reply().await.unwrap() {
            NextResponse::RollForward(content, tip) => {
                assert_eq!(content.cbor, hex::decode("deadbeefdeadbeef").unwrap());
                assert_eq!(tip.0, point1.clone());
                assert_eq!(tip.1, 123);
            }
            _ => panic!("unexpected response"),
        }

        // client sends done

        client_cs.send_done().await.unwrap();
    });

    _ = tokio::join!(client, server);
}

#[tokio::test]
#[ignore]
pub async fn local_state_query_server_and_client_happy_path() {
    let server = tokio::spawn({
        async move {
            // server setup
            let socket_path = Path::new("node.socket");

            if socket_path.exists() {
                fs::remove_file(&socket_path).unwrap();
            }

            let unix_listener = UnixListener::bind(socket_path).unwrap();

            let mut server = pallas_network::facades::NodeServer::accept(&unix_listener, 0)
                .await
                .unwrap();

            // wait for acquire request from client

            let maybe_acquire = server.statequery().recv_while_idle().await.unwrap();

            assert!(maybe_acquire.is_some());
            assert_eq!(*server.statequery().state(), localstate::State::Acquiring);

            server.statequery().send_acquired().await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

            // server receives query from client

            let query: localstate::queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!("unexpected message from client: {x:?}"),
                };

            assert_eq!(query, localstate::queries_v16::Request::GetSystemStart);
            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let result = AnyCbor::from_encode(localstate::queries_v16::SystemStart {
                year: 2020,
                day_of_year: 1,
                picoseconds_of_day: 999999999,
            });

            server.statequery().send_result(result).await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

            // server receives re-acquire from the client

            let maybe_point = match server.statequery().recv_while_acquired().await.unwrap() {
                ClientQueryRequest::ReAcquire(p) => p,
                x => panic!("unexpected message from client: {x:?}"),
            };

            assert_eq!(maybe_point, Some(Point::Specific(1337, vec![1, 2, 3])));
            assert_eq!(*server.statequery().state(), localstate::State::Acquiring);

            server.statequery().send_acquired().await.unwrap();

            // server receives release from the client

            match server.statequery().recv_while_acquired().await.unwrap() {
                ClientQueryRequest::Release => (),
                x => panic!("unexpected message from client: {x:?}"),
            };

            let next_request = server.statequery().recv_while_idle().await.unwrap();

            assert!(next_request.is_none());
            assert_eq!(*server.statequery().state(), localstate::State::Done);
        }
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // client setup

        let socket_path = "node.socket";

        let mut client = NodeClient::connect(socket_path, 0).await.unwrap();

        // client sends acquire

        client
            .statequery()
            .send_acquire(Some(Point::Origin))
            .await
            .unwrap();

        client.statequery().recv_while_acquiring().await.unwrap();

        assert_eq!(*client.statequery().state(), localstate::State::Acquired);

        // client sends a BlockQuery

        let request = AnyCbor::from_encode(localstate::queries_v16::Request::GetSystemStart);

        client.statequery().send_query(request).await.unwrap();

        let result: localstate::queries_v16::SystemStart = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        assert_eq!(
            result,
            localstate::queries_v16::SystemStart {
                year: 2020,
                day_of_year: 1,
                picoseconds_of_day: 999999999,
            }
        );

        // client sends a ReAquire

        client
            .statequery()
            .send_reacquire(Some(Point::Specific(1337, vec![1, 2, 3])))
            .await
            .unwrap();

        client.statequery().recv_while_acquiring().await.unwrap();

        client.statequery().send_release().await.unwrap();

        client.statequery().send_done().await.unwrap();
    });

    _ = tokio::join!(client, server);
}
