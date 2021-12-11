use std::{net::TcpStream, thread, time::Duration};

use log::info;
use pallas_multiplexer::{Channel, Multiplexer};

const PROTOCOLS: [u16; 2] = [0x0002u16, 0x0003u16];

fn main() {
    env_logger::init();

    info!("connecting to tcp socket on 127.0.0.1:3001");
    let bearer = TcpStream::connect("127.0.0.1:3001").unwrap();
    let mut muxer = Multiplexer::setup(bearer, &PROTOCOLS).unwrap();

    for protocol in PROTOCOLS {
        let handle = muxer.use_channel(protocol);

        thread::spawn(move || {
            let Channel(tx, _) = handle;

            loop {
                let payload = vec![1; 65545];
                info!("sending dumb payload for protocol: {}", protocol);
                tx.send(payload).unwrap();
                thread::sleep(Duration::from_millis(500u64 + (protocol as u64 * 10u64)));
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(6000));
    }
}
