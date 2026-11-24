//! Integration tests for pipelined ChainSync (`Client` primitives and `Pipeline`).

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use pallas_network::miniprotocols::chainsync::{
    self, ClientError, ClientRequest, DEFAULT_PIPELINE_WINDOW, HeaderContent, NextResponse,
    Pipeline, State, Tip,
};
use pallas_network::miniprotocols::{
    PROTOCOL_N2N_CHAIN_SYNC, PROTOCOL_N2N_KEEP_ALIVE, Point, keepalive,
};
use pallas_network::multiplexer::{AgentChannel, Bearer, Plexer, RunningPlexer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct Link {
    _client: RunningPlexer,
    _server: RunningPlexer,
}

async fn loopback_chainsync() -> (chainsync::N2NClient, chainsync::N2NServer, Link) {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let (initiator, responder) =
        tokio::join!(Bearer::connect_tcp(addr), Bearer::accept_tcp(&listener));
    let mut client_plexer = Plexer::new(initiator.unwrap());
    let mut server_plexer = Plexer::new(responder.unwrap().0);

    let client_ch = client_plexer.subscribe_client(PROTOCOL_N2N_CHAIN_SYNC);
    let server_ch = server_plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC);

    let link = Link {
        _client: client_plexer.spawn(),
        _server: server_plexer.spawn(),
    };

    (
        chainsync::N2NClient::new(client_ch),
        chainsync::N2NServer::new(server_ch),
        link,
    )
}

async fn loopback_with_keepalive() -> (
    chainsync::N2NClient,
    keepalive::Client,
    chainsync::N2NServer,
    keepalive::Server,
    Link,
) {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let (initiator, responder) =
        tokio::join!(Bearer::connect_tcp(addr), Bearer::accept_tcp(&listener));
    let mut client_plexer = Plexer::new(initiator.unwrap());
    let mut server_plexer = Plexer::new(responder.unwrap().0);

    let cs_client =
        chainsync::N2NClient::new(client_plexer.subscribe_client(PROTOCOL_N2N_CHAIN_SYNC));
    let ka_client = keepalive::Client::new(client_plexer.subscribe_client(PROTOCOL_N2N_KEEP_ALIVE));
    let cs_server =
        chainsync::N2NServer::new(server_plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC));
    let ka_server = keepalive::Server::new(server_plexer.subscribe_server(PROTOCOL_N2N_KEEP_ALIVE));

    let link = Link {
        _client: client_plexer.spawn(),
        _server: server_plexer.spawn(),
    };

    (cs_client, ka_client, cs_server, ka_server, link)
}

fn point_for(height: u64) -> Point {
    if height == 0 {
        Point::Origin
    } else {
        let mut hash = vec![0u8; 32];
        hash[..8].copy_from_slice(&height.to_be_bytes());
        Point::Specific(height * 20, hash)
    }
}

fn header_for(height: u64) -> HeaderContent {
    let mut cbor = vec![0u8; 1024];
    cbor[..8].copy_from_slice(&height.to_be_bytes());
    HeaderContent {
        variant: 6,
        byron_prefix: None,
        cbor,
    }
}

fn height_of(header: &HeaderContent) -> u64 {
    u64::from_be_bytes(header.cbor[..8].try_into().unwrap())
}

/// Spawns a single-connection TCP proxy that injects a FIFO one-way propagation
/// delay of `rtt / 2` in each direction without throttling streaming bandwidth.
async fn spawn_delay_proxy(
    upstream: SocketAddr,
    rtt: Duration,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let local_addr = listener.local_addr().unwrap();
    let one_way = rtt / 2;

    let handle = tokio::spawn(async move {
        let Ok((client, _)) = listener.accept().await else {
            return;
        };
        let _ = client.set_nodelay(true);
        let Ok(server) = TcpStream::connect(upstream).await else {
            return;
        };
        let _ = server.set_nodelay(true);

        let (c_read, c_write) = client.into_split();
        let (s_read, s_write) = server.into_split();

        async fn forward(
            mut reader: tokio::net::tcp::OwnedReadHalf,
            mut writer: tokio::net::tcp::OwnedWriteHalf,
            delay: Duration,
        ) {
            let (tx, mut rx) =
                tokio::sync::mpsc::unbounded_channel::<(tokio::time::Instant, Vec<u8>)>();
            let reader_task = tokio::spawn(async move {
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let deliver_at = tokio::time::Instant::now() + delay;
                            if tx.send((deliver_at, buf[..n].to_vec())).is_err() {
                                break;
                            }
                        }
                    }
                }
            });

            while let Some((deliver_at, data)) = rx.recv().await {
                if !delay.is_zero() {
                    tokio::time::sleep_until(deliver_at).await;
                }
                if writer.write_all(&data).await.is_err() {
                    break;
                }
            }
            let _ = writer.shutdown().await;
            reader_task.abort();
        }

        let _ = tokio::join!(
            forward(c_read, s_write, one_way),
            forward(s_read, c_write, one_way),
        );
    });

    (local_addr, handle)
}

#[tokio::test]
async fn legacy_send_request_next_still_rejects_double_send_and_blocks_done_while_in_flight() {
    let (mut client, mut server, _link) = loopback_chainsync().await;

    assert_eq!(client.in_flight(), 0);
    assert!(client.can_pipeline_request_next());

    // Non-pipelined send_request_next must preserve v1 AgencyIsTheirs on double send.
    client.send_request_next().await.unwrap();
    assert_eq!(client.in_flight(), 1);
    assert_eq!(*client.state(), State::CanAwait);
    assert!(matches!(
        client.send_request_next().await,
        Err(ClientError::AgencyIsTheirs)
    ));

    // Pipelined send IS allowed in CanAwait, and send_request_next_n(0) is a no-op.
    client.send_request_next_n(0).await.unwrap();
    assert_eq!(client.in_flight(), 1);

    client.send_request_next_pipelined().await.unwrap();
    assert_eq!(client.in_flight(), 2);

    // Done and FindIntersect are rejected while requests are in flight.
    assert!(matches!(
        client.send_done().await,
        Err(ClientError::AgencyIsTheirs)
    ));
    assert!(matches!(
        client.send_find_intersect(vec![Point::Origin]).await,
        Err(ClientError::AgencyIsTheirs)
    ));

    let server_task = tokio::spawn(async move {
        for h in 1..=2 {
            assert!(matches!(
                server.recv_while_idle().await.unwrap(),
                Some(ClientRequest::RequestNext)
            ));
            server
                .send_roll_forward(header_for(h), Tip(point_for(2), 2))
                .await
                .unwrap();
        }
        assert!(server.recv_while_idle().await.unwrap().is_none());
    });

    assert!(matches!(
        client.recv_next().await.unwrap(),
        NextResponse::RollForward(_, _)
    ));
    assert_eq!(client.in_flight(), 1);
    assert_eq!(*client.state(), State::CanAwait);

    assert!(matches!(
        client.recv_next().await.unwrap(),
        NextResponse::RollForward(_, _)
    ));
    assert_eq!(client.in_flight(), 0);
    assert_eq!(*client.state(), State::Idle);

    // Calling recv_next in Idle fails with AgencyIsOurs.
    assert!(matches!(
        client.recv_next().await,
        Err(ClientError::AgencyIsOurs)
    ));

    client.send_done().await.unwrap();
    assert!(client.is_done());
    assert!(!client.can_pipeline_request_next());
    server_task.await.unwrap();
}

#[tokio::test]
async fn stop_at_tip_probes_once_pipelines_window_and_drains_without_overshoot() {
    const TIP_HEIGHT: u64 = 250;
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let requests_received = Arc::new(AtomicUsize::new(0));
    let requests_for_server = requests_received.clone();

    let server_task = tokio::spawn(async move {
        let Some(ClientRequest::Intersect(_)) = server.recv_while_idle().await.unwrap() else {
            panic!("expected FindIntersect");
        };
        let tip = Tip(point_for(TIP_HEIGHT), TIP_HEIGHT);
        server
            .send_intersect_found(Point::Origin, tip.clone())
            .await
            .unwrap();

        // First RequestNext -> RollBackward(Origin, tip)
        let Some(ClientRequest::RequestNext) = server.recv_while_idle().await.unwrap() else {
            panic!("expected initial RequestNext");
        };
        requests_for_server.fetch_add(1, Ordering::SeqCst);
        server
            .send_roll_backward(Point::Origin, tip.clone())
            .await
            .unwrap();

        for h in 1..=TIP_HEIGHT {
            let Some(ClientRequest::RequestNext) = server.recv_while_idle().await.unwrap() else {
                panic!("expected RequestNext for block {h}");
            };
            requests_for_server.fetch_add(1, Ordering::SeqCst);
            server
                .send_roll_forward(header_for(h), tip.clone())
                .await
                .unwrap();
        }

        // Next message MUST be Done, with zero extra RequestNext past tip.1.
        assert!(
            server.recv_while_idle().await.unwrap().is_none(),
            "expected MsgDone immediately at tip with zero extra RequestNext"
        );
    });

    let (_, tip) = client.find_intersect(vec![Point::Origin]).await.unwrap();
    let mut pipeline = Pipeline::stop_at_tip(DEFAULT_PIPELINE_WINDOW);
    pipeline.reset_after_intersect(&tip);

    let mut received_heights = Vec::new();
    let mut max_in_flight = 0usize;

    // Probe #1: RollBackward(Origin) -> in_flight must be 0 after receive.
    let first = pipeline.next(&mut client).await.unwrap().unwrap();
    assert!(matches!(
        first,
        NextResponse::RollBackward(Point::Origin, _)
    ));
    assert_eq!(client.in_flight(), 0);

    // Probe #2: first RollForward(1) -> still single probe until height is reported.
    let second = pipeline.next(&mut client).await.unwrap().unwrap();
    let NextResponse::RollForward(h1, _) = second else {
        panic!("expected RollForward(1)");
    };
    assert_eq!(height_of(&h1), 1);
    assert_eq!(client.in_flight(), 0);
    received_heights.push(1);
    pipeline.set_local_height(1);

    // Remaining blocks 2..=TIP_HEIGHT should pipeline up to 64 and taper to 0.
    while let Some(next) = pipeline.next(&mut client).await.unwrap() {
        // After recv_next, in_flight was `before - 1`, so peak was `in_flight + 1`.
        max_in_flight = max_in_flight.max(client.in_flight() + 1);
        match next {
            NextResponse::RollForward(h, _) => {
                let height = height_of(&h);
                received_heights.push(height);
                pipeline.set_local_height(height);
            }
            other => panic!("unexpected response during catch-up: {other:?}"),
        }
    }

    assert_eq!(client.in_flight(), 0);
    assert_eq!(*client.state(), State::Idle);
    assert_eq!(max_in_flight, DEFAULT_PIPELINE_WINDOW);
    assert_eq!(received_heights, (1..=TIP_HEIGHT).collect::<Vec<_>>());

    client.send_done().await.unwrap();
    server_task.await.unwrap();

    // 1 for initial RollBackward + TIP_HEIGHT for headers 1..=TIP_HEIGHT.
    assert_eq!(
        requests_received.load(Ordering::SeqCst),
        (TIP_HEIGHT as usize) + 1
    );
}

#[tokio::test]
async fn stop_at_tip_already_synced_sends_zero_extra_requests() {
    const TIP_HEIGHT: u64 = 500;
    let tip_point = point_for(TIP_HEIGHT);
    let tip = Tip(tip_point.clone(), TIP_HEIGHT);
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let server_task = tokio::spawn({
        let tip_point = tip_point.clone();
        let tip = tip.clone();
        async move {
            let Some(ClientRequest::Intersect(_)) = server.recv_while_idle().await.unwrap() else {
                panic!("expected FindIntersect");
            };
            server
                .send_intersect_found(tip_point.clone(), tip.clone())
                .await
                .unwrap();

            let Some(ClientRequest::RequestNext) = server.recv_while_idle().await.unwrap() else {
                panic!("expected single probe RequestNext");
            };
            server.send_roll_backward(tip_point, tip).await.unwrap();

            assert!(
                server.recv_while_idle().await.unwrap().is_none(),
                "expected Done immediately without triggering AwaitReply"
            );
        }
    });

    let (_, tip) = client
        .find_intersect(vec![tip_point.clone()])
        .await
        .unwrap();
    let mut pipeline = Pipeline::stop_at_tip(DEFAULT_PIPELINE_WINDOW);
    pipeline.reset_after_intersect(&tip);

    let first = pipeline.next(&mut client).await.unwrap().unwrap();
    assert!(matches!(first, NextResponse::RollBackward(p, _) if p == tip_point));
    assert!(pipeline.next(&mut client).await.unwrap().is_none());
    assert_eq!(client.in_flight(), 0);
    assert_eq!(*client.state(), State::Idle);

    client.send_done().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test]
async fn stop_at_tip_drains_cleanly_when_peer_tip_shrinks_mid_flight() {
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let server_task = tokio::spawn(async move {
        let Some(ClientRequest::Intersect(_)) = server.recv_while_idle().await.unwrap() else {
            panic!("expected FindIntersect");
        };
        server
            .send_intersect_found(Point::Origin, Tip(point_for(100), 100))
            .await
            .unwrap();

        let Some(ClientRequest::RequestNext) = server.recv_while_idle().await.unwrap() else {
            panic!("expected probe");
        };
        server
            .send_roll_backward(Point::Origin, Tip(point_for(100), 100))
            .await
            .unwrap();

        // Block 1 establishes height 1 -> client sends 64 requests (for blocks 2..=65).
        // Blocks 2..=9 (8 eager refills) put requests for blocks 66..=73 on the wire.
        // At block 10, peer shrinks advertised tip from 100 -> 73, leaving 63 remaining
        // (matching the 63 already in flight for 11..=73). Client must send ZERO further
        // RequestNext messages and send Done immediately after draining block 73.
        for h in 1..=73u64 {
            let Some(ClientRequest::RequestNext) = server.recv_while_idle().await.unwrap() else {
                panic!("expected RequestNext {h}");
            };
            let advertised = if h < 10 { 100 } else { 73 };
            server
                .send_roll_forward(header_for(h), Tip(point_for(advertised), advertised))
                .await
                .unwrap();
        }

        assert!(
            server.recv_while_idle().await.unwrap().is_none(),
            "expected Done right after draining to shrunken tip 73"
        );
    });

    let (_, tip) = client.find_intersect(vec![Point::Origin]).await.unwrap();
    let mut pipeline = Pipeline::stop_at_tip(DEFAULT_PIPELINE_WINDOW);
    pipeline.reset_after_intersect(&tip);

    let mut last_height = 0;
    while let Some(next) = pipeline.next(&mut client).await.unwrap() {
        if let NextResponse::RollForward(h, _) = next {
            last_height = height_of(&h);
            pipeline.set_local_height(last_height);
        }
    }

    assert_eq!(last_height, 73);
    assert_eq!(client.in_flight(), 0);
    assert_eq!(*client.state(), State::Idle);
    client.send_done().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test]
async fn stop_at_tip_drains_when_tip_shrinks_below_in_flight() {
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let server_task = tokio::spawn(async move {
        let _ = server.recv_while_idle().await.unwrap().unwrap();
        server
            .send_intersect_found(Point::Origin, Tip(point_for(50), 50))
            .await
            .unwrap();
        let _ = server.recv_while_idle().await.unwrap().unwrap();
        server
            .send_roll_backward(Point::Origin, Tip(point_for(50), 50))
            .await
            .unwrap();

        // Block 1 establishes height 1 -> client pipelines 16 requests (for 2..=17).
        let _ = server.recv_while_idle().await.unwrap().unwrap();
        server
            .send_roll_forward(header_for(1), Tip(point_for(50), 50))
            .await
            .unwrap();

        // Immediately at block 2, peer advertises shrunken tip 10 (strictly below 17
        // already in flight). Client must send ZERO additional RequestNext messages,
        // and drain all 16 already-in-flight replies down to 0 before sending Done.
        for h in 2..=10u64 {
            let _ = server.recv_while_idle().await.unwrap().unwrap();
            server
                .send_roll_forward(header_for(h), Tip(point_for(10), 10))
                .await
                .unwrap();
        }
        // Remaining 7 already-in-flight requests receive a tip-aligned RollBackward(10, tip=10)
        // (or replacement headers after rollback) until the pipeline drains to 0.
        for _ in 11..=17u64 {
            let _ = server.recv_while_idle().await.unwrap().unwrap();
            server
                .send_roll_backward(point_for(10), Tip(point_for(10), 10))
                .await
                .unwrap();
        }

        assert!(
            server.recv_while_idle().await.unwrap().is_none(),
            "expected Done immediately after draining all 16 in-flight replies"
        );
    });

    let (_, tip) = client.find_intersect(vec![Point::Origin]).await.unwrap();
    let mut pipeline = Pipeline::stop_at_tip(16);
    pipeline.reset_after_intersect(&tip);

    let mut roll_forwards = 0usize;
    let mut roll_backwards = 0usize;
    while let Some(next) = pipeline.next(&mut client).await.unwrap() {
        match next {
            NextResponse::RollForward(h, _) => {
                roll_forwards += 1;
                pipeline.set_local_height(height_of(&h));
            }
            NextResponse::RollBackward(_, _) => {
                roll_backwards += 1;
            }
            NextResponse::Await => panic!("unexpected Await"),
        }
    }

    assert_eq!(roll_forwards, 10);
    assert_eq!(roll_backwards, 8); // 1 initial + 7 drained
    assert_eq!(client.in_flight(), 0);
    assert_eq!(*client.state(), State::Idle);
    client.send_done().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test]
async fn await_reply_holds_new_requests_until_pending_request_completes() {
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let server_task = tokio::spawn(async move {
        // Expect 2 pipelined requests from the client.
        assert!(matches!(
            server.recv_while_idle().await.unwrap(),
            Some(ClientRequest::RequestNext)
        ));
        server.send_await_reply().await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        server
            .send_roll_forward(header_for(10), Tip(point_for(10), 10))
            .await
            .unwrap();

        assert!(matches!(
            server.recv_while_idle().await.unwrap(),
            Some(ClientRequest::RequestNext)
        ));
        server.send_await_reply().await.unwrap();
        server
            .send_roll_forward(header_for(11), Tip(point_for(11), 11))
            .await
            .unwrap();

        assert!(server.recv_while_idle().await.unwrap().is_none());
    });

    client.send_request_next_n(2).await.unwrap();
    assert_eq!(client.in_flight(), 2);

    // First response is AwaitReply: in_flight MUST remain 2 and pipelining MUST be blocked.
    assert!(matches!(
        client.recv_next().await.unwrap(),
        NextResponse::Await
    ));
    assert_eq!(client.in_flight(), 2);
    assert_eq!(*client.state(), State::MustReply);
    assert!(!client.can_pipeline_request_next());
    assert!(matches!(
        client.send_request_next_pipelined().await,
        Err(ClientError::AgencyIsTheirs)
    ));

    // Completing request #1 transitions MustReply -> CanAwait (since request #2 is still in flight).
    let resp1 = client.recv_next().await.unwrap();
    assert!(matches!(resp1, NextResponse::RollForward(ref h, _) if height_of(h) == 10));
    assert_eq!(client.in_flight(), 1);
    assert_eq!(*client.state(), State::CanAwait);

    // Request #2 also receives AwaitReply -> MustReply, then RollForward -> Idle.
    assert!(matches!(
        client.recv_next().await.unwrap(),
        NextResponse::Await
    ));
    assert_eq!(client.in_flight(), 1);
    assert_eq!(*client.state(), State::MustReply);

    let resp2 = client.recv_next().await.unwrap();
    assert!(matches!(resp2, NextResponse::RollForward(ref h, _) if height_of(h) == 11));
    assert_eq!(client.in_flight(), 0);
    assert_eq!(*client.state(), State::Idle);

    client.send_done().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test]
async fn rollback_delivers_in_arrival_order_and_resets_to_probe_mode() {
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let server_task = tokio::spawn(async move {
        let Some(ClientRequest::Intersect(_)) = server.recv_while_idle().await.unwrap() else {
            panic!("expected FindIntersect");
        };
        server
            .send_intersect_found(Point::Origin, Tip(point_for(100), 100))
            .await
            .unwrap();

        // Probe #1: initial RollBackward(Origin)
        let _ = server.recv_while_idle().await.unwrap().unwrap();
        server
            .send_roll_backward(Point::Origin, Tip(point_for(100), 100))
            .await
            .unwrap();

        // Blocks 1..=5 on fork A
        for h in 1..=5u64 {
            let _ = server.recv_while_idle().await.unwrap().unwrap();
            server
                .send_roll_forward(header_for(h), Tip(point_for(100), 100))
                .await
                .unwrap();
        }

        // Rollback to block 3 while pipeline requests are in flight
        let _ = server.recv_while_idle().await.unwrap().unwrap();
        server
            .send_roll_backward(point_for(3), Tip(point_for(20), 20))
            .await
            .unwrap();

        // Replacement headers 4..=20 on fork B
        for h in 4..=20u64 {
            let _ = server.recv_while_idle().await.unwrap().unwrap();
            server
                .send_roll_forward(header_for(h), Tip(point_for(20), 20))
                .await
                .unwrap();
        }

        assert!(server.recv_while_idle().await.unwrap().is_none());
    });

    let (_, tip) = client.find_intersect(vec![Point::Origin]).await.unwrap();
    let mut pipeline = Pipeline::stop_at_tip(16);
    pipeline.reset_after_intersect(&tip);

    let mut events = Vec::new();
    while let Some(next) = pipeline.next(&mut client).await.unwrap() {
        match next {
            NextResponse::RollForward(h, _) => {
                let height = height_of(&h);
                events.push(format!("F:{height}"));
                pipeline.set_local_height(height);
            }
            NextResponse::RollBackward(p, _) => {
                let slot = p.slot_or_default();
                events.push(format!("B:{slot}"));
                // After rollback to non-tip point 3, local_height must be cleared.
                if slot > 0 {
                    assert_eq!(pipeline.local_height(), None);
                }
            }
            NextResponse::Await => panic!("unexpected Await"),
        }
    }

    // Verify RollBackward(slot 60 = block 3) arrived strictly after F:5 and before F:4 replacement.
    let rb_idx = events.iter().position(|e| e == "B:60").unwrap();
    assert_eq!(events[rb_idx - 1], "F:5");
    assert_eq!(events[rb_idx + 1], "F:4");
    assert_eq!(events.last().unwrap(), "F:20");

    client.send_done().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test]
async fn follow_tip_maintains_at_most_one_active_request_at_tip() {
    let tip_point = point_for(10);
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let server_task = tokio::spawn({
        let tip_point = tip_point.clone();
        async move {
            let Some(ClientRequest::Intersect(_)) = server.recv_while_idle().await.unwrap() else {
                panic!("expected FindIntersect");
            };
            server
                .send_intersect_found(tip_point.clone(), Tip(tip_point.clone(), 10))
                .await
                .unwrap();

            // Initial rollback to tip
            let _ = server.recv_while_idle().await.unwrap().unwrap();
            server
                .send_roll_backward(tip_point, Tip(point_for(10), 10))
                .await
                .unwrap();

            // Two live blocks each preceded by AwaitReply
            for h in 11..=12u64 {
                let _ = server.recv_while_idle().await.unwrap().unwrap();
                server.send_await_reply().await.unwrap();
                server
                    .send_roll_forward(header_for(h), Tip(point_for(h), h))
                    .await
                    .unwrap();
            }

            assert!(server.recv_while_idle().await.unwrap().is_none());
        }
    });

    let (_, tip) = client.find_intersect(vec![tip_point]).await.unwrap();
    let mut pipeline = Pipeline::follow_tip(DEFAULT_PIPELINE_WINDOW);
    pipeline.reset_after_intersect(&tip);

    let mut live_blocks = Vec::new();
    let mut awaits = 0usize;

    while live_blocks.len() < 2 {
        let next = pipeline.next(&mut client).await.unwrap().unwrap();
        assert!(
            client.in_flight() <= 1,
            "live tip following must keep at most 1 active request"
        );
        match next {
            NextResponse::RollBackward(_, _) => {}
            NextResponse::Await => {
                awaits += 1;
                assert_eq!(client.in_flight(), 1);
                assert_eq!(*client.state(), State::MustReply);
            }
            NextResponse::RollForward(h, _) => {
                let height = height_of(&h);
                live_blocks.push(height);
                pipeline.set_local_height(height);
                assert_eq!(client.in_flight(), 0);
            }
        }
    }

    assert_eq!(awaits, 2);
    assert_eq!(live_blocks, vec![11, 12]);
    client.send_done().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test]
async fn drain_next_flushes_active_pipeline_for_clean_early_done() {
    let (mut client, mut server, _link) = loopback_chainsync().await;

    let server_task = tokio::spawn(async move {
        let _ = server.recv_while_idle().await.unwrap().unwrap();
        server
            .send_intersect_found(Point::Origin, Tip(point_for(500), 500))
            .await
            .unwrap();
        let _ = server.recv_while_idle().await.unwrap().unwrap();
        server
            .send_roll_backward(Point::Origin, Tip(point_for(500), 500))
            .await
            .unwrap();

        let mut h = 1u64;
        while let Some(ClientRequest::RequestNext) = server.recv_while_idle().await.unwrap() {
            server
                .send_roll_forward(header_for(h), Tip(point_for(500), 500))
                .await
                .unwrap();
            h += 1;
        }
    });

    let (_, tip) = client.find_intersect(vec![Point::Origin]).await.unwrap();
    let mut pipeline = Pipeline::follow_tip(32);
    pipeline.reset_after_intersect(&tip);

    // Advance until pipeline is filled (in_flight > 10)
    while client.in_flight() < 10 {
        if let Some(NextResponse::RollForward(h, _)) = pipeline.next(&mut client).await.unwrap() {
            pipeline.set_local_height(height_of(&h));
        }
    }

    // Drain cleanly without sending any more RequestNext
    let mut drained = 0usize;
    while let Some(resp) = pipeline.drain_next(&mut client).await.unwrap() {
        assert!(matches!(resp, NextResponse::RollForward(_, _)));
        drained += 1;
    }

    assert!(drained >= 10);
    assert_eq!(client.in_flight(), 0);
    assert_eq!(*client.state(), State::Idle);
    client.send_done().await.unwrap();
    server_task.await.unwrap();
}

#[tokio::test]
async fn pipelining_64_requests_does_not_starve_concurrent_keepalive() {
    const BLOCKS: u64 = 500;
    let (mut cs_client, mut ka_client, mut cs_server, mut ka_server, _link) =
        loopback_with_keepalive().await;

    let ka_server_task =
        tokio::spawn(async move { while ka_server.keepalive_roundtrip().await.is_ok() {} });

    let cs_server_task = tokio::spawn(async move {
        let _ = cs_server.recv_while_idle().await.unwrap().unwrap();
        cs_server
            .send_intersect_found(Point::Origin, Tip(point_for(BLOCKS), BLOCKS))
            .await
            .unwrap();
        let _ = cs_server.recv_while_idle().await.unwrap().unwrap();
        cs_server
            .send_roll_backward(Point::Origin, Tip(point_for(BLOCKS), BLOCKS))
            .await
            .unwrap();
        for h in 1..=BLOCKS {
            let _ = cs_server.recv_while_idle().await.unwrap().unwrap();
            cs_server
                .send_roll_forward(header_for(h), Tip(point_for(BLOCKS), BLOCKS))
                .await
                .unwrap();
        }
        assert!(cs_server.recv_while_idle().await.unwrap().is_none());
    });

    let ka_client_task = tokio::spawn(async move {
        for _ in 0..5 {
            tokio::time::timeout(Duration::from_secs(2), ka_client.keepalive_roundtrip())
                .await
                .expect("keepalive must not stall while chainsync pipelines")
                .unwrap();
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });

    let (_, tip) = cs_client.find_intersect(vec![Point::Origin]).await.unwrap();
    let mut pipeline = Pipeline::stop_at_tip(DEFAULT_PIPELINE_WINDOW).with_min_refill(16);
    pipeline.reset_after_intersect(&tip);

    let mut count = 0u64;
    while let Some(next) = pipeline.next(&mut cs_client).await.unwrap() {
        if let NextResponse::RollForward(h, _) = next {
            let height = height_of(&h);
            count += 1;
            pipeline.set_local_height(height);
        }
    }

    assert_eq!(count, BLOCKS);
    cs_client.send_done().await.unwrap();
    ka_client_task.await.unwrap();
    cs_server_task.await.unwrap();
    ka_server_task.abort();
}

#[tokio::test]
async fn recv_next_preserves_n2n_message_size_limits() {
    let limit = chainsync::MESSAGE_SIZE_LIMIT;

    for disable in [false, true] {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (initiator, responder) =
            tokio::join!(Bearer::connect_tcp(addr), Bearer::accept_tcp(&listener));
        let mut client_plexer = Plexer::new(initiator.unwrap());
        let mut server_plexer = Plexer::new(responder.unwrap().0);
        let client_ch = client_plexer.subscribe_client(PROTOCOL_N2N_CHAIN_SYNC);
        let mut server_ch: AgentChannel = server_plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC);
        let _link = Link {
            _client: client_plexer.spawn(),
            _server: server_plexer.spawn(),
        };

        let mut client = chainsync::N2NClient::new(client_ch);
        if disable {
            client.disable_size_limits();
        }
        client.send_request_next_pipelined().await.unwrap();

        let oversized = chainsync::Message::RollForward(
            HeaderContent {
                variant: 6,
                byron_prefix: None,
                cbor: vec![0xaa; limit + 2_000],
            },
            Tip(point_for(1), 1),
        );
        let mut wire = Vec::new();
        pallas_codec::minicbor::encode(&oversized, &mut wire).unwrap();

        tokio::spawn(async move {
            for chunk in wire.chunks(pallas_network::multiplexer::MAX_SEGMENT_PAYLOAD_LENGTH) {
                if server_ch.enqueue_chunk(chunk.to_vec()).await.is_err() {
                    break;
                }
            }
        });

        let result = client.recv_next().await;
        if disable {
            assert!(
                result.is_ok(),
                "size limit opt-out must accept oversized header"
            );
        } else {
            let err = format!("{:?}", result.unwrap_err());
            assert!(
                err.contains(&format!("exceeds the size limit of {limit} bytes")),
                "unexpected error: {err}"
            );
        }
    }
}

#[tokio::test]
async fn simulated_wan_rtt_exceeds_stop_and_wait_ceiling() {
    for (rtt_ms, blocks) in [(5u64, 200u64), (20u64, 200u64)] {
        let rtt = Duration::from_millis(rtt_ms);
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (bearer, _) = Bearer::accept_tcp(&listener).await.unwrap();
            let mut plexer = Plexer::new(bearer);
            let mut server =
                chainsync::N2NServer::new(plexer.subscribe_server(PROTOCOL_N2N_CHAIN_SYNC));
            let _running = plexer.spawn();

            let _ = server.recv_while_idle().await.unwrap().unwrap();
            server
                .send_intersect_found(Point::Origin, Tip(point_for(blocks), blocks))
                .await
                .unwrap();
            let _ = server.recv_while_idle().await.unwrap().unwrap();
            server
                .send_roll_backward(Point::Origin, Tip(point_for(blocks), blocks))
                .await
                .unwrap();
            for h in 1..=blocks {
                let _ = server.recv_while_idle().await.unwrap().unwrap();
                server
                    .send_roll_forward(header_for(h), Tip(point_for(blocks), blocks))
                    .await
                    .unwrap();
            }
            assert!(server.recv_while_idle().await.unwrap().is_none());
        });

        let (proxy_addr, proxy_task) = spawn_delay_proxy(server_addr, rtt).await;
        let bearer = Bearer::connect_tcp(proxy_addr).await.unwrap();
        let mut plexer = Plexer::new(bearer);
        let mut client =
            chainsync::N2NClient::new(plexer.subscribe_client(PROTOCOL_N2N_CHAIN_SYNC));
        let _running = plexer.spawn();

        let (_, tip) = client.find_intersect(vec![Point::Origin]).await.unwrap();
        let mut pipeline = Pipeline::stop_at_tip(DEFAULT_PIPELINE_WINDOW).with_min_refill(16);
        pipeline.reset_after_intersect(&tip);

        let start = Instant::now();
        let mut max_in_flight = 0usize;
        let mut count = 0u64;

        while let Some(next) = pipeline.next(&mut client).await.unwrap() {
            max_in_flight = max_in_flight.max(client.in_flight() + 1);
            if let NextResponse::RollForward(h, _) = next {
                count += 1;
                pipeline.set_local_height(height_of(&h));
            }
        }
        let elapsed = start.elapsed();

        client.send_done().await.unwrap();
        server_task.await.unwrap();
        proxy_task.abort();

        let throughput = count as f64 / elapsed.as_secs_f64();
        let stop_and_wait_ceiling = 1000.0 / rtt_ms as f64;
        assert_eq!(count, blocks);
        assert!(
            max_in_flight >= 32,
            "expected pipelining at {rtt_ms}ms RTT, got max_in_flight={max_in_flight}"
        );
        assert!(
            throughput > stop_and_wait_ceiling * 4.0,
            "expected throughput >> 1/RTT ({stop_and_wait_ceiling:.1} hdr/s) at {rtt_ms}ms RTT, got {throughput:.1} hdr/s"
        );
    }
}
