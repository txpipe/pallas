use std::{net::TcpStream, thread, time::Duration};

use pallas_multiplexer::Multiplexer;

fn main() {
    env_logger::init();

    let bearer = TcpStream::connect("127.0.0.1:3001").unwrap();
    let handles =
        Multiplexer::new(bearer, &vec![0x0002u16, 0x0003u16][..]).unwrap();

    for (idx, handle) in handles.into_iter().enumerate() {
        thread::spawn(move || {
            let (id, rx, tx) = handle;

            loop {
                let payload = vec![1; 65545];
                tx.send(payload).unwrap();
                thread::sleep(Duration::from_millis(
                    50u64 + (idx as u64 * 10u64),
                ));
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(6000));
    }
}
