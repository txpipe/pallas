use std::{net::TcpStream, os::unix::net::UnixStream, thread, time::Duration};

use pallas_multiplexer::{Channel, Multiplexer};

const PROTOCOLS: [u16; 2] = [0x0002u16, 0x0003u16];

fn main() {
    env_logger::init();

    //let bearer = TcpStream::connect("127.0.0.1:3001").unwrap();
    let bearer = UnixStream::connect("/tmp/pallas").unwrap();
    let mut muxer = Multiplexer::setup(bearer, &PROTOCOLS).unwrap();

    for protocol in PROTOCOLS {
        let handle = muxer.use_channel(protocol);

        thread::spawn(move || {
            let Channel(tx, _) = handle;

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
