use futures_core::Stream;

use crate::kvtable::Error;

use super::{BlockHash, BlockSlot, Log, Seq, Store};

pub struct RollStream;

impl RollStream {
    pub fn start_after(store: Store, seq: Option<Seq>) -> impl Stream<Item = Log> {
        async_stream::stream! {
            let mut last_seq = seq;

            let iter = store.crawl_after(last_seq);

            for (seq, val) in iter.flatten() {
                yield val;
                last_seq = Some(seq);
            }

            loop {
                store.tip_change.notified().await;
                let iter = store.crawl_after(last_seq);

                for (seq, val) in iter.flatten() {
                    yield val;
                    last_seq = Some(seq);
                }
            }
        }
    }

    /// Returns stream starting at first WAL entry after the most recent Apply
    /// or Mark action relating to the specified block, or None if no such entry
    /// found on WAL
    pub fn start_after_point(
        store: Store,
        block: (BlockSlot, BlockHash),
    ) -> impl Stream<Item = Result<Log, Error>> {
        async_stream::try_stream! {
            let mut last_seq = None;

            if let Some(iter) = store.crawl_after_point(block)? {
                for (seq, val) in iter.flatten() {
                    yield val;
                    last_seq = Some(seq);
                }

                loop {
                    store.tip_change.notified().await;
                    let iter = store.crawl_after(last_seq);

                    for (seq, val) in iter.flatten() {
                        yield val;
                        last_seq = Some(seq);
                    }
                }
            } else {
                Err(Error::NotFound)?
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{pin_mut, StreamExt};

    use crate::{
        kvtable,
        wal::{BlockBody, BlockHash, BlockSlot, Store},
    };

    fn dummy_block(slot: u64) -> (BlockSlot, BlockHash, BlockBody) {
        let hash = pallas_crypto::hash::Hasher::<256>::hash(slot.to_be_bytes().as_slice());
        (slot, hash, slot.to_be_bytes().to_vec())
    }

    #[tokio::test]
    async fn test_stream_waiting() {
        let path = tempfile::tempdir().unwrap().into_path();
        let mut db = Store::open(path.clone(), 30).unwrap();

        for i in 0..=100 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        let mut db2 = db.clone();
        let background = tokio::spawn(async move {
            for i in 101..=200 {
                let (slot, hash, body) = dummy_block(i * 10);
                db2.roll_forward(slot, hash, body).unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        });

        let s = super::RollStream::start_after(db.clone(), None);

        pin_mut!(s);

        let evt = s.next().await;
        let evt = evt.unwrap();
        assert!(evt.is_origin());

        for i in 0..=200 {
            let evt = s.next().await;
            let evt = evt.unwrap();
            assert!(evt.is_apply());
            assert_eq!(evt.slot().unwrap(), i * 10);
        }

        background.abort();
        let _ = Store::destroy(path); //.unwrap();
    }

    #[tokio::test]
    async fn test_stream_after_point() {
        let path = tempfile::tempdir().unwrap().into_path();
        let mut db = Store::open(path.clone(), 30).unwrap();

        for i in 0..=100 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        let mut db2 = db.clone();
        let background = tokio::spawn(async move {
            for i in 101..=200 {
                let (slot, hash, body) = dummy_block(i * 10);
                db2.roll_forward(slot, hash, body).unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        });

        let (intersect_slot, intersect_hash, _) = dummy_block(50 * 10);
        let (target_slot, target_hash, _) = dummy_block(51 * 10);

        let s = super::RollStream::start_after_point(db, (intersect_slot, intersect_hash));

        pin_mut!(s);

        let evt = s.next().await;
        let evt = evt.unwrap().unwrap();
        assert!(evt.is_apply());
        assert_eq!(evt.slot(), Some(target_slot));
        assert_eq!(evt.hash(), Some(&target_hash));

        background.abort();
        let _ = Store::destroy(path); //.unwrap();
    }

    #[tokio::test]
    async fn test_stream_after_point_missing() {
        let path = tempfile::tempdir().unwrap().into_path();
        let mut db = Store::open(path.clone(), 30).unwrap();

        for i in 0..=100 {
            let (slot, hash, body) = dummy_block(i * 10);
            db.roll_forward(slot, hash, body).unwrap();
        }

        let mut db2 = db.clone();
        let background = tokio::spawn(async move {
            for i in 101..=200 {
                let (slot, hash, body) = dummy_block(i * 10);
                db2.roll_forward(slot, hash, body).unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        });

        let (intersect_slot, intersect_hash, _) = dummy_block(500 * 10);

        let s = super::RollStream::start_after_point(db, (intersect_slot, intersect_hash));

        pin_mut!(s);

        let evt = s.next().await;
        let evt = evt.unwrap();
        assert!(matches!(evt, Err(kvtable::Error::NotFound)));

        background.abort();
        let _ = Store::destroy(path); //.unwrap();
    }
}
