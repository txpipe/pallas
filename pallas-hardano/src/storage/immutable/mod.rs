use std::{
    cmp::Ordering,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use tap::Tap;
use tracing::debug;

pub mod chunk;
pub mod primary;
pub mod secondary;

/// Performs a binary search of the given chunks
/// and returns the index of the chunk which probably contains the point.
///
/// Current algorithm slightly modified from the original binary search.
/// It returns the index of the chunk in the `chunks` vector, 
/// which could contain searching element **BUT** it may not.
/// It assumes that **EACH** chunk it is a sorted collection of elements
/// e.g. `vec![vec![1, 2, 3], vec![4, 5], vec![7, 8, 9]]` and inside `cmp`
/// function you will compare the first element of the chunk e.g.
/// `let cmp = |chunk: &Vec<i32>, point: &i32| chunk[0].cmp(point)`.
fn chunk_binary_search<ChunkT, PointT>(
    chunks: &Vec<ChunkT>,
    point: PointT,
    cmp: impl Fn(&ChunkT, &PointT) -> Ordering,
) -> Option<usize> {
    let mut size = chunks.len();
    let mut left = 0;
    let mut right: usize = size;

    while left < right {
        let mid = left + size / 2;

        // SAFETY: the while condition means `size` is strictly positive, so
        // `size/2 < size`. Thus `left + size/2 < left + size`, which
        // coupled with the `left + size <= self.len()` invariant means
        // we have `left + size/2 < self.len()`, and this is in-bounds.
        match cmp(&chunks[mid], &point) {
            Ordering::Less => left = mid + 1,
            Ordering::Greater => right = mid,
            Ordering::Equal => return Some(mid),
        };

        size = right - left;
    }

    if left > 0 {
        Some(left - 1)
    } else {
        None
    }
}

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
            .tap(|name| debug!(name, "switched to new chunk"))
            .map(|name| chunk::read_blocks(&self.0, &name))
    }
}

pub type FallibleBlock = Result<Block, std::io::Error>;

pub fn read_blocks(dir: &Path) -> Result<impl Iterator<Item = FallibleBlock>, std::io::Error> {
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
    fn chunk_binary_search_test() {
        use super::chunk_binary_search;

        let vec = vec![vec![1, 2, 3], vec![4, 5], vec![7, 8, 9]];
        let cmp = |chunk: &Vec<i32>, point: &i32| chunk[0].cmp(point);

        assert_eq!(chunk_binary_search(&vec, 0, cmp), None);
        assert_eq!(chunk_binary_search(&vec, 1, cmp), Some(0));
        assert_eq!(chunk_binary_search(&vec, 2, cmp), Some(0));
        assert_eq!(chunk_binary_search(&vec, 3, cmp), Some(0));
        assert_eq!(chunk_binary_search(&vec, 4, cmp), Some(1));
        assert_eq!(chunk_binary_search(&vec, 5, cmp), Some(1));
        assert_eq!(chunk_binary_search(&vec, 6, cmp), Some(1));
        assert_eq!(chunk_binary_search(&vec, 7, cmp), Some(2));
        assert_eq!(chunk_binary_search(&vec, 8, cmp), Some(2));
        assert_eq!(chunk_binary_search(&vec, 9, cmp), Some(2));
        assert_eq!(chunk_binary_search(&vec, 10, cmp), Some(2));
    }

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

        let path = option_env!("PALLAS_MITHRIL_SNAPSHOT_PATH").unwrap();
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

        assert!(count > 0);
    }
}
