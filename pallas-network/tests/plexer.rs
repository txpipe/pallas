use std::net::{Ipv4Addr, SocketAddrV4};

use pallas_network::multiplexer::{Bearer, Plexer};
use rand::{distributions::Uniform, Rng};
use tokio::net::TcpListener;

async fn setup_passive_muxer<const P: u16>() -> Plexer {
    let server = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P))
        .await
        .unwrap();

    println!("listening for connections on port {P}");

    let (bearer, _) = Bearer::accept_tcp(&server).await.unwrap();

    Plexer::new(bearer)
}

async fn setup_active_muxer<const P: u16>() -> Plexer {
    let bearer = Bearer::connect_tcp(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P))
        .await
        .unwrap();

    println!("active plexer connected");

    Plexer::new(bearer)
}

fn random_payload(size: usize) -> Vec<u8> {
    let range = Uniform::from(0..255);
    rand::thread_rng().sample_iter(&range).take(size).collect()
}

#[tokio::test]
async fn one_way_small_sequence_of_payloads() {
    let passive = tokio::spawn(setup_passive_muxer::<50301>());

    // HACK: a small sleep seems to be required for Github actions runner to
    // formally expose the port
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let mut active = setup_active_muxer::<50301>().await;

    let mut passive = passive.await.unwrap();

    let mut sender_channel = active.subscribe_client(3);
    let mut receiver_channel = passive.subscribe_server(3);

    let passive_run = tokio::spawn(async move { passive.run().await });
    let active_run = tokio::spawn(async move { active.run().await });

    for _ in 0..100 {
        let payload = random_payload(50);
        println!("sending chunk");
        sender_channel.enqueue_chunk(payload.clone()).await.unwrap();
        let received_payload = receiver_channel.dequeue_chunk().await.unwrap();
        assert_eq!(payload, received_payload);
    }

    passive_run.abort();
    active_run.abort();
}
