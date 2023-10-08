use pallas_crypto::hash::Hash;
use std::{path::Path, sync::Arc};
use tracing::warn;

use rocksdb::{Options, WriteBatch, DB};

use crate::kvtable::*;

type BlockSlot = u64;
type BlockHash = Hash<32>;
type BlockBody = Vec<u8>;

#[derive(Clone)]
pub struct Chain {
    db: Arc<DB>,
    pub tip_change: Arc<tokio::sync::Notify>,
}

pub struct BlockByHashKV;

// hash -> block cbor
impl KVTable<DBHash, DBBytes> for BlockByHashKV {
    const CF_NAME: &'static str = "BlockByHashKV";
}

// slot => block hash
pub struct HashBySlotKV;

impl KVTable<DBInt, DBHash> for HashBySlotKV {
    const CF_NAME: &'static str = "HashBySlotKV";
}

pub struct ChainIterator<'a>(pub EntryIterator<'a, DBInt, DBHash>);

impl Iterator for ChainIterator<'_> {
    type Item = Result<(u64, Hash<32>), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|v| v.map(|(seq, val)| (seq.0, val.0)))
    }
}
impl Chain {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let db = DB::open_cf(&opts, path, [BlockByHashKV::CF_NAME, HashBySlotKV::CF_NAME])
            .map_err(|_| Error::IO)?;

        let out = Self {
            db: Arc::new(db),
            tip_change: Arc::new(tokio::sync::Notify::new()),
        };

        Ok(out)
    }

    pub fn get_block(&self, hash: Hash<32>) -> Result<Option<BlockBody>, Error> {
        let dbval = BlockByHashKV::get_by_key(&self.db, DBHash(hash))?;
        Ok(dbval.map(|x| x.0))
    }

    pub fn roll_forward(
        &mut self,
        slot: BlockSlot,
        hash: BlockHash,
        body: BlockBody,
    ) -> Result<(), Error> {
        let mut batch = WriteBatch::default();

        // keep track of the new block body
        BlockByHashKV::stage_upsert(&self.db, DBHash(hash), DBBytes(body), &mut batch);

        // add new block to HashBySlotKV
        HashBySlotKV::stage_upsert(&self.db, DBInt(slot), DBHash(hash), &mut batch);

        self.db.write(batch).map_err(|_| Error::IO)?;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn roll_back(&mut self, until: BlockSlot) -> Result<(), Error> {
        let mut batch = WriteBatch::default();

        // remove rollback-ed blocks from HashBySlotKV
        let to_remove = HashBySlotKV::iter_keys_from(&self.db, DBInt(until)).skip(1);

        for key in to_remove {
            HashBySlotKV::stage_delete(&self.db, key?, &mut batch);
        }

        self.db.write(batch).map_err(|_| Error::IO)?;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn roll_back_origin(&mut self) -> Result<(), Error> {
        HashBySlotKV::reset(&self.db)?;
        BlockByHashKV::reset(&self.db)?;

        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn find_tip(&self) -> Result<Option<(BlockSlot, BlockHash)>, Error> {
        let mut iter = HashBySlotKV::iter_entries(&self.db, rocksdb::IteratorMode::End);

        if let Some(last) = iter.next() {
            let (slot, hash) = last?;
            Ok(Some((slot.0, hash.0)))
        } else {
            Ok(None)
        }
    }

    pub fn intersect_options(
        &self,
        max_items: usize,
    ) -> Result<Vec<(BlockSlot, BlockHash)>, Error> {
        let mut iter = HashBySlotKV::iter_entries(&self.db, rocksdb::IteratorMode::End)
            .filter_map(|res| res.ok())
            .map(|(k, v)| (k.0, v.0));

        let mut out = Vec::with_capacity(max_items);

        while let Some((slot, hash)) = iter.next() {
            out.push((slot, hash));

            if out.len() >= max_items {
                break;
            }

            // skip exponentially
            let skip = 2usize.pow(out.len() as u32) - 1;
            for _ in 0..skip {
                iter.next();
            }
        }

        Ok(out)
    }

    pub fn crawl_after(&self, slot: Option<u64>) -> ChainIterator {
        if let Some(slot) = slot {
            let slot = Box::<[u8]>::from(DBInt(slot));
            let from = rocksdb::IteratorMode::From(&slot, rocksdb::Direction::Forward);
            let mut iter = HashBySlotKV::iter_entries(&self.db, from);

            // skip current
            iter.next();

            ChainIterator(iter)
        } else {
            let from = rocksdb::IteratorMode::Start;
            let iter = HashBySlotKV::iter_entries(&self.db, from);
            ChainIterator(iter)
        }
    }

    pub fn crawl(&self) -> ChainIterator {
        self.crawl_after(None)
    }

    pub fn read_chain_page(
        &self,
        from: BlockSlot,
        len: usize,
    ) -> impl Iterator<Item = Result<(BlockSlot, BlockHash), Error>> + '_ {
        HashBySlotKV::iter_entries_from(&self.db, DBInt(from))
            .map(|res| res.map(|(x, y)| (x.0, y.0)))
            .take(len)
    }

    /// Iterator over chain between two points (inclusive)
    ///
    /// To use Origin as start point set `from` to None.
    ///
    /// Returns None if either point in range don't exist or `to` point is
    /// earlier in chain than `from`.
    pub fn read_chain_range(
        &self,
        from: Option<(BlockSlot, BlockHash)>,
        to: (BlockSlot, BlockHash),
    ) -> Result<Option<impl Iterator<Item = Result<(BlockSlot, BlockHash), Error>> + '_>, Error>
    {
        // TODO: We want to use a snapshot here to avoid race condition where
        // point is checked to be in the HashBySlotKV but it is rolled-back before we
        // create the iterator. Problem is `HashBySlotKV` etc must take `DB`, not
        // `Snapshot<DB>`, so maybe we need a new way of creating something like
        // a "KVTableSnapshot" in addition to the current "KVTable" type, which
        // has methods on snapshots, but here I was having issues as there is
        // no `cf` method on Snapshot but it is used is KVTable.

        // let snapshot = self.db.snapshot();

        // check p2 not before p1
        let p1_slot = if let Some((slot, _)) = from {
            if to.0 < slot {
                warn!("chain range end slot before start slot");
                return Ok(None);
            } else {
                slot
            }
        } else {
            0 // Use 0 as slot for Origin
        };

        // check p1 exists in HashBySlotKV if provided
        if let Some((slot, hash)) = from {
            match HashBySlotKV::get_by_key(&self.db, DBInt(slot))? {
                Some(DBHash(found_hash)) => {
                    if hash != found_hash {
                        warn!("chain range start hash mismatch");
                        return Ok(None);
                    }
                }
                None => {
                    warn!("chain range start slot not found");
                    return Ok(None);
                }
            }
        }

        // check p2 exists in HashBySlotKV
        match HashBySlotKV::get_by_key(&self.db, DBInt(to.0))? {
            Some(DBHash(found_hash)) => {
                if to.1 != found_hash {
                    warn!("chain range end hash mismatch");
                    return Ok(None);
                }
            }
            None => {
                warn!("chain range end slot not found");
                return Ok(None);
            }
        };

        // return iterator between p1 and p2 inclusive
        Ok(Some(
            HashBySlotKV::iter_entries_from(&self.db, DBInt(p1_slot))
                .map(|res| res.map(|(x, y)| (x.0, y.0)))
                .take_while(move |x| {
                    if let Ok((slot, _)) = x {
                        // iter returns None once point is after `to` slot
                        *slot <= to.0
                    } else {
                        false
                    }
                }),
        ))
    }

    /// Check if a point (pair of slot and block hash) exists in the
    /// HashBySlotKV
    pub fn chain_contains(&self, slot: BlockSlot, hash: &BlockHash) -> Result<bool, Error> {
        if let Some(DBHash(found)) = HashBySlotKV::get_by_key(&self.db, DBInt(slot))? {
            if found == *hash {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn destroy(path: impl AsRef<Path>) -> Result<(), Error> {
        DB::destroy(&Options::default(), path).map_err(|_| Error::IO)
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockBody, BlockHash, BlockSlot, Chain};

    fn with_tmp_db<T>(op: fn(db: Chain) -> T) {
        let path = tempfile::tempdir().unwrap().into_path();
        let db = Chain::open(path.clone()).unwrap();

        op(db);

        Chain::destroy(path).unwrap();
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
}
