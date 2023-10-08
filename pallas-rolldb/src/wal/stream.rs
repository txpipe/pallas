use futures_core::Stream;

use crate::prelude::RawBlock;

use super::RollDB;

type Item = super::wal::Value;
type ItemWithBlock = (super::wal::Value, RawBlock);

pub struct RollStream;

impl RollStream {
    pub fn start_after(db: RollDB, seq: Option<super::wal::Seq>) -> impl Stream<Item = Item> {
        async_stream::stream! {
            let mut last_seq = seq;

            let iter = db.crawl_wal_after(last_seq);

            for (seq, val) in iter.flatten() {
                yield val;
                last_seq = Some(seq);
            }

            loop {
                db.tip_change.notified().await;
                let iter = db.crawl_wal_after(last_seq);

                for (seq, val) in iter.flatten() {
                    yield val;
                    last_seq = Some(seq);
                }
            }
        }
    }

    pub fn start_after_with_block(
        db: RollDB,
        seq: Option<super::wal::Seq>,
    ) -> impl Stream<Item = ItemWithBlock> {
        async_stream::stream! {
            let mut last_seq = seq;

            let iter = db.crawl_wal_after(last_seq);

            for x in iter {
                let x = x.and_then(|(s,v)| {
                    let b = db.get_block(*v.hash())?.unwrap();
                    Ok((s,v,b))
                });

                if let Ok((seq, val, blo)) = x {
                    yield (val, blo);
                    last_seq = Some(seq);
                }
            }

            loop {
                db.tip_change.notified().await;
                let iter = db.crawl_wal_after(last_seq);

                for x in iter {
                    let x = x.and_then(|(s,v)| {
                        let b = db.get_block(*v.hash())?.unwrap();
                        Ok((s,v,b))
                    });

                    if let Ok((seq, val, blo)) = x {
                        yield (val, blo);
                        last_seq = Some(seq);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{pin_mut, StreamExt};

    use crate::storage::rolldb::{BlockBody, BlockHash, BlockSlot};

    fn dummy_block(slot: u64) -> (BlockSlot, BlockHash, BlockBody) {
        let hash = pallas::crypto::hash::Hasher::<256>::hash(slot.to_be_bytes().as_slice());
        (slot, hash, slot.to_be_bytes().to_vec())
    }

    #[tokio::test]
    async fn test_stream_waiting() {
        let path = tempfile::tempdir().unwrap().into_path();
        let mut db = super::RollDB::open(path.clone(), 30).unwrap();

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
        assert_eq!(evt.slot(), 0);

        for i in 0..=200 {
            let evt = s.next().await;
            let evt = evt.unwrap();
            assert!(evt.is_apply());
            assert_eq!(evt.slot(), i * 10);
        }

        background.abort();
        let _ = super::RollDB::destroy(path); //.unwrap();
    }
}
