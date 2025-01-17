use std::path::{Path, PathBuf};

use anyhow::*;
use clap::Parser;
use pallas::{
    ledger::traverse::{MultiEraBlock, MultiEraTx},
    network::{
        facades::NodeClient,
        miniprotocols::{chainsync::NextResponse, Point},
    },
};

// An arbitrary predicate to decide whether to save the block or not;
// fill in with your own purpose built logic
async fn block_matches(block: &MultiEraBlock<'_>) -> bool {
    // As an example, we save any blocks that have an "Update proposal" in any era
    block.update().is_some() || block.txs().iter().any(|tx| tx.update().is_some())
}

// An arbitrary predicate to decide whether to save the transaction or not;
// fill in with your own purpose built logic
async fn tx_matches(_tx: &MultiEraTx<'_>) -> bool {
    false
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Connect to the local node over the file socket
    let mut client = NodeClient::connect(args.socket_path.clone(), args.network_magic)
        .await
        .unwrap();

    // Find an intersection point using the points on the command line
    // The response would tell us what point we found, and what the current tip is
    // which we don't need for this tool
    let (_, _) = client
        .chainsync()
        .find_intersect(args.point.clone())
        .await?;

    loop {
        // We either request the next block, or wait until we're told that the block is
        // ready
        let next = client.chainsync().request_or_await_next().await?;
        // And depending on the message we receive...
        match next {
            // The node will send "RollForward" messages to tell us
            // about the next block in the sequence; it contains the bytes
            // of the block, and what the current tip we're advancing towards is
            NextResponse::RollForward(bytes, _) => {
                // Decode the block
                let block = MultiEraBlock::decode(&bytes)?;
                let slot = block.slot();
                let height = block.number();
                let hash = block.hash();

                if height % 10000 == 0 {
                    println!("Processed block height {}: {}/{}", height, slot, hash);
                }
                // And check each transaction for the predicate, and save if needed
                for tx in block.txs() {
                    if tx_matches(&tx).await {
                        println!("Found matching tx in block {}/{}", slot, hash);
                        // Make sure we create the out diretory
                        std::fs::create_dir_all(format!("{}/txs", args.out.to_str().unwrap()))
                            .context("couldn't create output directory")?;
                        save_file(args.tx_path(&tx), tx.encode().as_slice())?;
                    }
                }
                // Then, we can check the block as a whole
                if block_matches(&block).await {
                    println!("Found matching block {}/{}", slot, hash);
                    // Make sure we create the out diretory
                    std::fs::create_dir_all(format!("{}/blocks", args.out.to_str().unwrap()))
                        .context("couldn't create output directory")?;
                    let path = args.block_path(&block);
                    // We drop the block, because the block is
                    // holding a reference to bytes, which we need to save it
                    drop(block);
                    save_file(path, &bytes)?;
                }
            }
            // Since we're just scraping data until we catch up, we don't need to handle rollbacks
            NextResponse::RollBackward(_, _) => {}
            // Await is returned once we've caught up, and we should let
            // the node notify us when there's a new block available
            NextResponse::Await => break,
        }
    }

    Ok(())
}

/// A small utility to crawl the Cardano blockchain and save sample data
#[derive(Parser)]
struct Args {
    /// The path to the node.sock file to connect to a local node
    #[arg(short, long, env("CARDANO_NODE_SOCKET_PATH"))]
    pub socket_path: String,
    /// The network magic used to handshake with that node; defaults to mainnet
    #[arg(short, long, env("CARDANO_NETWORK_MAGIC"), default_value_t = 764824073)]
    pub network_magic: u64,
    /// A list of points to use when trying to decide a startpoint; defaults to
    /// origin
    #[arg(short, long, value_parser = parse_point)]
    pub point: Vec<Point>,
    /// Download only the first block found that matches this criteria
    #[arg(long)]
    pub one: bool,
    /// The directory to save the files into
    #[arg(short, long, default_value = "out")]
    pub out: PathBuf,
}

impl Args {
    pub fn tx_path(&self, tx: &MultiEraTx) -> String {
        format!("{}/txs/{}.cbor", self.out.to_str().unwrap(), tx.hash())
    }
    pub fn block_path(&self, block: &MultiEraBlock) -> String {
        format!(
            "{}/blocks/{}.cbor",
            self.out.to_str().unwrap(),
            block.hash()
        )
    }
}

pub fn parse_point(s: &str) -> Result<Point, Box<dyn std::error::Error + Send + Sync + 'static>> {
    if s == "origin" {
        return std::result::Result::Ok(Point::Origin);
    }
    let parts: Vec<_> = s.split('/').collect();
    let slot = parts[0].parse()?;
    let hash = hex::decode(parts[1])?;
    std::result::Result::Ok(Point::Specific(slot, hash))
}

fn save_file<P: AsRef<Path>>(filename: P, bytes: &[u8]) -> Result<()> {
    std::fs::write(filename, bytes).context("couldn't write file")
}
