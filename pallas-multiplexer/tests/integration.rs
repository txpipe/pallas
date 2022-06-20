use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    thread::{self, JoinHandle},
};

use log::info;
use pallas_codec::minicbor;
use pallas_multiplexer::{
    agents::{Channel, ChannelBuffer},
    bearers::Bearer,
    Payload, StdPlexer,
};
use rand::{distributions::Uniform, Rng};

fn setup_passive_muxer<const P: u16>() -> JoinHandle<StdPlexer> {
    thread::spawn(|| {
        let server = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P)).unwrap();
        info!("listening for connections on port {}", P);

        let bearer = Bearer::accept_tcp(server).unwrap();

        StdPlexer::new(bearer)
    })
}

fn setup_active_muxer<const P: u16>() -> JoinHandle<StdPlexer> {
    thread::spawn(|| {
        let bearer = Bearer::connect_tcp(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P)).unwrap();

        StdPlexer::new(bearer)
    })
}

fn random_payload(size: usize) -> Vec<u8> {
    let range = Uniform::from(0..255);
    rand::thread_rng().sample_iter(&range).take(size).collect()
}

#[test]
fn one_way_small_sequence_of_payloads() {
    let passive = setup_passive_muxer::<50301>();

    // HACK: a small sleep seems to be required for Github actions runner to
    // formally expose the port
    thread::sleep(std::time::Duration::from_secs(1));

    let active = setup_active_muxer::<50301>();

    let mut active_plexer = active.join().unwrap();
    let mut passive_plexer = passive.join().unwrap();

    let mut sender_channel = active_plexer.use_channel(0x0003u16);
    let mut receiver_channel = passive_plexer.use_channel(0x8003u16);

    active_plexer.muxer.spawn();
    passive_plexer.demuxer.spawn();

    for _ in [0..100] {
        let payload = random_payload(50);
        sender_channel.enqueue_chunk(payload.clone()).unwrap();
        let received_payload = receiver_channel.dequeue_chunk().unwrap();
        assert_eq!(payload, received_payload);
    }
}
