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

    pub fn start_from_point(
        store: Store,
        block: (BlockSlot, BlockHash),
    ) -> impl Stream<Item = Result<Log, Error>> {
        let (slot, hash) = block;

        async_stream::try_stream! {
            // find seq for point on WAL, or return not found
            if let Some(wal_seq) = store.find_wal_seq(block) {
                let mut last_seq = wal_seq;
                let mut iter = store.crawl_from(Some(wal_seq));

                // yield NotFound if found intersect WAL seq no longer on WAL
                let (_, val) = iter.next().ok_or(Error::NotFound)??;

                if (val.is_apply() || val.is_mark()) && (val.slot() == Some(slot)) && (val.hash() == Some(&hash.into())) {
                    // first, yield the intersect point entry
                    yield val;

                    // then the rest of the iterator
                    for entry in iter {
                        let (seq, val) = entry?;
                        yield val;
                        last_seq = seq;
                    }

                    loop {
                        store.tip_change.notified().await;
                        // TODO: not safe
                        let iter = store.crawl_after(Some(last_seq));

                        for entry in iter {
                            let (seq, val) = entry?;

                            yield val;
                            last_seq = seq
                        }
                    }
                } else {
                    // yield NotFound if intersect not found on iterator created with found WAL seq
                    Err(Error::NotFound)?
                }
            } else {
                // yield NotFound if no intersect with WAL found (no seq for point)
                Err(Error::NotFound)?
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{pin_mut, StreamExt};

    use crate::wal::{BlockBody, BlockHash, BlockSlot, Store};

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
}
