use pallas::{network::{
    facades::PeerClient,
    miniprotocols::{chainsync, Point, MAINNET_MAGIC, blockfetch, keepalive},
}, ledger::traverse::MultiEraHeader};
use tokio::{time::Instant, select};
use thiserror::Error;
use futures::{future::FutureExt, pin_mut};

#[derive(Error, Debug)]
pub enum Error {
    #[error("hex conversion error")]
    FromHexError(#[from] hex::FromHexError),

    #[error("blockfetch error")]
    BlockFetchError(#[from] blockfetch::ClientError),

    #[error("chainsync error")]
    ChainSyncError(#[from] chainsync::ClientError),

    #[error("keepalive error")]
    KeepAliveError(#[from] keepalive::Error),

    #[error("pallas_traverse error")]
    PallasTraverseError(#[from] pallas::ledger::traverse::Error),
}

async fn do_blockfetch(blockfetch_client: &mut blockfetch::Client, range: (Point, Point)) -> Result<(), Error> {
    let blocks = blockfetch_client.fetch_range(range.clone()).await?;

    for block in &blocks {
        tracing::trace!("received block of size: {}", block.len());
    }
    tracing::info!("received {} blocks. last slot: {}", blocks.len(), range.1.slot_or_default());
    Ok(())
}

async fn do_chainsync(chainsync_client: &mut chainsync::N2NClient, blockfetch_client: &mut blockfetch::Client) -> Result<(), Error> {
    let known_points = vec![Point::Specific(
        43847831u64,
        hex::decode("15b9eeee849dd6386d3770b0745e0450190f7560e5159b1b3ab13b14b2684a45")?,
    )];

    let (point, _) = chainsync_client.find_intersect(known_points).await?;

    tracing::info!("intersected point is {:?}", point);

    let mut block_count = 0u16;
    let mut start_point = Point::Specific(0, vec![]);
    let mut end_point: Point;
    let mut next_log = Instant::now();
    loop {
        let next = chainsync_client.request_next().await?;

        match next {
            chainsync::NextResponse::RollForward(h, _) => {
                tracing::trace!("rolling forward, header size: {}", h.cbor.len());
                let point = match h.byron_prefix {
                    None => {
                        let multi_era_header = MultiEraHeader::decode(h.variant, None, &h.cbor)?;
                        let slot = multi_era_header.slot();
                        let hash = multi_era_header.hash().to_vec();
                        let number = multi_era_header.number();
                        match &multi_era_header {
                            MultiEraHeader::EpochBoundary(_) => {
                                tracing::info!("epoch boundary");
                                None
                            },
                            MultiEraHeader::AlonzoCompatible(_) | MultiEraHeader::Babbage(_) => {
                                if next_log.elapsed().as_secs() > 1 {
                                    tracing::info!("chainsync block header: {}", number);
                                    next_log = Instant::now();
                                }
                                Some(Point::Specific(slot, hash))
                            },
                            MultiEraHeader::Byron(_) => {
                                tracing::info!("ignoring byron header");
                                None
                            },
                        }
                    }
                    Some(_) => {
                        tracing::info!("skipping byron block");
                        None
                    }
                };
                match point {
                    Some(p) => {
                        block_count += 1;
                        if block_count == 1 {
                            start_point = p;
                        } 
                        else if block_count == 10 {
                            end_point = p;
                            do_blockfetch(blockfetch_client, (start_point.clone(), end_point.clone())).await?;
                            block_count = 0;
                        }
                    },
                    None => {},
                };
            }
            chainsync::NextResponse::RollBackward(x, _) => log::info!("rollback to {:?}", x),
            chainsync::NextResponse::Await => tracing::info!("tip of chaing reached"),
        };
    }
}

async fn do_keepalive(keepalive_client: &mut keepalive::Client) -> Result<(), Error> {
    let mut keepalive_timer = Instant::now();
    loop {
        if keepalive_timer.elapsed().as_secs() > 20 {
            tracing::info!("sending keepalive...");
            keepalive_client.send_keepalive().await?;
            tracing::info!("keepalive sent");
            keepalive_timer = Instant::now();
        }
    }
}

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish(),
    )
    .unwrap();

    loop {
        // setup a TCP socket to act as data bearer between our agents and the remote
        // relay.
        let server = "backbone.cardano-mainnet.iohk.io:3001";
        // let server = "localhost:6000";
        let mut peer = PeerClient::connect(server, MAINNET_MAGIC)
            .await
            .unwrap();

            let chainsync_handle = tokio::spawn(async move {
            do_chainsync(&mut peer.chainsync, &mut peer.blockfetch).await?;
            Ok::<(), Error>(())
        }).fuse();
        let keepalive_handle = tokio::spawn(async move {
            do_keepalive(&mut peer.keepalive).await?;
            Ok::<(), Error>(())
        }).fuse();

        pin_mut!(chainsync_handle, keepalive_handle);

        // If any of these concurrent tasks exit or fail, the others are canceled.
        select! {
            chainsync_result = chainsync_handle => {
                match chainsync_result {
                    Ok(result) => {
                        match result {
                            Ok(_) => {}
                            Err(error) => {
                                tracing::error!("chainsync error: {:?}", error);
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!("chainsync error: {:?}", error);
                    }
                }
            }
            keepalive_result = keepalive_handle => {
                match keepalive_result {
                    Ok(result) => {
                        match result {
                            Ok(_) => {}
                            Err(error) => {
                                tracing::error!("keepalive error: {:?}", error);
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!("keepalive error: {:?}", error);
                    }
                }
            }
        }
        peer.plexer_handle.abort();

        tracing::info!("waiting 10 seconds before reconnecting...");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
