use pallas_miniprotocols::{
    blockfetch,
    chainsync::{self, NextResponse},
    handshake::{self, Confirmation},
    txsubmission::{self, EraTxId, Reply, TxIdAndSize, EraTxBody, Server},
    Point, PROTOCOL_N2N_BLOCK_FETCH, PROTOCOL_N2N_CHAIN_SYNC, PROTOCOL_N2N_HANDSHAKE,
    PROTOCOL_N2N_TX_SUBMISSION,
};
use pallas_multiplexer::{bearers::Bearer, StdChannel, StdPlexer};

struct N2NChannels {
    chainsync: StdChannel,
    blockfetch: StdChannel,
    txsubmission: StdChannel,
}

fn setup_n2n_client_connection() -> N2NChannels {
    let bearer = Bearer::connect_tcp("preview-node.world.dev.cardano.org:30002").unwrap();
    let mut plexer = StdPlexer::new(bearer);

    let handshake = plexer.use_channel(PROTOCOL_N2N_HANDSHAKE);
    let chainsync = plexer.use_channel(PROTOCOL_N2N_CHAIN_SYNC);
    let blockfetch = plexer.use_channel(PROTOCOL_N2N_BLOCK_FETCH);
    let txsubmission = plexer.use_channel(PROTOCOL_N2N_TX_SUBMISSION);

    plexer.muxer.spawn();
    plexer.demuxer.spawn();

    let mut client = handshake::N2NClient::new(handshake);

    let confirmation = client
        .handshake(handshake::n2n::VersionTable::v7_and_above(2))
        .unwrap();

    assert!(matches!(confirmation, Confirmation::Accepted(..)));

    if let Confirmation::Accepted(v, _) = confirmation {
        assert!(v >= 7);
    }

    N2NChannels {
        chainsync,
        blockfetch,
        txsubmission,
    }
}

#[test]
#[ignore]
pub fn chainsync_history_happy_path() {
    let N2NChannels { chainsync, .. } = setup_n2n_client_connection();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let mut client = chainsync::N2NClient::new(chainsync);

    let (point, _) = client.find_intersect(vec![known_point.clone()]).unwrap();

    assert!(matches!(client.state(), chainsync::State::Idle));

    match point {
        Some(point) => assert_eq!(point, known_point.clone()),
        None => panic!("expected point"),
    }

    let next = client.request_next().unwrap();

    match next {
        NextResponse::RollBackward(point, _) => assert_eq!(point, known_point.clone()),
        _ => panic!("expected rollback"),
    }

    assert!(matches!(client.state(), chainsync::State::Idle));

    for _ in 0..10 {
        let next = client.request_next().unwrap();

        match next {
            NextResponse::RollForward(_, _) => (),
            _ => panic!("expected roll-forward"),
        }

        assert!(matches!(client.state(), chainsync::State::Idle));
    }

    client.send_done().unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[test]
#[ignore]
pub fn chainsync_tip_happy_path() {
    let N2NChannels { chainsync, .. } = setup_n2n_client_connection();

    let mut client = chainsync::N2NClient::new(chainsync);

    client.intersect_tip().unwrap();

    assert!(matches!(client.state(), chainsync::State::Idle));

    let next = client.request_next().unwrap();

    assert!(matches!(next, NextResponse::RollBackward(..)));

    let mut await_count = 0;

    for _ in 0..4 {
        let next = if client.has_agency() {
            client.request_next().unwrap()
        } else {
            await_count += 1;
            client.recv_while_must_reply().unwrap()
        };

        match next {
            NextResponse::RollForward(_, _) => (),
            NextResponse::Await => (),
            _ => panic!("expected roll-forward or await"),
        }
    }

    assert!(await_count > 0, "tip was never reached");

    client.send_done().unwrap();

    assert!(matches!(client.state(), chainsync::State::Done));
}

#[test]
#[ignore]
pub fn blockfetch_happy_path() {
    let N2NChannels { blockfetch, .. } = setup_n2n_client_connection();

    let known_point = Point::Specific(
        1654413,
        hex::decode("7de1f036df5a133ce68a82877d14354d0ba6de7625ab918e75f3e2ecb29771c2").unwrap(),
    );

    let mut client = blockfetch::Client::new(blockfetch);

    let range_ok = client.request_range((known_point.clone(), known_point.clone()));

    assert!(matches!(client.state(), blockfetch::State::Streaming));

    assert!(matches!(range_ok, Ok(_)));

    for _ in 0..1 {
        let next = client.recv_while_streaming().unwrap();

        match next {
            Some(body) => assert_eq!(body.len(), 3251),
            _ => panic!("expected block body"),
        }

        assert!(matches!(client.state(), blockfetch::State::Streaming));
    }

    let next = client.recv_while_streaming().unwrap();

    assert!(matches!(next, None));

    client.send_done().unwrap();

    assert!(matches!(client.state(), blockfetch::State::Done));
}

#[test]
#[ignore]
pub fn txsubmission_server_happy_path() {
    // TODO(pi): Note that the below doesn't work; we need a node to connect *to us*
    // during the integration test which seems awkward;
    // Alternatively, we can just set up both a client and server connecting to
    // themselves for testing!

    let N2NChannels { txsubmission, .. } = setup_n2n_client_connection();

    let mut server: Server<_, EraTxId, EraTxBody> = txsubmission::Server::new(txsubmission);

    assert!(matches!(server.wait_for_init(), Ok(_)));

    assert!(matches!(
        server.acknowledge_and_request_tx_ids(false, 0, 3),
        Ok(_)
    ));

    let reply: Result<_, _> = server.receive_next_reply();
    assert!(matches!(reply, Ok(Reply::TxIds(_))));
    let Ok(Reply::TxIds(tx_ids)) = reply else { unreachable!() };

    assert!(tx_ids.len() <= 3);

    assert!(matches!(
        server.request_txs(
            tx_ids
                .into_iter()
                .map(|txid: TxIdAndSize<EraTxId>| txid.0)
                .collect()
        ),
        Ok(_)
    ));

    let reply = server.receive_next_reply();
    assert!(matches!(reply, Ok(Reply::Txs(_))));
    let Ok(Reply::Txs(first_txs)) = reply else { unreachable!() };

    assert!(matches!(
        server.acknowledge_and_request_tx_ids(false, 1, 3),
        Ok(_)
    ));

    let reply = server.receive_next_reply();
    assert!(matches!(reply, Ok(Reply::Txs(_))));
    let Ok(Reply::Txs(second_txs)) = reply else { unreachable!() };

    // Make sure we receive the second and third tx again, indicating we sent the
    // `acknowledge 1` bit correctly
    assert_eq!(second_txs[0], first_txs[1]);
    assert_eq!(second_txs[1], first_txs[2]);

    assert!(matches!(
        server.acknowledge_and_request_tx_ids(true, 3, 3),
        Ok(_)
    ));

    match server.receive_next_reply() {
        Ok(Reply::Done) => return, // Server aint havin none of our sh*t
        Ok(Reply::TxIds(tx_ids)) => assert_eq!(tx_ids.len(), 3),
        Ok(_) | Err(_) => assert!(false),
    }
}
