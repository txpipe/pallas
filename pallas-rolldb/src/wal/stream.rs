use futures_core::Stream;

use crate::kvtable::Error;

use super::{Log, Store};

pub struct RollStream;

impl RollStream {
    pub fn stream_wal(
        store: Store,
        last_wal_seq: Option<u64>,
    ) -> impl Stream<Item = Result<Log, Error>> {
        async_stream::try_stream! {
            let mut last_seq = last_wal_seq;

            let iter = store.crawl_after(last_wal_seq);

            for entry in iter {
                let (wal_seq, log) = entry?;

                if let Some(prev_seq) = last_seq {
                    if wal_seq != (prev_seq + 1) {
                        Err(Error::UnexpectedWalSeq(prev_seq + 1, wal_seq))?
                    }
                };

                yield log;
                last_seq = Some(wal_seq);
            }

            loop {
                store.tip_change.notified().await;
                let iter = store.crawl_after(last_seq);

                for entry in iter {
                    let (wal_seq, log) = entry?;

                    if let Some(prev_seq) = last_seq {
                        if wal_seq != (prev_seq + 1) {
                            Err(Error::UnexpectedWalSeq(prev_seq + 1, wal_seq))?
                        }
                    };

                    yield log;
                    last_seq = Some(wal_seq);
                }
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

        let s = super::RollStream::stream_wal(db.clone(), None);

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
