#![cfg(feature = "leios")]
use pallas_network::{
    facades::{PeerClient, PeerServer},
    miniprotocols::leiosfetch::{self, minicbor, AnyCbor, ClientRequest},
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
        hex::decode("c579268ab0275662d47a3fe2dfcb41981426ddfc217ed3091364ae8f58198809").unwrap();

    // CBOR bytes obtained from `leiosdemo202510` binary @ ccbe69384bd3d352dc5d31
    let endorser_block = hex::decode(
        "bf5820521cacab5d8886db5c111290f8901276a44bc3f3b11b781bef5233\
         ddab1b2db618375820daa5ecee19aa3f240024a59103b37ceb3f4dc7d7ea\
         d8b0c675ff5939d7faa143183758200b1457b31bd0d0293cde0ca2b9f4d4\
         8707e63d2959914c78a798536f9d310850183758205723adfca7765e74f4\
         a0659abeaffadc09be35325aa306e3ff1f6f4f74bb47491903e8ff",
    )
    .unwrap();

    let endorser_block: leiosfetch::EndorserBlock = minicbor::decode(&endorser_block).unwrap();

    let rb_header: leiosfetch::Header =
        hex::decode("eade0000eade0000eade0000eade0000eade0000eade0000eade0000eade0000").unwrap();

    let block_txs_hash: leiosfetch::Hash =
        hex::decode("bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0bee0").unwrap();

    // Selects first 3 transactions
    let tx_map = leiosfetch::TxMap::from([(0, 0xe000000000000000)]);

    let block_slot: leiosfetch::Slot = 5;
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
        let tx_map = tx_map.clone();
        let block_txs_hash = block_txs_hash.clone();
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

            // server receives `BlockTxsRequest` from client
            debug!("server waiting for txs request");
            assert_eq!(
                server_lf.recv_while_idle().await.unwrap().unwrap(),
                ClientRequest::BlockTxsRequest(block_slot, block_txs_hash, tx_map.clone()),
            );
            assert_eq!(*server_lf.state(), leiosfetch::State::BlockTxs);

            // Server selects Txs according to map and sends
            server_lf
                .send_block_txs(tx_selection(tx_map, &eb_tx()))
                .await
                .unwrap();
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

        // client sends `BlockRequest`, receives endorser block
        client_lf
            .send_block_request(block_slot, block_hash)
            .await
            .unwrap();
        assert_eq!(client_lf.recv_block().await.unwrap(), endorser_block);
        assert_eq!(*client_lf.state(), leiosfetch::State::Idle);

        // client sends `BlockTxsRequest`, receives vec of txs
        client_lf
            .send_block_txs_request(block_slot, block_txs_hash, tx_map)
            .await
            .unwrap();
        assert_eq!(client_lf.recv_block_txs().await.unwrap(), eb_tx()[0..3]);

        // client sends Done
        client_lf.send_done().await.unwrap();
        assert!(client_lf.is_done())
    });

    tokio::try_join!(client, server).unwrap();
}

fn bitmap_to_indices(bitmap: u64) -> Vec<usize> {
    (0..64)
        .rev()
        .enumerate()
        .filter(|(_, y)| (bitmap >> y) & 1 == 1)
        .map(|(x, _)| x)
        .collect()
}

fn tx_selection(tx_map: leiosfetch::TxMap, data: &[AnyCbor]) -> Vec<AnyCbor> {
    tx_map
        .into_iter()
        .map(|(index, bitmap)| {
            bitmap_to_indices(bitmap)
                .into_iter()
                .map(move |i| data[64 * index as usize + i].clone())
        })
        .flatten()
        .collect()
}

fn eb_tx() -> Vec<AnyCbor> {
    vec![
        hex::decode(
            "58359719B92F47E7ABC8436813A42C1A5780C4ADDBF008E58E6CB8A4A3142067\
             E2BD47E713EBDB3672446C8DD5697D6F29477DA5ABD6F9",
        )
        .unwrap(),
        hex::decode(
            "583551C27E9FD7D03351C243B98F6E33E9D29AD62CE9061580358B9CD4754505\
             7B54A726322F849C5D73C01AE9881AA458F3A5F9DEA664",
        )
        .unwrap(),
        hex::decode(
            "58356764A66870461BD63041BF1028FF898BDC58E95DA9EA6E684EBCC225F97A\
             ECF647BC7EA72BAC069D1FF9E3E9CB59C72181585FD4F0",
        )
        .unwrap(),
        hex::decode(
            "5903E584035557626AE726D5BCE067C798B43B3DE035C3618F86CA1CF31969EB\
             B6711D354C445650D52E34F9E9A2057ECB363FE04FD3D5CE76B05E7C0CE7C563\
             C8F89AF65F3B57D6E34481A13889FACCE87AF020F0044B5EEA3C1BD48387506D\
             BD3C75ED4B9EFD7605DC3571A95B6E97F349C61C5D444A93DDE14F27C7B6EF74\
             F802EA1AB809ECBBEFD9229A85B42BC959B70BD207C06F30675B177096931759\
             462E64B9F9F90EA5E5C5AA975A454F12AC6E4D21BC641A00B994B15E54BE2D79\
             382A5ECF65BAA76496433D191CD0BEEB1AD979CD070CDC94FFFECD01CB3BF1E9\
             86FEA8FE343C419AE71FC9CE7053697BCB75A45552006EFB1D4F36A34E9D70FE\
             663C5B28D497373DB42AE1A6B8B5BD05390FBF580FCD75D857C9047FBB2A3FA8\
             265702FD21773E124A5338E88D922A892331B9A7EE3F7375F9864E6990901D32\
             3E37AB088528FC456B9082F40527C9565248D1D0403CEBEAE8BE8DDF290D0C0F\
             C415487747EFA5D256FA3F997E0D0F111C9F22D9F41C384C0FAA22AFE97BCCCB\
             D663268AE89A7BEC8898D5CEED1ECDFABC33205F8B01CEC18079B03BB7D5BBD8\
             EF80D6FB65FDC4F0445C8712CD717E5879663400652C16C8ECA980AFEC745A2C\
             C17D6A3EA1F9D2A4B0D534F784B35BAD97CCBB495E961D010C0A3FCF89FE7EAE\
             091B00991EFF8BDB6E36C47FCBD1620130CAE67D68E68CFBE8D43BEBBA8B2331\
             F89F931D9FAA722789BFF1A6A0070480D87D59A94C62A8944EF5D327E7200030\
             5502F26E7F3FF43C7C46097204C449F07C2F3DA9A9962B7AE51E6117FBF2B591\
             AB4273BA88F9C758EE64CF10FB2BF5F25B0B287F5081A79CEFDBBB0CBB70B9D9\
             DACBC1868C37B731C6C73F49F31C4F047D236DF3ED0BD2C41F4F19B9164D2DA3\
             CAC0067168746965C1B77EDE72A35F0BBD478FF21AE128D20FED009FCA1653CC\
             16B7DE7F4FC1FBA75062B2E41BA0FFCBB8CA7213694C6947678BA2547BEF34FE\
             CD165A8ABB1DF0E52EBC0600361EFDE93031B290FA63F72F7DBA8F94FB34E6E3\
             331C84367E4E887BBE982A905564993D7432BD2FE60061B39F0411486669FACA\
             F43E2A589EEBCC635F3D1C887C8444BD8994C2AE726F402CC846E6E150688FA9\
             EEAF836AC0EA978C776C4A14B4ECD9A54104A0D4FA8EEABBB5FBD4EEE80A19A0\
             01547A1893BF3FAFF98994AD3E127CC4E35E13DA8EDF587DE0DB61824B2601C0\
             46B83088A95B3DAE5CE118516F7E95E90DBD22A7315A1B990FBB81C264D4E903\
             5935536ED84FF3D9951EED006ADB6C15F09691DC27037F19227004AE54D682F3\
             6EE41C20A27E07F10CC3BF2CF68C92E4429D9AA75D2AE487C759AD1EF37263F3\
             0BD4A50B4145C2B41C833C382FE4A5D15456346BF039A1E840BBF32F99AC80B4\
             A1930D5E838254F5",
        )
        .unwrap(),
    ]
    .into_iter()
    .map(|x| minicbor::decode(&x).unwrap())
    .collect()
}

// For testing purposes:
//
//     println!("{:?}", bitmap_to_indices(0xd000000000000002));             // [0, 1, 3, 62]
//     println!("{:?}", bitmap_to_indices(0xe000000000000000));             // [0, 1, 2]
//     println!("{:02X?}", tx_selection(0, 0x4000000000000000, &eb_tx()));  // [second tx]
