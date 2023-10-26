use super::{BlockBody, BlockHash, BlockSlot, Store};

fn with_tmp_db<T>(k_param: u64, op: fn(store: Store) -> T) {
    let path = tempfile::tempdir().unwrap().into_path();
    let store = Store::open(path.clone(), k_param).unwrap();

    op(store);

    Store::destroy(path).unwrap();
}

fn dummy_block(slot: u64) -> (BlockSlot, BlockHash, BlockBody) {
    let hash = pallas_crypto::hash::Hasher::<256>::hash(slot.to_be_bytes().as_slice());
    (slot, hash, slot.to_be_bytes().to_vec())
}

#[test]
fn test_origin_event() {
    with_tmp_db(30, |db| {
        let mut iter = db.crawl_after(None);

        let origin = iter.next();
        assert!(origin.is_some());

        let origin = origin.unwrap();
        assert!(origin.is_ok());

        let (seq, value) = origin.unwrap();
        assert_eq!(seq, 0);
        assert!(value.is_origin());
    });
}

#[test]
fn test_basic_append() {
    with_tmp_db(30, |mut db| {
        let (slot, hash, body) = dummy_block(11);
        db.roll_forward(slot, hash, body.clone()).unwrap();

        // ensure tip matches
        let (tip_slot, tip_hash) = db.find_tip().unwrap().unwrap();
        assert_eq!(tip_slot, slot);
        assert_eq!(tip_hash, hash);

        // ensure chain has item
        let mut iter = db.crawl_after(None);

        // skip origin
        iter.next();

        let (seq, log) = iter.next().unwrap().unwrap();
        assert_eq!(seq, 1);
        assert_eq!(log.slot().unwrap(), slot);
        assert_eq!(log.hash().unwrap(), &hash);
        assert_eq!(log.body().unwrap(), &body);
    });
}

#[test]
fn test_rollback_undos() {
    with_tmp_db(30, |mut db| {
        for i in 0..=5 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        db.roll_back(20).unwrap();

        // ensure tip show rollback point
        let (tip_slot, _) = db.find_tip().unwrap().unwrap();
        assert_eq!(tip_slot, 20);

        // ensure chain has items not rolled back
        let mut wal = db.crawl_after(None);

        let (seq, log) = wal.next().unwrap().unwrap();
        assert_eq!(seq, 0);
        assert!(log.is_origin());

        for i in 0..=5 {
            let (_, log) = wal.next().unwrap().unwrap();
            assert!(log.is_apply());
            assert_eq!(log.slot().unwrap(), i * 10);
        }

        for i in (3..=5).rev() {
            let (_, log) = wal.next().unwrap().unwrap();
            assert!(log.is_undo());
            assert_eq!(log.slot().unwrap(), i * 10);
        }

        let (_, log) = wal.next().unwrap().unwrap();
        assert!(log.is_mark());
        assert_eq!(log.slot().unwrap(), 20);

        // ensure chain stops here
        assert!(wal.next().is_none());
    });
}

//TODO: test rollback beyond K
//TODO: test rollback with unknown slot

#[test]
fn test_prune_linear() {
    with_tmp_db(30, |mut db| {
        for i in 0..100 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        db.prune_wal().unwrap();

        let mut wal = db.crawl_after(None);

        for i in 96..100 {
            let (_, val) = wal.next().unwrap().unwrap();
            assert_eq!(val.slot().unwrap(), i * 10);
        }

        assert!(wal.next().is_none());
    });
}

#[test]
fn test_prune_with_rollback() {
    with_tmp_db(30, |mut db| {
        for i in 0..100 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        db.roll_back(800).unwrap();

        // tip is 800 (Mark)

        db.prune_wal().unwrap();

        let mut wal = db.crawl_after(None);

        for i in 77..100 {
            let (_, val) = wal.next().unwrap().unwrap();
            assert!(val.is_apply());
            assert_eq!(val.slot().unwrap(), i * 10);
        }

        for i in (81..100).rev() {
            let (_, val) = wal.next().unwrap().unwrap();
            assert!(val.is_undo());
            assert_eq!(val.slot().unwrap(), i * 10);
        }

        let (_, val) = wal.next().unwrap().unwrap();
        assert!(val.is_mark());
        assert_eq!(val.slot().unwrap(), 800);

        assert!(wal.next().is_none());
    });
}

#[test]
fn test_intersect_options() {
    with_tmp_db(1000, |mut db| {
        for i in 0..200 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        db.prune_wal().unwrap();

        let intersect = db.intersect_options(10).unwrap();

        let expected = vec![1990, 1970, 1930, 1850, 1690, 1370, 980];

        for (out, exp) in intersect.iter().zip(expected) {
            assert_eq!(out.0, exp);
        }
    });
}
