use std::{
    cmp::Ordering,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use pallas_network::miniprotocols::Point;
use pallas_traverse::MultiEraBlock;
use tap::Tap;
use tracing::debug;

pub mod chunk;
pub mod primary;
pub mod secondary;

/// Performs a binary search of the given sorted chunks in descending order
/// and returns the index of the chunk which probably contains the point.
///
/// Current algorithm slightly modified from the original binary search.
/// It returns the index of the chunk in the `chunks` vector,
/// which could contain searching element **BUT** it may not.
/// It assumes that **EACH** chunk it is a sorted collection **BUT** in ascending order,
/// e.g. `vec![vec![7, 8, 9], vec![4, 5], vec![1, 2, 3]]` and inside `cmp`
/// function you will compare the first element of the chunk e.g.
/// `let cmp = |chunk: &Vec<i32>, point: &i32| chunk[0].cmp(point)`.
fn chunk_binary_search<ChunkT, PointT>(
    chunks: &Vec<ChunkT>,
    point: &PointT,
    cmp: impl Fn(&ChunkT, &PointT) -> Result<Ordering, Box<dyn std::error::Error>>,
) -> Result<Option<usize>, Box<dyn std::error::Error>> {
    let mut size = chunks.len();
    let mut left = 0;
    let mut right: usize = size;

    while size > 0 {
        let mid = left + size / 2;

        // SAFETY: the while condition means `size` is strictly positive, so
        // `size/2 < size`. Thus `left + size/2 < left + size`, which
        // coupled with the `left + size <= self.len()` invariant means
        // we have `left + size/2 < self.len()`, and this is in-bounds.
        match cmp(&chunks[mid], point)? {
            Ordering::Less => right = mid,
            Ordering::Greater => left = mid + 1,
            Ordering::Equal => return Ok(Some(mid)),
        };

        size = right - left;
    }

    if right < chunks.len() {
        Ok(Some(right))
    } else {
        Ok(None)
    }
}

/// Iterates the blocks till given slot and block hash.
/// Returns an iterator over the blocks if specific block found, otherwise returns an error.
fn iterate_till_point(
    iter: impl Iterator<Item = FallibleBlock>,
    slot: u64,
    block_hash: &[u8],
) -> Result<impl Iterator<Item = FallibleBlock>, Box<dyn std::error::Error>> {
    let mut iter = iter.peekable();
    match iter.peek() {
        Some(Ok(block_data)) => {
            let mut block_data = block_data.clone();
            let mut block = MultiEraBlock::decode(&block_data)?;

            while block.slot() < slot {
                iter.next();

                match iter.peek() {
                    Some(Ok(data)) => {
                        block_data = data.clone();
                        block = MultiEraBlock::decode(&block_data)?;
                    }
                    Some(Err(_)) | None => return Ok(iter),
                }
            }

            if block.hash().as_ref().eq(block_hash) && block.slot() == slot {
                Ok(iter)
            } else {
                Err("Cannot find the block".into())
            }
        }
        Some(Err(_)) | None => Ok(iter),
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

/// Returns an iterator over the chain from the given point.
pub fn read_blocks_from_point(
    dir: &Path,
    point: Point,
) -> Result<Box<dyn Iterator<Item = FallibleBlock>>, Box<dyn std::error::Error>> {
    let names = build_stack_of_chunk_names(dir)?;

    match point {
        // Establish iterator from the beginning of the chain
        Point::Origin => {
            let iter = ChunkReaders(dir.to_owned(), names)
                .map_while(Result::ok)
                .flatten();

            Ok(Box::new(iter))
        }
        // Establish iterator from specific block
        Point::Specific(slot, block_hash) => {
            // Comparator function for binary search.
            // Takes the first block from the chunk
            // and compares block's slot with provided slot number
            let cmp = {
                |chunk_name: &String, point: &u64| {
                    let mut blocks = chunk::read_blocks(dir, &chunk_name)?;

                    // Try to read the first block from the chunk
                    if let Some(block_data) = blocks.next() {
                        let block_data = block_data?;

                        let block = MultiEraBlock::decode(&block_data)?;
                        Ok(block.slot().cmp(point))
                    } else {
                        Ok(Ordering::Greater)
                    }
                }
            };

            // Finds chunk index and creates a truncated `names` vector from the found index.
            let names = chunk_binary_search(&names, &slot, cmp)?
                .map(|chunk_index| names[chunk_index..].to_vec())
                .ok_or::<Box<dyn std::error::Error>>("Cannot find the block".into())?;

            let iter = ChunkReaders(dir.to_owned(), names.clone())
                .map_while(Result::ok)
                .flatten()
                .peekable();

            // Iterates util the block is found by the provided `point`.
            Ok(iterate_till_point(iter, slot, block_hash.as_slice()).map(Box::new)?)
        }
    }
}

pub fn get_tip(dir: &Path) -> Result<Point, Box<dyn std::error::Error>> {
    let names = build_stack_of_chunk_names(dir)?;

    let iter = ChunkReaders(
        dir.to_owned(),
        vec![names
            .into_iter()
            .next()
            .ok_or::<Box<dyn std::error::Error>>("Cannot find the tip".into())?],
    )
    .map_while(Result::ok)
    .flatten();
    let tip_data = iter
        .last()
        .ok_or::<Box<dyn std::error::Error>>("Cannot find the tip".into())??;

    let block = MultiEraBlock::decode(&tip_data)?;

    Ok(Point::Specific(block.slot(), block.hash().to_vec()))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pallas_network::miniprotocols::Point;
    use pallas_traverse::MultiEraBlock;
    use tracing::trace;

    #[test]
    fn chunk_binary_search_test() {
        use super::chunk_binary_search;

        let vec = vec![vec![7, 8, 9], vec![4, 5], vec![1, 2, 3]];
        let cmp = |chunk: &Vec<i32>, point: &i32| Ok(chunk[0].cmp(point));

        assert_eq!(chunk_binary_search(&vec, &0, cmp).unwrap(), None);
        assert_eq!(chunk_binary_search(&vec, &1, cmp).unwrap(), Some(2));
        assert_eq!(chunk_binary_search(&vec, &2, cmp).unwrap(), Some(2));
        assert_eq!(chunk_binary_search(&vec, &3, cmp).unwrap(), Some(2));
        assert_eq!(chunk_binary_search(&vec, &4, cmp).unwrap(), Some(1));
        assert_eq!(chunk_binary_search(&vec, &5, cmp).unwrap(), Some(1));
        assert_eq!(chunk_binary_search(&vec, &6, cmp).unwrap(), Some(1));
        assert_eq!(chunk_binary_search(&vec, &7, cmp).unwrap(), Some(0));
        assert_eq!(chunk_binary_search(&vec, &8, cmp).unwrap(), Some(0));
        assert_eq!(chunk_binary_search(&vec, &9, cmp).unwrap(), Some(0));
        assert_eq!(chunk_binary_search(&vec, &10, cmp).unwrap(), Some(0));
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

    #[test]
    fn get_tip_test() {
        let dir = Path::new("/Users/alexeypoghilenkov/Work/preprod-e112-i2170.593a95cee76541823a6a67b8b4d918006d767896c1a5da27a64efa3eb3f0c296/immutable");

        let tip = super::get_tip(dir).unwrap();
        assert_eq!(
            tip,
            Point::Specific(
                46894222,
                hex::decode("e8c8688ad8b71005cff5b4bf165dc657e8aeeb0a8076770fc6eca1007c5fee93")
                    .unwrap()
            )
        );
    }

    #[test]
    fn tmp_test() {
        let dir = Path::new("/Users/alexeypoghilenkov/Work/preprod-e112-i2170.593a95cee76541823a6a67b8b4d918006d767896c1a5da27a64efa3eb3f0c296/immutable");
        let reader = super::read_blocks_from_point(
            dir,
            Point::Specific(
                46894222,
                hex::decode("e8c8688ad8b71005cff5b4bf165dc657e8aeeb0a8076770fc6eca1007c5fee93")
                    .unwrap(),
            ),
        )
        .unwrap();

        for block in reader.take(10) {
            let block = block.unwrap();
            let block = MultiEraBlock::decode(&block).unwrap();
            println!(
                "hash: {}, era: {}, slot: {}, epoch: ({},{}), number: {}",
                block.hash(),
                block.era(),
                block.slot(),
                block
                    .epoch(&pallas_traverse::wellknown::GenesisValues::preprod())
                    .0,
                block
                    .epoch(&pallas_traverse::wellknown::GenesisValues::preprod())
                    .1,
                block.header().number(),
            );
        }
    }
}
