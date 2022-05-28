use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

use log::info;
use pallas_multiplexer::{spawn_demuxer, spawn_muxer, Channel, Multiplexer};
use rand::{distributions::Uniform, Rng};

fn setup_passive_muxer<const P: u16>() -> JoinHandle<Multiplexer<TcpStream>> {
    thread::spawn(|| {
        let server = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P)).unwrap();
        info!("listening for connections on port {}", P);
        let (bearer, _) = server.accept().unwrap();
        Multiplexer::new(bearer)
    })
}

fn setup_active_muxer<const P: u16>() -> JoinHandle<Multiplexer<TcpStream>> {
    thread::spawn(|| {
        let bearer = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P)).unwrap();
        Multiplexer::new(bearer)
    })
}

fn random_payload(size: usize) -> Vec<u8> {
    let range = Uniform::from(0..255);
    rand::thread_rng().sample_iter(&range).take(size).collect()
}

#[test]
fn one_way_small_payload_is_consistent() {
    let passive = setup_passive_muxer::<50201>();

    // HACK: a small sleep seems to be required for Github actions runner to
    // formally expose the port
    thread::sleep(std::time::Duration::from_secs(1));

    let active = setup_active_muxer::<50201>();

    let mut active_plexer = active.join().unwrap();
    let mut passive_plexer = passive.join().unwrap();

    let Channel(tx, _) = active_plexer.use_channel(0x0003u16);
    let Channel(_, rx) = passive_plexer.use_channel(0x8003u16);

    spawn_muxer(active_plexer.muxer);
    spawn_demuxer(passive_plexer.demuxer);

    let payload = random_payload(50);
    tx.send_payload(payload.clone()).unwrap();
    let received_payload = rx.recv().unwrap();
    assert_eq!(payload, received_payload)
}

#[test]
fn one_way_small_sequence_of_payloads_are_consistent() {
    let passive = setup_passive_muxer::<50301>();

    // HACK: a small sleep seems to be required for Github actions runner to
    // formally expose the port
    thread::sleep(std::time::Duration::from_secs(1));

    let active = setup_active_muxer::<50301>();

    let mut active_plexer = active.join().unwrap();
    let mut passive_plexer = passive.join().unwrap();

    let Channel(tx, _) = active_plexer.use_channel(0x0003u16);
    let Channel(_, rx) = passive_plexer.use_channel(0x8003u16);

    spawn_muxer(active_plexer.muxer);
    spawn_demuxer(passive_plexer.demuxer);

    for _ in [0..100] {
        let payload = random_payload(50);
        tx.send_payload(payload.clone()).unwrap();
        let received_payload = rx.recv().unwrap();
        assert_eq!(payload, received_payload)
    }
}
