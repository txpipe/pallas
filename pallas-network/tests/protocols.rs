use hex::FromHex;
use pallas_codec::utils::{AnyCbor, AnyUInt, Bytes, KeyValuePairs, TagWrap, Nullable};
use pallas_crypto::hash::Hash;
use pallas_network::miniprotocols::localstate::queries_v16::{
    self, Addr, Addrs, ChainBlockNumber, Fraction, GenesisConfig, RationalNumber, Snapshots,
    StakeAddr, Stakes, SystemStart, UnitInterval, Value, PoolParams,
    primitives::{PoolMetadata, Relay},
};
use pallas_network::{
    facades::{NodeClient, PeerClient, PeerServer},
    miniprotocols::{
        blockfetch,
        blockfetch::BlockRequest,
        chainsync::{self, NextResponse},
        chainsync::{ClientRequest, HeaderContent, Tip},
        handshake,
        handshake::n2n::VersionData,
        localstate,
        localstate::ClientQueryRequest,
        txsubmission,
        txsubmission::{EraTxBody, TxIdAndSize},
        Point, MAINNET_MAGIC,
    },
    multiplexer::{Bearer, Plexer},
};
use std::{
    collections::{BTreeSet, BTreeMap},
    fs,
    net::{Ipv4Addr, SocketAddrV4},
    path::Path,
    str::FromStr,
    time::Duration,
};

use tokio::net::TcpListener;

#[cfg(unix)]
use tokio::net::UnixListener;

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

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30003))
        .await
        .unwrap();

    let server = tokio::spawn({
        let bodies = block_bodies.clone();
        let point = point.clone();
        async move {
            // server setup

            let mut peer_server = PeerServer::accept(&listener, 0).await.unwrap();

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

        let mut client_to_server_conn = PeerClient::connect("localhost:30003", 0).await.unwrap();

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

    tokio::try_join!(client, server).unwrap();
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

            let server_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30002))
                .await
                .unwrap();

            let (bearer, _) = Bearer::accept_tcp(&server_listener).await.unwrap();

            let mut server_plexer = Plexer::new(bearer);

            let mut server_hs: handshake::Server<VersionData> =
                handshake::Server::new(server_plexer.subscribe_server(0));
            let mut server_cs = chainsync::N2NServer::new(server_plexer.subscribe_server(2));

            let server_plexer = server_plexer.spawn();

            server_hs.receive_proposed_versions().await.unwrap();
            server_hs
                .accept_version(10, VersionData::new(0, false, None, None))
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

            server_plexer.abort().await;
        }
    });

    let client = tokio::spawn(async move {
        let mut client_to_server_conn = PeerClient::connect("localhost:30002", 0).await.unwrap();

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

    tokio::try_join!(client, server).unwrap();
}

#[cfg(unix)]
#[tokio::test]
pub async fn local_state_query_server_and_client_happy_path() {
    let server = tokio::spawn({
        async move {
            // server setup
            let socket_path = Path::new("node1.socket");

            if socket_path.exists() {
                fs::remove_file(socket_path).unwrap();
            }

            let listener = UnixListener::bind(socket_path).unwrap();

            let mut server = pallas_network::facades::NodeServer::accept(&listener, 0)
                .await
                .unwrap();

            // wait for acquire request from client

            let maybe_acquire = server.statequery().recv_while_idle().await.unwrap();

            assert!(maybe_acquire.is_some());
            assert_eq!(*server.statequery().state(), localstate::State::Acquiring);

            server.statequery().send_acquired().await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

            // server receives query from client

            let query: queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!(
                        "(While expecting `GetSystemStart`) \
                                 Unexpected message from client: {x:?}"
                    ),
                };

            assert_eq!(query, queries_v16::Request::GetSystemStart);
            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let result = AnyCbor::from_encode(SystemStart {
                year: 2020,
                day_of_year: 1,
                picoseconds_of_day: 999999999,
            });

            server.statequery().send_result(result).await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

            // server receives query from client
            let query: queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!(
                        "(While expecting `GetChainBlockNo`) \
                                 Unexpected message from client: {x:?}"
                    ),
                };

            assert_eq!(query, queries_v16::Request::GetChainBlockNo);
            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let result = AnyCbor::from_encode(ChainBlockNumber {
                slot_timeline: 1,
                block_number: 2143789,
            });

            server.statequery().send_result(result).await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

            // server receives query from client

            let query: queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!(
                        "(While expecting `GetStakeDistribution`) \
                                 Unexpected message from client: {x:?}"
                    ),
                };

            assert_eq!(
                query,
                queries_v16::Request::LedgerQuery(queries_v16::LedgerQuery::BlockQuery(
                    5,
                    queries_v16::BlockQuery::GetStakeDistribution,
                ),)
            );
            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let rational = RationalNumber {
                numerator: 10,
                denominator: 20,
            };
            let pool = localstate::queries_v16::Pool {
                stakes: rational.clone(),
                hashes: b"pool1qv4qgv62s3ha74p0643nexee9zvcdydcyahqqnavhj90zheuykz"
                    .to_vec()
                    .into(),
            };

            let pools = vec![(
                b"pool1qvfw4r3auysa5mhpr90n7mmdhs55js8gdywh0y2e3sy6568j2wp"
                    .to_vec()
                    .into(),
                pool,
            )];

            let pools = KeyValuePairs::from(pools);

            let result = AnyCbor::from_encode(queries_v16::StakeDistribution { pools });
            server.statequery().send_result(result).await.unwrap();

            // server receives query from client

            let query: queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!(
                        "(While expecting `GetUTxOByAddress`) \
                                 Unexpected message from client: {x:?}"
                    ),
                };

            let addr_hex =
"981D186018CE18F718FB185F188918A918C7186A186518AC18DD1874186D189E188410184D186F1882184D187D18C4184F1842187F18CA18A118DD"
;
            let addr = hex::decode(addr_hex).unwrap();
            let addr: Addr = addr.to_vec().into();
            let addrs: Addrs = Vec::from([addr]);

            assert_eq!(
                query,
                queries_v16::Request::LedgerQuery(queries_v16::LedgerQuery::BlockQuery(
                    5,
                    queries_v16::BlockQuery::GetUTxOByAddress(addrs),
                ),)
            );

            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let tx_hex = "1e4e5cf2889d52f1745b941090f04a65dea6ce56c5e5e66e69f65c8e36347c17";
            let txbytes: [u8; 32] = hex::decode(tx_hex).unwrap().try_into().unwrap();
            let transaction_id = Hash::from(txbytes);
            let index = AnyUInt::MajorByte(2);
            let lovelace = AnyUInt::MajorByte(2);
            let hex_datum = "9118D81879189F18D81879189F1858181C18C918CF18711866181E185316189118BA";
            let datum = hex::decode(hex_datum).unwrap().into();
            let tag = TagWrap::<_, 24>::new(datum);
            let inline_datum = Some((1_u16, tag));
            let values =
                queries_v16::TransactionOutput::Current(queries_v16::PostAlonsoTransactionOutput {
                    address: b"addr_test1vr80076l3x5uw6n94nwhgmv7ssgy6muzf47ugn6z0l92rhg2mgtu0"
                        .to_vec()
                        .into(),
                    amount: Value::Coin(lovelace),
                    inline_datum,
                    script_ref: None,
                });

            let utxo = KeyValuePairs::from(vec![(
                queries_v16::UTxO {
                    transaction_id,
                    index,
                },
                values,
            )]);

            let result = AnyCbor::from_encode(queries_v16::UTxOByAddress { utxo });
            server.statequery().send_result(result).await.unwrap();

            // server receives query from client

            let query: queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!(
                        "(While expecting `GetCurrentPParams`) \
                                 Unexpected message from client: {x:?}"
                    ),
                };
            assert_eq!(
                query,
                queries_v16::Request::LedgerQuery(queries_v16::LedgerQuery::BlockQuery(
                    5,
                    queries_v16::BlockQuery::GetCurrentPParams,
                ),)
            );
            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let result = AnyCbor::from_encode(vec![queries_v16::ProtocolParam {
                minfee_a: Some(44),
                minfee_b: Some(155381),
                max_block_body_size: Some(65536),
                max_transaction_size: Some(16384),
                max_block_header_size: Some(1100),
                key_deposit: Some(AnyUInt::U32(2000000)),
                pool_deposit: Some(AnyUInt::U32(500000000)),
                maximum_epoch: Some(100000),
                desired_number_of_stake_pools: Some(500),
                pool_pledge_influence: Some(UnitInterval {
                    numerator: 150,
                    denominator: 100,
                }),
                expansion_rate: Some(UnitInterval {
                    numerator: 3,
                    denominator: 1000000,
                }),
                treasury_growth_rate: Some(UnitInterval {
                    numerator: 3,
                    denominator: 1000000,
                }),
                protocol_version: Some((5, 0)),
                min_pool_cost: Some(AnyUInt::U32(340000000)),
                ada_per_utxo_byte: Some(AnyUInt::U16(44)),
                cost_models_for_script_languages: None,
                execution_costs: None,
                max_tx_ex_units: None,
                max_block_ex_units: None,
                max_value_size: None,
                collateral_percentage: None,
                max_collateral_inputs: None,
            }]);

            server.statequery().send_result(result).await.unwrap();

            // server receives query from client

            let query: queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!(
                        "(While expecting `GetStakeSnapshots`) \
                                 Unexpected message from client: {x:?}"
                    ),
                };

            assert_eq!(
                query,
                queries_v16::Request::LedgerQuery(queries_v16::LedgerQuery::BlockQuery(
                    5,
                    queries_v16::BlockQuery::GetStakeSnapshots(BTreeSet::new()),
                ),)
            );

            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let pool_id: Bytes =
                hex::decode("fdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b")
                    .unwrap()
                    .into();

            let stake_snapshots = KeyValuePairs::from(vec![(
                pool_id,
                Stakes {
                    snapshot_mark_pool: 0,
                    snapshot_set_pool: 0,
                    snapshot_go_pool: 0,
                },
            )]);

            let snapshots = Snapshots {
                stake_snapshots,
                snapshot_stake_mark_total: 0,
                snapshot_stake_set_total: 0,
                snapshot_stake_go_total: 0,
            };

            let result = AnyCbor::from_encode(queries_v16::StakeSnapshot { snapshots });
            server.statequery().send_result(result).await.unwrap();

            // server receives query from client
            let query: queries_v16::Request =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.into_decode().unwrap(),
                    x => panic!(
                        "(While expecting `GetGenesisConfig`) \
                                 Unexpected message from client: {x:?}"
                    ),
                };

            assert_eq!(
                query,
                queries_v16::Request::LedgerQuery(queries_v16::LedgerQuery::BlockQuery(
                    5,
                    queries_v16::BlockQuery::GetGenesisConfig,
                ),)
            );

            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let genesis = vec![GenesisConfig {
                system_start: SystemStart {
                    year: 2021,
                    day_of_year: 150,
                    picoseconds_of_day: 0,
                },
                network_magic: 42,
                network_id: 42,
                active_slots_coefficient: Fraction { num: 6, den: 10 },
                security_param: 2160,
                epoch_length: 432000,
                slots_per_kes_period: 129600,
                max_kes_evolutions: 62,
                slot_length: 1,
                update_quorum: 5,
                max_lovelace_supply: AnyUInt::MajorByte(2),
            }];

            let result = AnyCbor::from_encode(genesis);
            server.statequery().send_result(result).await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

            // server receives re-acquire from the client

            let maybe_point = match server.statequery().recv_while_acquired().await.unwrap() {
                ClientQueryRequest::ReAcquire(p) => p,
                x => panic!(
                    "(While expecting `ReAcquire`) \
                             Unexpected message from client: {x:?}"
                ),
            };

            assert_eq!(maybe_point, Some(Point::Specific(1337, vec![1, 2, 3])));
            assert_eq!(*server.statequery().state(), localstate::State::Acquiring);

            server.statequery().send_acquired().await.unwrap();

            // server receives query from client
            let query: Vec<u8> = match server.statequery().recv_while_acquired().await.unwrap() {
                ClientQueryRequest::Query(q) => q.unwrap(),
                x => panic!(
                    "(While expecting `GetFilteredDeleg...`) \
                                 Unexpected message from client: {x:?}"
                ),
            };

            let addr: Addr =
                <[u8; 28]>::from_hex("1218F563E4E10958FDABBDFB470B2F9D386215763CC89273D9BDFFFA")
                    .unwrap()
                    .to_vec()
                    .into();
            // CBOR got from preprod node. Mind the stripped `8203`.
            let cbor_query = Vec::<u8>::from_hex(
                "820082008206820a818200581c1218f563e4e10958fdabbdfb470b2f9d386215763cc89273d9bdfffa"
            ).unwrap();

            assert_eq!(query, cbor_query);
            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let pool_addr: Addr =
                <[u8; 28]>::from_hex("1E3105F23F2AC91B3FB4C35FA4FE301421028E356E114944E902005B")
                    .unwrap()
                    .to_vec()
                    .into();

            let delegs = KeyValuePairs::from(vec![(StakeAddr::from((0, addr.clone())), pool_addr)]);
            let rewards = KeyValuePairs::from(vec![(StakeAddr::from((0, addr)), 250526523)]);
            let delegs_rewards = queries_v16::FilteredDelegsRewards { delegs, rewards };

            let result = AnyCbor::from_encode(delegs_rewards);
            server.statequery().send_result(result).await.unwrap();

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
        let socket_path = "node1.socket";

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

        let request = AnyCbor::from_encode(queries_v16::Request::GetSystemStart);

        client.statequery().send_query(request).await.unwrap();

        let result: SystemStart = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        assert_eq!(
            result,
            queries_v16::SystemStart {
                year: 2020,
                day_of_year: 1,
                picoseconds_of_day: 999999999,
            }
        );

        let request = AnyCbor::from_encode(queries_v16::Request::GetChainBlockNo);
        client.statequery().send_query(request).await.unwrap();

        let result: ChainBlockNumber = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        assert_eq!(
            result,
            queries_v16::ChainBlockNumber {
                slot_timeline: 1, // current
                block_number: 2143789,
            }
        );

        let request = AnyCbor::from_encode(queries_v16::Request::LedgerQuery(
            queries_v16::LedgerQuery::BlockQuery(5, queries_v16::BlockQuery::GetStakeDistribution),
        ));

        client.statequery().send_query(request).await.unwrap();

        let result: queries_v16::StakeDistribution = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        let rational = RationalNumber {
            numerator: 10,
            denominator: 20,
        };
        let pool = localstate::queries_v16::Pool {
            stakes: rational.clone(),
            hashes: b"pool1qv4qgv62s3ha74p0643nexee9zvcdydcyahqqnavhj90zheuykz"
                .to_vec()
                .into(),
        };

        let pools = vec![(
            b"pool1qvfw4r3auysa5mhpr90n7mmdhs55js8gdywh0y2e3sy6568j2wp"
                .to_vec()
                .into(),
            pool,
        )];

        let pools = KeyValuePairs::from(pools);

        assert_eq!(result, queries_v16::StakeDistribution { pools });

        let addr_hex =
"981D186018CE18F718FB185F188918A918C7186A186518AC18DD1874186D189E188410184D186F1882184D187D18C4184F1842187F18CA18A118DD"
;
        let addr = hex::decode(addr_hex).unwrap();
        let addr: Addr = addr.to_vec().into();
        let addrs: Addrs = Vec::from([addr]);

        let request = AnyCbor::from_encode(queries_v16::Request::LedgerQuery(
            queries_v16::LedgerQuery::BlockQuery(
                5,
                queries_v16::BlockQuery::GetUTxOByAddress(addrs),
            ),
        ));

        client.statequery().send_query(request).await.unwrap();

        let result: queries_v16::UTxOByAddress = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        let tx_hex = "1e4e5cf2889d52f1745b941090f04a65dea6ce56c5e5e66e69f65c8e36347c17";
        let txbytes: [u8; 32] = hex::decode(tx_hex).unwrap().try_into().unwrap();
        let transaction_id = Hash::from(txbytes);
        let index = AnyUInt::MajorByte(2);
        let lovelace = AnyUInt::MajorByte(2);
        let hex_datum = "9118D81879189F18D81879189F1858181C18C918CF18711866181E185316189118BA";
        let datum = hex::decode(hex_datum).unwrap().into();
        let tag = TagWrap::<_, 24>::new(datum);
        let inline_datum = Some((1_u16, tag));
        let values =
            queries_v16::TransactionOutput::Current(queries_v16::PostAlonsoTransactionOutput {
                address: b"addr_test1vr80076l3x5uw6n94nwhgmv7ssgy6muzf47ugn6z0l92rhg2mgtu0"
                    .to_vec()
                    .into(),
                amount: Value::Coin(lovelace),
                inline_datum,
                script_ref: None,
            });

        let utxo = KeyValuePairs::from(vec![(
            queries_v16::UTxO {
                transaction_id,
                index,
            },
            values,
        )]);

        assert_eq!(result, queries_v16::UTxOByAddress { utxo });

        let request = AnyCbor::from_encode(queries_v16::Request::LedgerQuery(
            queries_v16::LedgerQuery::BlockQuery(5, queries_v16::BlockQuery::GetCurrentPParams),
        ));

        client.statequery().send_query(request).await.unwrap();

        let result: Vec<queries_v16::ProtocolParam> = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        assert_eq!(
            result,
            vec![queries_v16::ProtocolParam {
                minfee_a: Some(44),
                minfee_b: Some(155381),
                max_block_body_size: Some(65536),
                max_transaction_size: Some(16384),
                max_block_header_size: Some(1100),
                key_deposit: Some(AnyUInt::U32(2000000)),
                pool_deposit: Some(AnyUInt::U32(500000000)),
                maximum_epoch: Some(100000),
                desired_number_of_stake_pools: Some(500),
                pool_pledge_influence: Some(UnitInterval {
                    numerator: 150,
                    denominator: 100,
                }),
                expansion_rate: Some(UnitInterval {
                    numerator: 3,
                    denominator: 1000000,
                }),
                treasury_growth_rate: Some(UnitInterval {
                    numerator: 3,
                    denominator: 1000000,
                }),
                protocol_version: Some((5, 0)),
                min_pool_cost: Some(AnyUInt::U32(340000000)),
                ada_per_utxo_byte: Some(AnyUInt::U16(44)),
                cost_models_for_script_languages: None,
                execution_costs: None,
                max_tx_ex_units: None,
                max_block_ex_units: None,
                max_value_size: None,
                collateral_percentage: None,
                max_collateral_inputs: None,
            }]
        );

        let request = AnyCbor::from_encode(queries_v16::Request::LedgerQuery(
            queries_v16::LedgerQuery::BlockQuery(
                5,
                queries_v16::BlockQuery::GetStakeSnapshots(BTreeSet::new()),
            ),
        ));

        client.statequery().send_query(request).await.unwrap();

        let result: queries_v16::StakeSnapshot = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        let pool_id: Bytes =
            hex::decode("fdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b")
                .unwrap()
                .into();

        let stake_snapshots = KeyValuePairs::from(vec![(
            pool_id,
            Stakes {
                snapshot_mark_pool: 0,
                snapshot_set_pool: 0,
                snapshot_go_pool: 0,
            },
        )]);

        let snapshots = Snapshots {
            stake_snapshots,
            snapshot_stake_mark_total: 0,
            snapshot_stake_set_total: 0,
            snapshot_stake_go_total: 0,
        };

        assert_eq!(result, queries_v16::StakeSnapshot { snapshots });

        let request = AnyCbor::from_encode(queries_v16::Request::LedgerQuery(
            queries_v16::LedgerQuery::BlockQuery(5, queries_v16::BlockQuery::GetGenesisConfig),
        ));

        client.statequery().send_query(request).await.unwrap();

        let result: Vec<GenesisConfig> = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .into_decode()
            .unwrap();

        let genesis = vec![GenesisConfig {
            system_start: SystemStart {
                year: 2021,
                day_of_year: 150,
                picoseconds_of_day: 0,
            },
            network_magic: 42,
            network_id: 42,
            active_slots_coefficient: Fraction { num: 6, den: 10 },
            security_param: 2160,
            epoch_length: 432000,
            slots_per_kes_period: 129600,
            max_kes_evolutions: 62,
            slot_length: 1,
            update_quorum: 5,
            max_lovelace_supply: AnyUInt::MajorByte(2),
        }];

        assert_eq!(result, genesis);

        // client sends a ReAquire
        client
            .statequery()
            .send_reacquire(Some(Point::Specific(1337, vec![1, 2, 3])))
            .await
            .unwrap();

        client.statequery().recv_while_acquiring().await.unwrap();

        let addr: Addr =
            <[u8; 28]>::from_hex("1218F563E4E10958FDABBDFB470B2F9D386215763CC89273D9BDFFFA")
                .unwrap()
                .to_vec()
                .into();
        let mut addrs = BTreeSet::new();
        addrs.insert(StakeAddr::from((0x00, addr.clone())));

        let request = AnyCbor::from_encode(queries_v16::Request::LedgerQuery(
            queries_v16::LedgerQuery::BlockQuery(
                6,
                queries_v16::BlockQuery::GetFilteredDelegationsAndRewardAccounts(addrs),
            ),
        ));
        client.statequery().send_query(request).await.unwrap();

        let result: Vec<u8> = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .unwrap();

        let delegs_rewards_cbor = Vec::<u8>::from_hex(
            "8182a18200581c1218f563e4e10958fdabbdfb470b2f9d386215763cc89273d9bd\
             fffa581c1e3105f23f2ac91b3fb4c35fa4fe301421028e356e114944e902005ba1\
             8200581c1218f563e4e10958fdabbdfb470b2f9d386215763cc89273d9bdfffa1a\
             0eeebb3b",
        )
        .unwrap();

        assert_eq!(result, delegs_rewards_cbor);

        client.statequery().send_release().await.unwrap();

        client.statequery().send_done().await.unwrap();
    });

    tokio::try_join!(client, server).unwrap();
}

#[cfg(unix)]
#[tokio::test]
pub async fn local_state_query_server_and_client_happy_path2() {
    let server = tokio::spawn({
        async move {
            // server setup
            let socket_path = Path::new("node2.socket");

            if socket_path.exists() {
                fs::remove_file(socket_path).unwrap();
            }

            let listener = UnixListener::bind(socket_path).unwrap();

            let mut server = pallas_network::facades::NodeServer::accept(&listener, 0)
                .await
                .unwrap();

            // wait for acquire request from client

            let maybe_acquire = server.statequery().recv_while_idle().await.unwrap();

            assert!(maybe_acquire.is_some());
            assert_eq!(*server.statequery().state(), localstate::State::Acquiring);

            server.statequery().send_acquired().await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

            // server receives query from client

            let query: Vec<u8> =
                match server.statequery().recv_while_acquired().await.unwrap() {
                    ClientQueryRequest::Query(q) => q.unwrap(),
                    x => panic!("While expecting `GetStakePoolParams`) \
                                 Unexpected message from client: {x:?}"),
                };

            // CBOR got from preprod node. Mind the stripped `82038200`.
            let cbor_query = Vec::<u8>::from_hex(
                "820082068211d9010281581cfdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b"
            ).unwrap();
            
            assert_eq!(query, cbor_query);

            assert_eq!(*server.statequery().state(), localstate::State::Querying);

            let pool_id: Bytes = Vec::<u8>::from_hex(
                "fdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b"
            ).unwrap().into();
            let operator = pool_id.clone();
            let vrf_keyhash = Vec::<u8>::from_hex(
                "2A6A3D82278A554E9C1777C427BF0397FAF5CD7734900752D698E57679CC523F"
            ).unwrap().into();
            let reward_account = Vec::<u8>::from_hex(
                "E01AEF81CBAB75DB2DE0FE3885332EBE67C34EB1ADBF43BB2408BA3981"
            ).unwrap().into();
            let pool_metadata: Nullable<PoolMetadata> = Some(PoolMetadata {
                    url: "https://csouza.me/jp-pp.json".to_string(),
                    hash: Hash::<32>::from_str(
                        "C9623111188D0BF90E8305E40AA91A040D8036C7813A4ECA44E06FA0A1A893A6"
                    ).unwrap(),
            }).into();
            let pool_params = PoolParams {
                operator,
                vrf_keyhash,
                pledge: AnyUInt::U64(5_000_000_000),
                cost: AnyUInt::U32(340_000_000),
                margin: localstate::queries_v16::RationalNumber{ numerator: 3, denominator: 40},
                reward_account,
                pool_owners: BTreeSet::from([Bytes::from(Vec::<u8>::from_hex(
                    "1AEF81CBAB75DB2DE0FE3885332EBE67C34EB1ADBF43BB2408BA3981"
                ).unwrap())]).into(),
                relays: vec![Relay::SingleHostName(
                    Some(3001).into(),
                    "preprod.junglestakepool.com".to_string(),
                )],
                pool_metadata,
            };
            // The map is inside a (singleton) array
            let result = AnyCbor::from_encode([BTreeMap::from([(
                pool_id,
                pool_params,
            )])]);

            server.statequery().send_result(result).await.unwrap();

            assert_eq!(*server.statequery().state(), localstate::State::Acquired);

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
        let socket_path = "node2.socket";

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

        let pool_id1 = "fdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b";
        let pool_id1: Bytes = Vec::<u8>::from_hex(pool_id1).unwrap().into();
        let mut pools = BTreeSet::<Bytes>::new();
        pools.insert(pool_id1);

        let request = AnyCbor::from_encode(
            localstate::queries_v16::LedgerQuery::BlockQuery(
                6,
                localstate::queries_v16::BlockQuery::GetStakePoolParams(pools.into())
            )
        );

        client.statequery().send_query(request).await.unwrap();

        let result: Vec<u8> = client
            .statequery()
            .recv_while_querying()
            .await
            .unwrap()
            .unwrap();
        // CBOR got from preprod node.
        let pool_params_cbor = Vec::<u8>::from_hex(
            "81a1581cfdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b8958\
             1cfdb5834ba06eb4baafd50550d2dc9b3742d2c52cc5ee65bf8673823b58202a6a3d\
             82278a554e9c1777c427bf0397faf5cd7734900752d698e57679cc523f1b00000001\
             2a05f2001a1443fd00d81e82031828581de01aef81cbab75db2de0fe3885332ebe67\
             c34eb1adbf43bb2408ba3981d9010281581c1aef81cbab75db2de0fe3885332ebe67\
             c34eb1adbf43bb2408ba3981818301190bb9781b70726570726f642e6a756e676c65\
             7374616b65706f6f6c2e636f6d82781c68747470733a2f2f63736f757a612e6d652f\
             6a702d70702e6a736f6e5820c9623111188d0bf90e8305e40aa91a040d8036c7813a\
             4eca44e06fa0a1a893a6").unwrap();
        
        assert_eq!(result, pool_params_cbor);

        client.statequery().send_release().await.unwrap();

        client.statequery().send_done().await.unwrap();
    });

    tokio::try_join!(client, server).unwrap();
}

#[tokio::test]
#[ignore]
pub async fn txsubmission_server_and_client_happy_path_n2n() {
    let test_txs = vec![(vec![0], vec![0, 0, 0]), (vec![1], vec![1, 1, 1])];

    let server_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 30001))
        .await
        .unwrap();

    let server = tokio::spawn({
        let test_txs = test_txs.clone();
        async move {
            let mut peer_server = PeerServer::accept(&server_listener, 0).await.unwrap();

            let server_txsub = peer_server.txsubmission();

            // server waits for init

            server_txsub.wait_for_init().await.unwrap();

            // server requests some tx ids

            server_txsub
                .acknowledge_and_request_tx_ids(false, 0, 2)
                .await
                .unwrap();

            assert_eq!(*server_txsub.state(), txsubmission::State::TxIdsNonBlocking);

            // server receives tx ids

            let txids = match server_txsub.receive_next_reply().await.unwrap() {
                txsubmission::Reply::TxIds(x) => x,
                _ => panic!("unexpected message"),
            };

            assert_eq!(*server_txsub.state(), txsubmission::State::Idle);

            // server requests txs for ids

            let txids: Vec<_> = txids.into_iter().map(|t| t.0).collect();

            assert_eq!(txids[0].1, test_txs[0].0);
            assert_eq!(txids[1].1, test_txs[1].0);

            server_txsub.request_txs(txids).await.unwrap();

            assert_eq!(*server_txsub.state(), txsubmission::State::Txs);

            // server receives txs

            let txs = match server_txsub.receive_next_reply().await.unwrap() {
                txsubmission::Reply::Txs(x) => x,
                _ => panic!("unexpected message"),
            };

            assert_eq!(*server_txsub.state(), txsubmission::State::Idle);

            assert_eq!(txs[0].1, test_txs[0].1);
            assert_eq!(txs[1].1, test_txs[1].1);

            // server requests more tx ids (blocking)

            server_txsub
                .acknowledge_and_request_tx_ids(true, 2, 1)
                .await
                .unwrap();

            assert_eq!(*server_txsub.state(), txsubmission::State::TxIdsBlocking);

            // server receives done from client

            match server_txsub.receive_next_reply().await.unwrap() {
                txsubmission::Reply::Done => (),
                _ => panic!("unexpected message"),
            }

            assert_eq!(*server_txsub.state(), txsubmission::State::Done);
        }
    });

    let client = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut mempool = test_txs.clone();

        // client setup
        let mut client_to_server_conn = PeerClient::connect("localhost:30001", 0).await.unwrap();

        let client_txsub = client_to_server_conn.txsubmission();

        // send init

        client_txsub.send_init().await.unwrap();

        assert_eq!(*client_txsub.state(), txsubmission::State::Idle);

        // receive ids request from server

        let (_, req) = match client_txsub.next_request().await.unwrap() {
            txsubmission::Request::TxIdsNonBlocking(ack, req) => (ack, req),
            _ => panic!("unexpected message"),
        };

        assert_eq!(*client_txsub.state(), txsubmission::State::TxIdsNonBlocking);

        // send ids to server

        let to_send = mempool.drain(..req as usize).collect::<Vec<_>>();

        let ids_and_size = to_send
            .clone()
            .into_iter()
            .map(|(h, b)| TxIdAndSize(txsubmission::EraTxId(0, h), b.len() as u32))
            .collect();

        client_txsub.reply_tx_ids(ids_and_size).await.unwrap();

        assert_eq!(*client_txsub.state(), txsubmission::State::Idle);

        // receive txs request from server

        let ids = match client_txsub.next_request().await.unwrap() {
            txsubmission::Request::Txs(ids) => ids,
            _ => panic!("unexpected message"),
        };

        assert_eq!(*client_txsub.state(), txsubmission::State::Txs);

        assert_eq!(ids[0].1, test_txs[0].0);
        assert_eq!(ids[1].1, test_txs[1].0);

        // send txs to server

        let txs_to_send: Vec<_> = to_send.into_iter().map(|(_, b)| EraTxBody(0, b)).collect();

        client_txsub.reply_txs(txs_to_send).await.unwrap();

        assert_eq!(*client_txsub.state(), txsubmission::State::Idle);

        // receive tx ids request from server (blocking)

        match client_txsub.next_request().await.unwrap() {
            txsubmission::Request::TxIds(_, _) => (),
            _ => panic!("unexpected message"),
        };

        assert_eq!(*client_txsub.state(), txsubmission::State::TxIdsBlocking);

        // send done to server

        client_txsub.send_done().await.unwrap();

        assert_eq!(*client_txsub.state(), txsubmission::State::Done);
    });

    tokio::try_join!(client, server).unwrap();
}

#[tokio::test]
#[ignore]
pub async fn txsubmission_submit_to_mainnet_peer_n2n() {
    let tx_hash =
        hex::decode("8b6e50e09376b5021e93fe688ba9e7100e3682cebcb39970af5f4e5962bc5a3d").unwrap();
    let tx_hex = include_str!("../../test_data/babbage11.tx");
    let tx_bytes = hex::decode(tx_hex).unwrap();

    let mempool = vec![(tx_hash, tx_bytes)];

    // client setup
    let mut client_to_server_conn =
        PeerClient::connect("relays-new.cardano-mainnet.iohk.io:3001", MAINNET_MAGIC)
            .await
            .unwrap();

    let client_txsub = client_to_server_conn.txsubmission();

    // send init

    client_txsub.send_init().await.unwrap();

    assert_eq!(*client_txsub.state(), txsubmission::State::Idle);

    // receive ids request from server

    let ack = match client_txsub.next_request().await.unwrap() {
        txsubmission::Request::TxIds(ack, _) => {
            assert_eq!(*client_txsub.state(), txsubmission::State::TxIdsBlocking);
            ack
        }
        txsubmission::Request::TxIdsNonBlocking(ack, _) => {
            assert_eq!(*client_txsub.state(), txsubmission::State::TxIdsNonBlocking);
            ack
        }
        _ => panic!("unexpected message"),
    };

    assert_eq!(ack, 0);

    // send ids to server

    let to_send = mempool.clone();

    let ids_and_size = to_send
        .clone()
        .into_iter()
        .map(|(h, b)| TxIdAndSize(txsubmission::EraTxId(4, h), b.len() as u32))
        .collect();

    client_txsub.reply_tx_ids(ids_and_size).await.unwrap();

    assert_eq!(*client_txsub.state(), txsubmission::State::Idle);

    // receive txs request from server

    let ids = match client_txsub.next_request().await.unwrap() {
        txsubmission::Request::Txs(ids) => ids,
        _ => panic!("unexpected message"),
    };

    assert_eq!(*client_txsub.state(), txsubmission::State::Txs);

    assert_eq!(ids[0].1, mempool[0].0);

    // send txs to server

    let txs_to_send: Vec<_> = to_send.into_iter().map(|(_, b)| EraTxBody(4, b)).collect();

    client_txsub.reply_txs(txs_to_send).await.unwrap();

    assert_eq!(*client_txsub.state(), txsubmission::State::Idle);

    // receive tx ids request from server (blocking)

    // server usually sends another request before processing/acknowledging our
    // previous response, so ack is 0. the ack comes in the next message.
    match client_txsub.next_request().await.unwrap() {
        txsubmission::Request::TxIdsNonBlocking(_, _) => {
            assert_eq!(*client_txsub.state(), txsubmission::State::TxIdsNonBlocking);
        }
        _ => panic!("unexpected message"),
    };

    client_txsub.reply_tx_ids(vec![]).await.unwrap();

    let ack = match client_txsub.next_request().await.unwrap() {
        txsubmission::Request::TxIds(ack, _) => {
            assert_eq!(*client_txsub.state(), txsubmission::State::TxIdsBlocking);

            client_txsub.send_done().await.unwrap();
            assert_eq!(*client_txsub.state(), txsubmission::State::Done);

            ack
        }
        txsubmission::Request::TxIdsNonBlocking(ack, _) => {
            assert_eq!(*client_txsub.state(), txsubmission::State::TxIdsNonBlocking);

            ack
        }
        _ => panic!("unexpected message"),
    };

    // server should acknowledge the one transaction we sent now
    assert_eq!(ack, 1);
}
