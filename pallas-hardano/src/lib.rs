#![feature(trait_alias)]
#![feature(result_option_inspect)]

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use tracing::debug;

pub mod chunk;
pub mod primary;
pub mod secondary;

fn build_stack_of_chunk_names(dir: &Path) -> Result<ChunkNameSack, std::io::Error> {
    let mut chunks = std::fs::read_dir(dir)?
        .map_while(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|e| e.to_string_lossy() == "chunk")
                .unwrap_or_default()
        })
        .filter_map(|e| e.path().file_stem().map(OsStr::to_owned))
        .map(|s| s.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    chunks.sort();
    chunks.reverse();

    Ok(chunks)
}

pub type Block = Vec<u8>;

pub type ChunkName = String;
pub type ChunkNameSack = Vec<ChunkName>;

pub struct ChunkReaders(PathBuf, ChunkNameSack);

impl Iterator for ChunkReaders {
    type Item = Result<chunk::Reader, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.1
            .pop()
            .inspect(|name| debug!(name, "switched to new chunk"))
            .map(|name| chunk::read_blocks(&self.0, &name))
    }
}

pub trait BlockIterator = Iterator<Item = Result<Block, std::io::Error>>;

pub fn read_blocks(dir: &Path) -> Result<impl BlockIterator, std::io::Error> {
    let names = build_stack_of_chunk_names(dir)?;

    let iter = ChunkReaders(dir.to_owned(), names)
        .map_while(Result::ok)
        .flatten();

    Ok(iter)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pallas_traverse::MultiEraBlock;
    use tracing::trace;

    #[test]
    fn can_read_multiple_chunks_from_folder() {
        let reader = super::read_blocks(Path::new("../test_data")).unwrap();

        let mut count = 0;
        let mut last_slot = None;

        for block in reader {
            let block = block.unwrap();
            let block = MultiEraBlock::decode(&block).unwrap();

            if let Some(last_slot) = last_slot {
                assert!(last_slot < block.slot());
            }

            last_slot = Some(block.slot());
            count += 1;
        }

        assert_eq!(count, 1778);
    }

    #[test]
    #[ignore]
    fn can_read_whole_mithril_snapshot() {
        tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )
        .unwrap();

        let path = option_env!("PALLAS_MITRHIL_SNAPSHOT_PATH").unwrap();
        let reader = super::read_blocks(Path::new(path)).unwrap();

        let mut count = 0;
        let mut last_slot = None;
        let mut last_height = None;
        let mut last_hash = None;

        for block in reader.take_while(Result::is_ok) {
            let block = block.unwrap();
            let block = MultiEraBlock::decode(&block).unwrap();

            trace!("{}", block.hash());

            if let Some(last_slot) = last_slot {
                assert!(last_slot < block.slot());
            }

            if let Some(last_height) = last_height {
                assert_eq!(last_height + 1, block.number());
            }

            if let Some(last_hash) = last_hash {
                if let Some(expected) = block.header().previous_hash() {
                    assert_eq!(last_hash, expected)
                }
            }

            last_slot = Some(block.slot());
            last_height = Some(block.number());
            last_hash = Some(block.hash());

            count += 1;
        }

        assert_eq!(count, 1_563_646);
    }
}
