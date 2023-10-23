use super::{BlockBody, BlockHash, BlockSlot, Store};

fn with_tmp_db<T>(op: fn(db: Store) -> T) {
    let path = tempfile::tempdir().unwrap().into_path();
    let db = Store::open(path.clone()).unwrap();

    op(db);

    Store::destroy(path).unwrap();
}

fn dummy_block(slot: u64) -> (BlockSlot, BlockHash, BlockBody) {
    let hash = pallas_crypto::hash::Hasher::<256>::hash(slot.to_be_bytes().as_slice());
    (slot, hash, slot.to_be_bytes().to_vec())
}

#[test]
fn test_roll_forward_blackbox() {
    with_tmp_db(|mut db| {
        let (slot, hash, body) = dummy_block(11);
        db.roll_forward(slot, hash, body.clone()).unwrap();

        // ensure block body is persisted
        let persisted = db.get_block(hash).unwrap().unwrap();
        assert_eq!(persisted, body);

        // ensure tip matches
        let (tip_slot, tip_hash) = db.find_tip().unwrap().unwrap();
        assert_eq!(tip_slot, slot);
        assert_eq!(tip_hash, hash);

        // ensure chain has item
        let (chain_slot, chain_hash) = db.crawl().next().unwrap().unwrap();
        assert_eq!(chain_slot, slot);
        assert_eq!(chain_hash, hash);
    });
}

#[test]
fn test_roll_back_blackbox() {
    with_tmp_db(|mut db| {
        for i in 0..=5 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        db.roll_back(20).unwrap();

        // ensure tip show rollback point
        let (tip_slot, _) = db.find_tip().unwrap().unwrap();
        assert_eq!(tip_slot, 20);

        // ensure chain has items not rolled back
        let mut chain = db.crawl();

        for i in 0..=2 {
            let (slot, _) = chain.next().unwrap().unwrap();
            assert_eq!(slot, i * 10);
        }

        // ensure chain stops here
        assert!(chain.next().is_none());
    });
}

//TODO: test rollback beyond K
//TODO: test rollback with unknown slot

#[test]
fn test_chain_page() {
    with_tmp_db(|mut db| {
        for i in 0..100 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        let mut chain = db.read_chain_page(200, 15);

        for i in 0..15 {
            let (slot, _) = chain.next().unwrap().unwrap();
            assert_eq!(200 + (i * 10), slot)
        }

        assert!(chain.next().is_none());
    });
}

#[test]
fn test_intersect_options() {
    with_tmp_db(|mut db| {
        for i in 0..200 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        let intersect = db.intersect_options(10).unwrap();

        let expected = vec![1990, 1970, 1930, 1850, 1690, 1370, 730];

        for (out, exp) in intersect.iter().zip(expected) {
            assert_eq!(out.0, exp);
        }
    });
}
