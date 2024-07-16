use std::{
    cmp::Ordering,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use pallas_traverse::MultiEraBlock;
use tap::Tap;
use tracing::debug;

pub mod chunk;
pub mod primary;
pub mod secondary;

// TODO: we should make Point accessible in some crate more generic that
// `network`.
pub type Point = pallas_network::miniprotocols::Point;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Cannot find block by the provided point: {0:?}")]
    CannotFindBlock(Point),
    #[error("Origin block is missing, provided truncated chain data")]
    OriginMissing,
    #[error("Cannot read directory, error: {0}")]
    CannotReadDir(std::io::Error),
    #[error("Cannot decode block, error: {0}")]
    CannotDecodeBlock(pallas_traverse::Error),
    #[error(transparent)]
    ChunkReadError(chunk::Error),
}

/// Performs a binary search of the given sorted chunks in descending order
/// and returns the index of the chunk which probably contains the point.
///
/// Current algorithm slightly modified from the original binary search.
/// It returns the index of the chunk in the `chunks` vector,
/// which could contain searching element **BUT** it may not.
/// It assumes that **EACH** chunk it is a sorted collection **BUT** in
/// ascending order, e.g. `vec![vec![7, 8, 9], vec![4, 5], vec![1, 2, 3]]` and
/// inside `cmp` function you will compare the first element of the chunk e.g.
/// `let cmp = |chunk: &Vec<i32>, point: &i32| chunk[0].cmp(point)`.
fn chunk_binary_search<ChunkT, PointT>(
    chunks: &[ChunkT],
    point: &PointT,
    cmp: impl Fn(&ChunkT, &PointT) -> Result<Ordering, Error>,
) -> Result<Option<usize>, Error> {
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

/// Iterates through the blocks until the given slot and block hash are reached.
/// Returns an iterator over the blocks if the specific block is found,
/// otherwise returns an error.
///
/// IFF the `block_hash` is zero length, then the search is "fuzzy",
/// meaning that it will return the first block whose slot is greater than or
/// equal to `slot`.  
///
/// Fuzzy Search allows a block to be found by an "expected slot#" without
/// knowing precisely which block is being retrieved.
fn iterate_till_point(
    iter: impl Iterator<Item = FallibleBlock>,
    slot: u64,
    block_hash: &[u8],
) -> Result<impl Iterator<Item = FallibleBlock>, Error> {
    let mut iter = iter.peekable();
    match iter.peek() {
        Some(Ok(block_data)) => {
            let mut block_data = block_data.clone();
            let mut block = MultiEraBlock::decode(&block_data).map_err(Error::CannotDecodeBlock)?;

            while block.slot() < slot {
                iter.next();

                match iter.peek() {
                    Some(Ok(data)) => {
                        block_data.clone_from(data);
                        block =
                            MultiEraBlock::decode(&block_data).map_err(Error::CannotDecodeBlock)?;
                    }
                    Some(Err(_)) | None => return Ok(iter),
                }
            }

            if (block_hash.is_empty() && block.slot() >= slot)
                || (block.hash().as_ref().eq(block_hash) && block.slot() == slot)
            {
                Ok(iter)
            } else {
                Err(Error::CannotFindBlock(Point::Specific(
                    slot,
                    block_hash.to_vec(),
                )))
            }
        }
        Some(Err(_)) | None => Ok(iter),
    }
}

fn build_stack_of_chunk_names(dir: &Path) -> Result<ChunkNameSack, Error> {
    let mut chunks = std::fs::read_dir(dir)
        .map_err(Error::CannotReadDir)?
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
    // According to this docs https://mithril.network/doc/glossary/#immutable-file-number,
    // the last chunk files are not really immutable.
    // So to preserve only immutable data the last chunk files are omitted.
    chunks.pop();
    chunks.reverse();

    Ok(chunks)
}

pub type Block = Vec<u8>;

pub type ChunkName = String;
pub type ChunkNameSack = Vec<ChunkName>;

pub struct ChunkReaders(PathBuf, ChunkNameSack);

impl Iterator for ChunkReaders {
    type Item = Result<chunk::Reader, chunk::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.1
            .pop()
            .tap(|name| debug!(name, "switched to new chunk"))
            .map(|name| chunk::read_blocks(&self.0, &name))
    }
}

pub type FallibleBlock = Result<Block, chunk::Error>;

pub fn read_blocks(dir: &Path) -> Result<impl Iterator<Item = FallibleBlock>, Error> {
    let names = build_stack_of_chunk_names(dir)?;

    let iter = ChunkReaders(dir.to_owned(), names)
        .map_while(Result::ok)
        .flatten();

    Ok(iter)
}

/// Returns an iterator over the chain from the given point if the specific
/// block is found, otherwise returns an error.
///
/// # Note:
///
/// If the given `point` is not the Origin, AND the BlockHash of the point is
/// zero length. then the search for the first block is "Fuzzy".
/// Only the Slot# of the point will be used and the first block to be returned
/// will be the block whose slot is >= the given slot# in the point.
///
/// This allows iteration to commence from a calculated slot# where the precise
/// block is unknown, and then continue iterating blocks after that point.
///
/// # Errors
///
/// * `Error::OriginMissing` - If the first block in the chain is not the
///   genesis block.
/// * `Error::CannotFindBlock` - If the specific block indicated by the `Point`
///   value is not found.
/// * `Error::CannotReadDir` - If the directory cannot be read.
/// * `Error::ChunkReadError` - Chunk read error.
/// * `Error::CannotDecodeBlock` - If the block cannot be decoded.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use std::error::Error;
/// use pallas_hardano::storage::immutable::{Point, read_blocks_from_point};
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let dir = Path::new("/path/to/blocks");
///     let point = Point::Origin;
///     
///     match read_blocks_from_point(dir, point) {
///         Ok(iterator) => {
///             for block in iterator {
///                 match block {
///                     Ok(block_data) => {
///                         println!("Block data: {:?}", block_data);
///                     }
///                     Err(error) => {
///                         println!("Error reading block: {:?}", error);
///                     }
///                 }
///             }
///         }
///         Err(error) => {
///             println!("Error reading blocks from point: {:?}", error);
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub fn read_blocks_from_point(
    dir: &Path,
    point: Point,
) -> Result<Box<dyn Iterator<Item = FallibleBlock> + Send + Sync>, Error> {
    let names = build_stack_of_chunk_names(dir)?;

    match point {
        // Establish iterator from the beginning of the chain
        Point::Origin => {
            let mut iter = ChunkReaders(dir.to_owned(), names)
                .map_while(Result::ok)
                .flatten()
                .peekable();

            // check the first block
            match iter.peek() {
                Some(Ok(block_data)) => {
                    let block =
                        MultiEraBlock::decode(block_data).map_err(Error::CannotDecodeBlock)?;
                    // check that the first block is genesis
                    if block.slot() == 0 && block.number() == 0 {
                        Ok(Box::new(iter))
                    } else {
                        Err(Error::OriginMissing)
                    }
                }
                Some(Err(_)) | None => Ok(Box::new(iter)),
            }
        }
        // Establish iterator from specific block
        Point::Specific(slot, block_hash) => {
            // Comparator function for binary search.
            // Takes the first block from the chunk
            // and compares block's slot with provided slot number
            let cmp = {
                |chunk_name: &String, point: &u64| {
                    let mut blocks =
                        chunk::read_blocks(dir, chunk_name).map_err(Error::ChunkReadError)?;

                    // Try to read the first block from the chunk
                    if let Some(block_data) = blocks.next() {
                        let block_data = block_data.map_err(Error::ChunkReadError)?;

                        let block =
                            MultiEraBlock::decode(&block_data).map_err(Error::CannotDecodeBlock)?;
                        Ok(block.slot().cmp(point))
                    } else {
                        Ok(Ordering::Greater)
                    }
                }
            };

            // Finds chunk index and creates a truncated `names` vector from the found
            // index.
            let names = chunk_binary_search(&names, &slot, cmp)?
                .map(|chunk_index| names[..chunk_index + 1].to_vec())
                .ok_or(Error::CannotFindBlock(Point::Specific(
                    slot,
                    block_hash.clone(),
                )))?;

            let iter = ChunkReaders(dir.to_owned(), names.clone())
                .map_while(Result::ok)
                .flatten()
                .peekable();

            // Iterates util the block is found by the provided `point`.
            Ok(iterate_till_point(iter, slot, block_hash.as_slice()).map(Box::new)?)
        }
    }
}

/// Retrieves the tip `Point` value for the given directory.
///
/// The function takes a directory path as input and returns the `Point` value
/// of the latest block if it exists, or `None` if there are no blocks in the
/// directory.
///
/// # Errors
///
/// * `Error::CannotReadDir` - If the directory cannot be read.
/// * `Error::ChunkReadError` - Chunk read error.
/// * `Error::CannotDecodeBlock` - If the block cannot be decoded.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use std::error::Error;
/// use pallas_hardano::storage::immutable::{Point, get_tip};
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let dir = Path::new("/path/to/blocks");
///     let tip = get_tip(dir)?;
///
///     match tip {
///         Some(point) => {
///             println!("The tip Point value is: {:?}", point);
///         }
///         None => {
///             println!("There are no blocks in the directory.");
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub fn get_tip(dir: &Path) -> Result<Option<Point>, Error> {
    match build_stack_of_chunk_names(dir)?.into_iter().next() {
        Some(name) => {
            let tip_point = ChunkReaders(dir.to_owned(), vec![name])
                .map_while(Result::ok)
                .flatten()
                .last()
                .transpose()
                .map_err(Error::ChunkReadError)?
                .map(|tip_data| {
                    MultiEraBlock::decode(&tip_data)
                        .map(|block| Point::Specific(block.slot(), block.hash().to_vec()))
                })
                .transpose()
                .map_err(Error::CannotDecodeBlock)?;
            Ok(tip_point)
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pallas_network::miniprotocols::Point;
    use pallas_traverse::MultiEraBlock;
    use tracing::{trace, warn};

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
    fn iterate_till_point_test() {
        use super::{iterate_till_point, read_blocks};

        let mut reader = read_blocks(Path::new("../test_data")).unwrap();
        let block = reader.next().unwrap().unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        assert_eq!(block.slot(), 27756007);
        assert_eq!(
            hex::encode(block.hash()),
            "230199f16ba0d935e60bf7288373fa01beaa1e20516c34a6481c2231e73a2fd1"
        );

        let reader = read_blocks(Path::new("../test_data")).unwrap();
        let mut reader = iterate_till_point(
            reader,
            27756199,
            hex::decode("3dcf4b00e32099b20c598fd90aed0060e77b1899e58645b9fe7b95a7ca9b306c")
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let block = reader.next().unwrap().unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        assert_eq!(block.slot(), 27756199);
        assert_eq!(
            hex::encode(block.hash()),
            "3dcf4b00e32099b20c598fd90aed0060e77b1899e58645b9fe7b95a7ca9b306c"
        );
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

        assert_eq!(count, 1777);
    }

    #[test]
    fn can_read_multiple_chunks_from_folder_at_specific_point() {
        let reader = super::read_blocks_from_point(
            Path::new("../test_data"),
            Point::Specific(
                27756007,
                hex::decode("230199f16ba0d935e60bf7288373fa01beaa1e20516c34a6481c2231e73a2fd1")
                    .unwrap(),
            ),
        )
        .unwrap();

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

        assert_eq!(count, 1777);
    }

    #[test]
    fn get_tip_test() {
        use super::get_tip;

        let tip = get_tip(Path::new("../test_data")).unwrap();
        assert_eq!(
            tip,
            Some(Point::Specific(
                39679163,
                hex::decode("53af88680ff3380814fdddc148caa1c6dbb89e5a30a5f6a439ee313424a14c55")
                    .unwrap()
            ))
        );

        let tip = get_tip(Path::new("..")).unwrap();
        assert_eq!(tip, None);
    }

    #[test]
    fn read_blocks_from_point_test() {
        use super::read_blocks_from_point;

        // first block from the chain
        let point = Point::Specific(
            27756007,
            hex::decode("230199f16ba0d935e60bf7288373fa01beaa1e20516c34a6481c2231e73a2fd1")
                .unwrap(),
        );
        let mut reader = read_blocks_from_point(Path::new("../test_data"), point.clone()).unwrap();
        let block = reader.next().unwrap().unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        assert_eq!(Point::Specific(block.slot(), block.hash().to_vec()), point);

        // below middle block
        let point = Point::Specific(
            27767113,
            hex::decode("bec3280e1f7e5803cb92e3649bdb0b385daed76aa6833b10cf10c6d6eeb243b3")
                .unwrap(),
        );
        let mut reader = read_blocks_from_point(Path::new("../test_data"), point.clone()).unwrap();
        let block = reader.next().unwrap().unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        assert_eq!(Point::Specific(block.slot(), block.hash().to_vec()), point);

        // middle block
        let point = Point::Specific(
            39658182,
            hex::decode("bc61c5eee23c879ccf855db3da281d6d0a8b8dacc02c19813e0b5b38df67f636")
                .unwrap(),
        );
        let mut reader = read_blocks_from_point(Path::new("../test_data"), point.clone()).unwrap();
        let block = reader.next().unwrap().unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        assert_eq!(Point::Specific(block.slot(), block.hash().to_vec()), point);

        // above middle block
        let point = Point::Specific(
            39668772,
            hex::decode("e0af9b05d8007a98935cc47fd8c27ecd034cf708318459631aee6240ea9900ad")
                .unwrap(),
        );
        let mut reader = read_blocks_from_point(Path::new("../test_data"), point.clone()).unwrap();
        let block = reader.next().unwrap().unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        assert_eq!(Point::Specific(block.slot(), block.hash().to_vec()), point);

        // tip
        let point = Point::Specific(
            39679163,
            hex::decode("53af88680ff3380814fdddc148caa1c6dbb89e5a30a5f6a439ee313424a14c55")
                .unwrap(),
        );
        let mut reader = read_blocks_from_point(Path::new("../test_data"), point.clone()).unwrap();
        let block = reader.next().unwrap().unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        assert_eq!(Point::Specific(block.slot(), block.hash().to_vec()), point);

        // Try to read an origin block
        assert!(matches!(
            read_blocks_from_point(Path::new("../test_data"), Point::Origin)
                .err()
                .unwrap(),
            super::Error::OriginMissing
        ));

        // Try to read from a non existing point
        let point = Point::Specific(0, vec![]);
        assert!(matches!(
            read_blocks_from_point(Path::new("../test_data"), point.clone())
                .err()
                .unwrap(),
            super::Error::CannotFindBlock(_)
        ));
    }

    fn read_full_snapshot(path: &Path) {
        let reader = super::read_blocks(&path).unwrap();

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

    // This test will inspect the /test_data/full_snapshots dir in the repo hoping
    // to find full mithril snapshots to process. Full snapshots are ignored in git
    // for obvious reasons. Is up to the developer to download and extract snapshots
    // into that folder. If no snapshots are available, this test will pass.
    #[test]
    fn read_available_full_snapshots() {
        tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )
        .unwrap();

        let path = Path::new("../test_data/full_snapshots");

        let dir = match std::fs::read_dir(path) {
            Ok(x) => x,
            Err(_) => {
                warn!("full_snapshot folder not available");
                return;
            }
        };

        for snapshot in dir {
            let snapshot = snapshot.unwrap();
            let immutable = snapshot.path().join("immutable");
            if immutable.is_dir() {
                read_full_snapshot(&immutable);
            }
        }
    }
}
