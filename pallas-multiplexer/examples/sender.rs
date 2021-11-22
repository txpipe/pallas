use std::{net::TcpStream, thread, time::Duration};

use pallas_multiplexer::Multiplexer;

const PROTOCOLS: [u16; 2] = [0x0002u16, 0x0003u16];

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("127.0.0.1:3001").unwrap();
    let mut muxer = Multiplexer::try_setup(bearer, &PROTOCOLS).unwrap();

    for protocol in PROTOCOLS {
        let handle = muxer.use_channel(protocol);

        thread::spawn(move || {
            let (_rx, tx) = handle;

            loop {
                let payload = vec![1; 65545];
                tx.send(payload).unwrap();
                thread::sleep(Duration::from_millis(
                    50u64 + (protocol as u64 * 10u64),
                ));
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(6000));
    }
}
