use std::{net::TcpListener, thread, time::Duration};

use log::info;
use pallas_multiplexer::{Channel, Multiplexer};

const PROTOCOLS: [u16; 2] = [0x8002u16, 0x8003u16];

fn main() {
    env_logger::init();

    let server = TcpListener::bind("0.0.0.0:3001").unwrap();

    info!("listening for connections on port 3001");
    let (bearer, _) = server.accept().unwrap();

    let mut muxer = Multiplexer::setup(bearer, &PROTOCOLS).unwrap();

    for protocol in PROTOCOLS {
        let handle = muxer.use_channel(protocol);

        thread::spawn(move || {
            info!("starting thread for protocol: {}", protocol);

            let Channel(_, rx) = handle;

            loop {
                let payload = rx.recv().unwrap();
                info!(
                    "got message within thread, id:{}, length:{}",
                    protocol,
                    payload.len()
                );
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(6000));
    }
}
