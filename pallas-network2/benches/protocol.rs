mod harness;

use criterion::{Criterion, criterion_group, criterion_main};
use harness::{InitiatorNode, ResponderNode};
use pallas_network2::behavior::InitiatorCommand;
use pallas_network2::protocol::Point;

fn bench_handshake(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("handshake", |b| {
        b.to_async(&rt).iter(|| async {
            let (addr, responder) = ResponderNode::bind().await.spawn();
            let mut initiator = InitiatorNode::connect_to(addr);
            initiator.wait_for_handshake().await;
            responder.abort();
        });
    });
}

fn bench_chainsync_headers(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("chainsync_10_headers", |b| {
        b.to_async(&rt).iter(|| async {
            let (addr, responder) = ResponderNode::bind().await.spawn();
            let mut initiator = InitiatorNode::connect_to(addr);

            initiator.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
            initiator.wait_for_intersection().await;

            for _ in 0..10 {
                initiator.execute(InitiatorCommand::ContinueSync(initiator.peer_id()));
                initiator.wait_for_header().await;
            }

            responder.abort();
        });
    });
}

fn bench_blockfetch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("blockfetch", |b| {
        b.to_async(&rt).iter(|| async {
            let (addr, responder) = ResponderNode::bind().await.spawn();
            let mut initiator = InitiatorNode::connect_to(addr);

            // chainsync is needed to promote the peer to Hot
            initiator.execute(InitiatorCommand::StartSync(vec![Point::Origin]));
            initiator.wait_for_intersection().await;

            let range = (Point::Origin, Point::new(100, vec![0xAA; 32]));
            initiator.execute(InitiatorCommand::RequestBlocks(range));
            initiator.execute(InitiatorCommand::Housekeeping);
            initiator.wait_for_block().await;

            responder.abort();
        });
    });
}

criterion_group!(
    benches,
    bench_handshake,
    bench_chainsync_headers,
    bench_blockfetch,
);
criterion_main!(benches);
