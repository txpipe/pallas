use futures_core::Stream;

use crate::kvtable::Error;

use super::{BlockHash, BlockSlot, Log, Store};

pub struct RollStream;

impl RollStream {
    pub fn intersect(
        store: Store,
        intersect: Vec<(BlockSlot, BlockHash)>,
    ) -> Result<impl Stream<Item = Result<Log, Error>>, Error> {
        let mut last_seq = store.find_wal_seq(&intersect)?;

        Ok(async_stream::try_stream! {
            let iter = store.crawl_after(last_seq);

            for entry in iter {
                let (seq, val) = entry?;

                yield val;
                last_seq = Some(seq);
            }

            loop {
                store.tip_change.notified().await;
                let iter = store.crawl_after(last_seq);

                for entry in iter {
                    let (seq, val) = entry?;

                    yield val;
                    last_seq = Some(seq);
                }
            }
        })
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

        let s = super::RollStream::intersect(db.clone(), vec![]).unwrap();

        pin_mut!(s);

        let evt = s.next().await;
        let evt = evt.unwrap().unwrap();
        assert!(evt.is_origin());

        for i in 0..=200 {
            let evt = s.next().await;
            let evt = evt.unwrap().unwrap();
            assert!(evt.is_apply());
            assert_eq!(evt.slot().unwrap(), i * 10);
        }

        background.abort();
        let _ = Store::destroy(path); //.unwrap();
    }
}
