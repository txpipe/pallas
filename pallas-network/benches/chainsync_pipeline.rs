//! Live benchmark comparing stop-and-wait vs pipelined ChainSync against a
//! running Cardano node (defaulting to `local-node.service`).
//!
//! Run with:
//! ```sh
//! cargo bench -p pallas-network --bench chainsync_pipeline
//! ```
//!
//! Environment overrides:
//! - `PALLAS_BENCH_N2N_ADDR` (default `127.0.0.1:6000`)
//! - `PALLAS_BENCH_N2C_SOCKET` (default `/home/westbam/haskell/local/db/socket`)
//! - `PALLAS_BENCH_MAGIC` (default `764824073` / Mainnet)
//! - `PALLAS_BENCH_DURATION_SECS` (optional; when set, runs each case for at
//!   least this many seconds instead of a fixed item count)
//! - `PALLAS_BENCH_HEADERS` (default `2000` for 0 ms RTT when duration is unset)
//! - `PALLAS_BENCH_N2C_BLOCKS` (default `500` for N2C when duration is unset)

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};

use pallas_codec::minicbor;
#[cfg(unix)]
use pallas_network::facades::NodeClient;
use pallas_network::facades::PeerClient;
use pallas_network::miniprotocols::chainsync::{
    self, DEFAULT_PIPELINE_WINDOW, HeaderContent, NextResponse, Pipeline,
};
use pallas_network::miniprotocols::{MAINNET_MAGIC, Point};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Known Mainnet Alonzo-era point so headers are ~1.1 KB Shelley+ headers with
/// standard `[header_body, body_signature]` CBOR structure.
const MAINNET_START_SLOT: u64 = 43_847_831;
const MAINNET_START_HASH: &str = "15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45";

#[derive(Clone, Copy)]
enum StopAfter {
    Items(usize),
    Duration(Duration),
}

impl StopAfter {
    fn should_continue(self, items: usize, started: Instant) -> bool {
        match self {
            StopAfter::Items(limit) => items < limit,
            StopAfter::Duration(dur) => started.elapsed() < dur,
        }
    }
}

struct BenchResult {
    label: String,
    items: usize,
    elapsed: Duration,
    max_in_flight: usize,
}

impl BenchResult {
    fn rate(&self) -> f64 {
        self.items as f64 / self.elapsed.as_secs_f64()
    }
}

/// Extract `block_number` from a Shelley+ `HeaderContent` without depending on
/// `pallas-traverse`. In Shelley..Conway, a header is `[header_body, body_sig]`
/// and `header_body` begins with `block_number: u64`.
fn decode_shelley_plus_block_number(header: &HeaderContent) -> Option<u64> {
    if header.byron_prefix.is_some() {
        return None;
    }
    let mut decoder = minicbor::Decoder::new(&header.cbor);
    decoder.array().ok()?;
    decoder.array().ok()?;
    decoder.u64().ok()
}

/// Single-connection TCP proxy injecting `rtt / 2` one-way FIFO propagation
/// delay in each direction while preserving streaming bandwidth.
async fn spawn_delay_proxy(
    upstream: String,
    rtt: Duration,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).await?;
    let local_addr = listener.local_addr()?;
    let one_way = rtt / 2;

    let handle = tokio::spawn(async move {
        let Ok((client, _)) = listener.accept().await else {
            return;
        };
        let _ = client.set_nodelay(true);
        let Ok(server) = TcpStream::connect(&upstream).await else {
            return;
        };
        let _ = server.set_nodelay(true);

        let (c_read, c_write) = client.into_split();
        let (s_read, s_write) = server.into_split();

        async fn forward(
            mut reader: tokio::net::tcp::OwnedReadHalf,
            mut writer: tokio::net::tcp::OwnedWriteHalf,
            delay: Duration,
        ) {
            let (tx, mut rx) =
                tokio::sync::mpsc::unbounded_channel::<(tokio::time::Instant, Vec<u8>)>();
            let reader_task = tokio::spawn(async move {
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let deliver_at = tokio::time::Instant::now() + delay;
                            if tx.send((deliver_at, buf[..n].to_vec())).is_err() {
                                break;
                            }
                        }
                    }
                }
            });

            while let Some((deliver_at, data)) = rx.recv().await {
                if !delay.is_zero() {
                    tokio::time::sleep_until(deliver_at).await;
                }
                if writer.write_all(&data).await.is_err() {
                    break;
                }
            }
            let _ = writer.shutdown().await;
            reader_task.abort();
        }

        let _ = tokio::join!(
            forward(c_read, s_write, one_way),
            forward(s_read, c_write, one_way),
        );
    });

    Ok((local_addr, handle))
}

async fn resolve_start_point(
    client: &mut chainsync::N2NClient,
    preferred: Point,
) -> Result<(Point, chainsync::Tip), chainsync::ClientError> {
    let (found, tip) = client
        .find_intersect(vec![preferred, Point::Origin])
        .await?;
    Ok((found.unwrap_or(Point::Origin), tip))
}

async fn bench_n2n_stop_and_wait(
    addr: &str,
    magic: u64,
    start_point: Point,
    stop: StopAfter,
) -> Result<BenchResult, Box<dyn std::error::Error>> {
    let mut peer = PeerClient::connect(addr, magic).await?;
    let client = peer.chainsync();
    let _ = resolve_start_point(client, start_point).await?;

    let start = Instant::now();
    let mut received = 0usize;

    while stop.should_continue(received, start) {
        match client.request_next().await? {
            NextResponse::RollForward(_, _) => received += 1,
            NextResponse::RollBackward(_, _) => {}
            NextResponse::Await => break,
        }
    }

    let elapsed = start.elapsed();
    client.send_done().await?;
    peer.abort().await;

    Ok(BenchResult {
        label: "stop-and-wait (window=1)".to_string(),
        items: received,
        elapsed,
        max_in_flight: 1,
    })
}

async fn bench_n2n_pipelined(
    addr: &str,
    magic: u64,
    start_point: Point,
    stop: StopAfter,
    window: usize,
    min_refill: usize,
) -> Result<BenchResult, Box<dyn std::error::Error>> {
    let mut peer = PeerClient::connect(addr, magic).await?;
    let client = peer.chainsync();
    let (_, tip) = resolve_start_point(client, start_point).await?;

    let mut pipeline = Pipeline::follow_tip(window).with_min_refill(min_refill);
    pipeline.reset_after_intersect(&tip);

    let start = Instant::now();
    let mut received = 0usize;
    let mut max_in_flight = 0usize;

    while stop.should_continue(received, start) {
        let Some(next) = pipeline.next(client).await? else {
            break;
        };
        max_in_flight = max_in_flight.max(client.in_flight() + 1);
        match next {
            NextResponse::RollForward(header, _) => {
                received += 1;
                if let Some(height) = decode_shelley_plus_block_number(&header) {
                    pipeline.set_local_height(height);
                } else {
                    // Fallback if running against Byron origin: use received count.
                    pipeline.set_local_height(received as u64);
                }
            }
            NextResponse::RollBackward(_, _) => {}
            NextResponse::Await => break,
        }
    }

    let elapsed = start.elapsed();

    // Drain any in-flight responses cleanly so send_done succeeds in State::Idle.
    while pipeline.drain_next(client).await?.is_some() {}
    client.send_done().await?;
    peer.abort().await;

    Ok(BenchResult {
        label: format!("pipelined (window={window}, refill={min_refill})"),
        items: received,
        elapsed,
        max_in_flight,
    })
}

async fn verify_stop_at_tip_already_synced(
    addr: &str,
    magic: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut peer = PeerClient::connect(addr, magic).await?;
    let client = peer.chainsync();

    let (_, chainsync::Tip(tip_point, _)) = client.find_intersect(vec![Point::Origin]).await?;
    let (found, tip) = client.find_intersect(vec![tip_point.clone()]).await?;
    assert_eq!(found, Some(tip_point.clone()));

    let mut pipeline = Pipeline::stop_at_tip(DEFAULT_PIPELINE_WINDOW);
    pipeline.reset_after_intersect(&tip);

    let mut saw_rollback = false;
    while let Some(next) = pipeline.next(client).await? {
        match next {
            NextResponse::RollBackward(_, _) => saw_rollback = true,
            NextResponse::RollForward(h, _) => {
                if let Some(height) = decode_shelley_plus_block_number(&h) {
                    pipeline.set_local_height(height);
                }
            }
            NextResponse::Await => break,
        }
    }

    assert!(saw_rollback, "expected initial RollBackward at tip");
    assert_eq!(client.in_flight(), 0);
    client.send_done().await?;
    peer.abort().await;
    Ok(())
}

#[cfg(unix)]
async fn bench_n2c(
    socket_path: &str,
    magic: u64,
    start_point: Point,
    stop: StopAfter,
) -> Result<Vec<BenchResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    // Stop-and-wait N2C
    {
        let mut node = NodeClient::connect(socket_path, magic).await?;
        let client = node.chainsync();
        let _ = client
            .find_intersect(vec![start_point.clone(), Point::Origin])
            .await?;

        let start = Instant::now();
        let mut received = 0usize;
        while stop.should_continue(received, start) {
            match client.request_next().await? {
                NextResponse::RollForward(_, _) => received += 1,
                NextResponse::RollBackward(_, _) => {}
                NextResponse::Await => break,
            }
        }
        let elapsed = start.elapsed();
        client.send_done().await?;
        node.abort().await;

        results.push(BenchResult {
            label: "N2C stop-and-wait (window=1)".to_string(),
            items: received,
            elapsed,
            max_in_flight: 1,
        });
    }

    // Pipelined N2C
    for window in [16usize, 32, 64] {
        let mut node = NodeClient::connect(socket_path, magic).await?;
        let client = node.chainsync();
        let (_, tip) = client
            .find_intersect(vec![start_point.clone(), Point::Origin])
            .await?;

        let mut pipeline = Pipeline::fixed_window(window).with_min_refill(window / 4);
        pipeline.reset_after_intersect(&tip);

        let start = Instant::now();
        let mut received = 0usize;
        let mut max_in_flight = 0usize;

        while stop.should_continue(received, start) {
            let Some(next) = pipeline.next(client).await? else {
                break;
            };
            max_in_flight = max_in_flight.max(client.in_flight() + 1);
            match next {
                NextResponse::RollForward(_, _) => received += 1,
                NextResponse::RollBackward(_, _) => {}
                NextResponse::Await => break,
            }
        }
        let elapsed = start.elapsed();

        while pipeline.drain_next(client).await?.is_some() {}
        client.send_done().await?;
        node.abort().await;

        results.push(BenchResult {
            label: format!("N2C pipelined (window={window})"),
            items: received,
            elapsed,
            max_in_flight,
        });
    }

    Ok(results)
}

fn print_table(title: &str, unit: &str, results: &[BenchResult]) {
    println!("\n=== {title} ===");
    println!(
        "{:<38} {:>8} {:>10} {:>14} {:>10} {:>10}",
        "mode", unit, "elapsed", "throughput", "speedup", "peak_inflt"
    );
    let baseline = results.first().map(BenchResult::rate).unwrap_or(1.0);
    for r in results {
        let rate = r.rate();
        let speedup = rate / baseline;
        println!(
            "{:<38} {:>8} {:>8.1}ms {:>11.1}/s {:>9.2}x {:>10}",
            r.label,
            r.items,
            r.elapsed.as_secs_f64() * 1000.0,
            rate,
            speedup,
            r.max_in_flight,
        );
    }
}

#[tokio::main]
async fn main() {
    let n2n_addr =
        std::env::var("PALLAS_BENCH_N2N_ADDR").unwrap_or_else(|_| "127.0.0.1:6000".to_string());
    let n2c_socket = std::env::var("PALLAS_BENCH_N2C_SOCKET")
        .unwrap_or_else(|_| "/home/westbam/haskell/local/db/socket".to_string());
    let magic: u64 = std::env::var("PALLAS_BENCH_MAGIC")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(MAINNET_MAGIC);
    let duration_override: Option<Duration> = std::env::var("PALLAS_BENCH_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&s| s > 0)
        .map(Duration::from_secs);
    let local_headers: usize = std::env::var("PALLAS_BENCH_HEADERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000);
    let n2c_blocks: usize = std::env::var("PALLAS_BENCH_N2C_BLOCKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);

    let start_point = Point::Specific(
        MAINNET_START_SLOT,
        hex::decode(MAINNET_START_HASH).expect("valid hex hash"),
    );

    println!("ChainSync Pipeline Benchmark against {n2n_addr} (magic={magic})");
    if let Some(dur) = duration_override {
        println!(
            "Running sustained time-boxed mode: {}s per case.",
            dur.as_secs()
        );
    }

    if let Err(err) = verify_stop_at_tip_already_synced(&n2n_addr, magic).await {
        eprintln!("Could not connect to local-node at {n2n_addr}: {err}");
        eprintln!("Ensure local-node.service is running or set PALLAS_BENCH_N2N_ADDR.");
        std::process::exit(1);
    }
    println!("Verified live tip intersection + StopAtTip clean drain/Done on {n2n_addr}.");

    // 1. Direct N2N (0 ms added RTT)
    let direct_stop = duration_override
        .map(StopAfter::Duration)
        .unwrap_or(StopAfter::Items(local_headers));
    let mut direct = Vec::new();
    direct.push(
        bench_n2n_stop_and_wait(&n2n_addr, magic, start_point.clone(), direct_stop)
            .await
            .unwrap(),
    );
    for (window, min_refill) in [(8, 1), (16, 1), (32, 1), (64, 1), (64, 16)] {
        direct.push(
            bench_n2n_pipelined(
                &n2n_addr,
                magic,
                start_point.clone(),
                direct_stop,
                window,
                min_refill,
            )
            .await
            .unwrap(),
        );
    }
    print_table(
        &format!("N2N Direct ({n2n_addr}, 0ms added RTT)"),
        "headers",
        &direct,
    );

    // 2. Simulated WAN RTT against real local-node.service
    for (rtt_ms, sw_headers, pipe_headers) in [(5u64, 300usize, 2_000usize), (20, 100, 2_000)] {
        let rtt = Duration::from_millis(rtt_ms);
        let sw_stop = duration_override
            .map(StopAfter::Duration)
            .unwrap_or(StopAfter::Items(sw_headers));
        let pipe_stop = duration_override
            .map(StopAfter::Duration)
            .unwrap_or(StopAfter::Items(pipe_headers));
        let mut wan = Vec::new();

        {
            let (proxy_addr, proxy) = spawn_delay_proxy(n2n_addr.clone(), rtt).await.unwrap();
            wan.push(
                bench_n2n_stop_and_wait(
                    &proxy_addr.to_string(),
                    magic,
                    start_point.clone(),
                    sw_stop,
                )
                .await
                .unwrap(),
            );
            proxy.abort();
        }

        for (window, min_refill) in [(16, 1), (32, 1), (64, 1), (64, 16)] {
            let (proxy_addr, proxy) = spawn_delay_proxy(n2n_addr.clone(), rtt).await.unwrap();
            wan.push(
                bench_n2n_pipelined(
                    &proxy_addr.to_string(),
                    magic,
                    start_point.clone(),
                    pipe_stop,
                    window,
                    min_refill,
                )
                .await
                .unwrap(),
            );
            proxy.abort();
        }

        print_table(
            &format!("N2N Simulated WAN ({rtt_ms}ms RTT -> {n2n_addr})"),
            "headers",
            &wan,
        );
    }

    // 3. Direct N2C Unix socket (full blocks)
    #[cfg(unix)]
    if std::path::Path::new(&n2c_socket).exists() {
        let n2c_stop = duration_override
            .map(StopAfter::Duration)
            .unwrap_or(StopAfter::Items(n2c_blocks));
        match bench_n2c(&n2c_socket, magic, start_point, n2c_stop).await {
            Ok(n2c) => print_table(
                &format!("N2C Direct Unix Socket ({n2c_socket})"),
                "blocks",
                &n2c,
            ),
            Err(err) => eprintln!("Skipping N2C benchmark ({n2c_socket}): {err}"),
        }
    }
}
