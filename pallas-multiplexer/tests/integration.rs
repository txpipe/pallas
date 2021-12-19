use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

use log::info;
use pallas_multiplexer::{Channel, Multiplexer};
use rand::{distributions::Uniform, Rng};

fn setup_passive_muxer<const P: u16>() -> JoinHandle<Multiplexer> {
    thread::spawn(|| {
        let server = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P)).unwrap();
        info!("listening for connections on port 3001");
        let (bearer, _) = server.accept().unwrap();
        Multiplexer::setup(bearer, &[0x8003u16]).unwrap()
    })
}

fn setup_active_muxer<const P: u16>() -> JoinHandle<Multiplexer> {
    thread::spawn(|| {
        let bearer = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, P)).unwrap();
        Multiplexer::setup(bearer, &[0x0003u16]).unwrap()
    })
}

fn random_payload(size: usize) -> Vec<u8> {
    let range = Uniform::from(0..255);
    rand::thread_rng().sample_iter(&range).take(size).collect()
}

#[test]
fn one_way_small_payload_is_consistent() {
    let passive = setup_passive_muxer::<3001>();
    let active = setup_active_muxer::<3001>();

    let mut active_muxer = active.join().unwrap();
    let mut passive_muxer = passive.join().unwrap();

    let Channel(tx, _) = active_muxer.use_channel(0x0003u16);
    let Channel(_, rx) = passive_muxer.use_channel(0x8003u16);

    let payload = random_payload(50);
    tx.send(payload.clone()).unwrap();
    let received_payload = rx.recv().unwrap();
    assert_eq!(payload, received_payload)
}

#[test]
fn one_way_small_sequence_of_payloads_are_consistent() {
    let passive = setup_passive_muxer::<3002>();
    let active = setup_active_muxer::<3002>();

    let mut active_muxer = active.join().unwrap();
    let mut passive_muxer = passive.join().unwrap();

    let Channel(tx, _) = active_muxer.use_channel(0x0003u16);
    let Channel(_, rx) = passive_muxer.use_channel(0x8003u16);

    for _ in [0..100] {
        let payload = random_payload(50);
        tx.send(payload.clone()).unwrap();
        let received_payload = rx.recv().unwrap();
        assert_eq!(payload, received_payload)
    }
}
