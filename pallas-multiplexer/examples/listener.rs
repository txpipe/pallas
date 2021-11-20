use std::{net::TcpListener, thread, time::Duration};

use pallas_multiplexer::Multiplexer;

fn main() {
    env_logger::init();

    let server = TcpListener::bind("0.0.0.0:3001").unwrap();
    let (bearer, _) = server.accept().unwrap();

    let handles =
        Multiplexer::new(bearer, &vec![0x8002u16, 0x8003u16][..]).unwrap();

    for handle in handles {
        thread::spawn(move || {
            let (id, rx, tx) = handle;
            loop {
                let payload = rx.recv().unwrap();
                println!("id:{}, length:{}", id, payload.len());
            }
        });
    }

    loop {
        thread::sleep(Duration::from_secs(6000));
    }
}
