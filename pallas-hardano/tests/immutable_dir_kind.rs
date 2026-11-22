//! The immutable reader's `DirKind`: `Live` leaves out the highest chunk,
//! `Certified` reads it too.

use std::path::{Path, PathBuf};

use pallas_hardano::storage::immutable::{
    DirKind, FallibleBlock, Point, chunk, get_tip, get_tip_with, read_blocks,
    read_blocks_from_point, read_blocks_from_point_with, read_blocks_with,
};
use pallas_traverse::MultiEraBlock;

const TEST_DATA: &str = "../test_data";

/// Copies the given chunks, with their index files, from `test_data/` into a
/// fresh directory of their own.
fn fixture_dir(name: &str, chunks: &[&str]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    for chunk in chunks {
        for ext in ["chunk", "primary", "secondary"] {
            let file = format!("{chunk}.{ext}");
            std::fs::copy(Path::new(TEST_DATA).join(&file), dir.join(&file)).unwrap();
        }
    }

    dir
}

fn points(iter: impl Iterator<Item = FallibleBlock>) -> Vec<Point> {
    iter.map(|block| {
        let block = block.unwrap();
        let block = MultiEraBlock::decode(&block).unwrap();
        Point::Specific(block.slot(), block.hash().to_vec())
    })
    .collect()
}

fn chunk_points(chunk: &str) -> Vec<Point> {
    points(chunk::read_blocks(Path::new(TEST_DATA), chunk).unwrap())
}

fn point(slot: u64, hash: &str) -> Point {
    Point::Specific(slot, hex::decode(hash).unwrap())
}

#[test]
fn two_chunks_live_reads_the_lower_one_only() {
    let dir = fixture_dir("two_chunks_live", &["01285", "01836"]);

    let blocks = points(read_blocks(&dir).unwrap());
    assert_eq!(blocks.len(), 864);
    assert_eq!(blocks, chunk_points("01285"));
    assert_eq!(
        points(read_blocks_with(&dir, DirKind::Live).unwrap()),
        blocks
    );

    let tip = point(
        27777565,
        "501a67d6b7d11ee12a69f87c3c799515af638620b123a11e668a39b8c17e42b6",
    );
    assert_eq!(get_tip(&dir).unwrap(), Some(tip.clone()));
    assert_eq!(get_tip_with(&dir, DirKind::Live).unwrap(), Some(tip));
}

#[test]
fn two_chunks_certified_reads_both_in_order() {
    let dir = fixture_dir("two_chunks_certified", &["01285", "01836"]);

    let blocks = points(read_blocks_with(&dir, DirKind::Certified).unwrap());
    let mut expected = chunk_points("01285");
    expected.extend(chunk_points("01836"));
    assert_eq!(blocks.len(), 864 + 913);
    assert_eq!(blocks, expected);

    let tip = point(
        39679163,
        "53af88680ff3380814fdddc148caa1c6dbb89e5a30a5f6a439ee313424a14c55",
    );
    assert_eq!(blocks.last(), Some(&tip));
    assert_eq!(get_tip_with(&dir, DirKind::Certified).unwrap(), Some(tip));
}

#[test]
fn certified_from_a_point_in_the_lower_chunk_reads_through_the_higher() {
    let dir = fixture_dir("from_lower_certified", &["01285", "01836"]);

    // block 432 of 864 in 01285
    let start = point(
        27766900,
        "ebce514df3a51ad42120de125768bdc8d45472c06a86f1f7c3a67f0e2605bc31",
    );
    let blocks =
        points(read_blocks_from_point_with(&dir, start.clone(), DirKind::Certified).unwrap());

    let mut expected = chunk_points("01285")[432..].to_vec();
    expected.extend(chunk_points("01836"));
    assert_eq!(blocks.first(), Some(&start));
    assert_eq!(blocks, expected);

    // live stops at the end of the lower chunk
    let live = points(read_blocks_from_point(&dir, start).unwrap());
    assert_eq!(live, chunk_points("01285")[432..].to_vec());
}

#[test]
fn from_a_point_in_the_highest_chunk() {
    let dir = fixture_dir("from_highest", &["01285", "01836"]);

    // block 456 of 913 in 01836
    let start = point(
        39668475,
        "b9ddb29428a563387e138ddfa8d9183301b4e3f5894722135b5ddaa0ed681ff4",
    );

    let certified =
        points(read_blocks_from_point_with(&dir, start.clone(), DirKind::Certified).unwrap());
    assert_eq!(certified.first(), Some(&start));
    assert_eq!(certified, chunk_points("01836")[456..].to_vec());

    // live never sees the highest chunk, so it reads past the end of the
    // lower one and yields nothing
    let live = points(read_blocks_from_point(&dir, start.clone()).unwrap());
    assert!(live.is_empty());
    let live = points(read_blocks_from_point_with(&dir, start, DirKind::Live).unwrap());
    assert!(live.is_empty());
}

#[test]
fn single_chunk() {
    let dir = fixture_dir("single_chunk", &["02019"]);

    assert!(points(read_blocks(&dir).unwrap()).is_empty());
    assert_eq!(get_tip(&dir).unwrap(), None);

    let tip = point(
        43610414,
        "80d02d3c9a576af65e4f36b95a57ae528b62c14836282fbd8d6a93aa1fef557f",
    );
    let blocks = points(read_blocks_with(&dir, DirKind::Certified).unwrap());
    assert_eq!(blocks, vec![tip.clone()]);
    assert_eq!(
        get_tip_with(&dir, DirKind::Certified).unwrap(),
        Some(tip.clone())
    );

    let from = points(read_blocks_from_point_with(&dir, tip.clone(), DirKind::Certified).unwrap());
    assert_eq!(from, vec![tip]);
}

#[test]
fn default_kind_is_live() {
    assert_eq!(DirKind::default(), DirKind::Live);
}
