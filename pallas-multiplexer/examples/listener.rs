use std::{net::TcpListener, thread, time::Duration};

use pallas_multiplexer::Multiplexer;

const PROTOCOLS: [u16; 2] = [0x8002u16, 0x8003u16];

fn main() {
    env_logger::init();

    let server = TcpListener::bind("0.0.0.0:3001").unwrap();
    let (bearer, _) = server.accept().unwrap();

    let mut muxer = Multiplexer::try_setup(bearer, &PROTOCOLS).unwrap();

    for protocol in PROTOCOLS {
        let handle = muxer.use_channel(protocol);
        
        thread::spawn(move || {
            let (rx, _tx) = handle;

            loop {
                let payload = rx.recv().unwrap();
                println!("id:{}, length:{}", protocol, payload.len());
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(6000));
    }
}
