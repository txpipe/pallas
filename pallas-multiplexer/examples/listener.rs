use std::{net::TcpListener, os::unix::net::UnixListener, thread, time::Duration};

use pallas_multiplexer::{Channel, Multiplexer};

const PROTOCOLS: [u16; 2] = [0x8002u16, 0x8003u16];

fn main() {
    env_logger::init();

    //let server = TcpListener::bind("0.0.0.0:3001").unwrap();
    let server = UnixListener::bind("/tmp/pallas").unwrap();
    let (bearer, _) = server.accept().unwrap();

    let mut muxer = Multiplexer::setup(bearer, &PROTOCOLS).unwrap();

    for protocol in PROTOCOLS {
        let handle = muxer.use_channel(protocol);
        
        thread::spawn(move || {
            let Channel(_, rx) = handle;

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
