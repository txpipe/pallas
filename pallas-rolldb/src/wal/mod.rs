use pallas::crypto::hash::Hash;
use rocksdb::{IteratorMode, WriteBatch, DB};
use rocksdb::{Options, WriteBatch, DB};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tracing::warn;

use self::wal::WalKV;

use super::kvtable::*;

pub mod stream;

use crate::storage::kvtable::*;

pub type Seq = u64;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum WalAction {
    Apply,
    Undo,
    Mark,
    Origin,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Value(WalAction, super::BlockSlot, super::BlockHash);

impl Value {
    pub fn origin() -> Self {
        Self(WalAction::Origin, 0, super::BlockHash::new([0; 32]))
    }

    pub fn into_apply(
        slot: impl Into<super::BlockSlot>,
        hash: impl Into<super::BlockHash>,
    ) -> Self {
        Self(WalAction::Apply, slot.into(), hash.into())
    }

    pub fn action(&self) -> WalAction {
        self.0
    }

    pub fn slot(&self) -> super::BlockSlot {
        self.1
    }

    pub fn hash(&self) -> &super::BlockHash {
        &self.2
    }

    pub fn into_undo(self) -> Option<Self> {
        match self.0 {
            WalAction::Apply => Some(Self(WalAction::Undo, self.1, self.2)),
            WalAction::Undo => None,
            WalAction::Mark => None,
            WalAction::Origin => None,
        }
    }

    pub fn into_mark(self) -> Option<Self> {
        match self.0 {
            WalAction::Apply => Some(Self(WalAction::Mark, self.1, self.2)),
            WalAction::Mark => Some(Self(WalAction::Mark, self.1, self.2)),
            WalAction::Origin => Some(Self(WalAction::Origin, self.1, self.2)),
            WalAction::Undo => None,
        }
    }

    pub fn is_apply(&self) -> bool {
        matches!(self.0, WalAction::Apply)
    }

    pub fn is_mark(&self) -> bool {
        matches!(self.0, WalAction::Mark)
    }

    pub fn is_undo(&self) -> bool {
        matches!(self.0, WalAction::Undo)
    }

    pub fn is_origin(&self) -> bool {
        matches!(self.0, WalAction::Origin)
    }
}

// slot => block hash
pub struct WalKV;

impl KVTable<DBInt, DBSerde<Value>> for WalKV {
    const CF_NAME: &'static str = "WalKV";
}

pub struct WalIterator<'a>(pub EntryIterator<'a, DBInt, DBSerde<Value>>);

impl Iterator for WalIterator<'_> {
    type Item = Result<(u64, Value), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|v| v.map(|(seq, val)| (seq.0, val.0)))
    }
}

impl WalKV {
    pub fn initialize(db: &DB) -> Result<Seq, Error> {
        if Self::is_empty(db) {
            Self::write_seed(db)?;
            Ok(0)
        } else {
            let last = Self::last_key(db)?.map(|x| x.0);
            Ok(last.unwrap())
        }
    }

    fn write_seed(db: &DB) -> Result<(), Error> {
        let mut batch = WriteBatch::default();
        let k = DBInt(0);
        let v = DBSerde(Value::origin());
        Self::stage_upsert(db, k, v, &mut batch);

        db.write(batch).map_err(|_| Error::IO)
    }

    fn stage_append(
        db: &DB,
        last_seq: Seq,
        value: Value,
        batch: &mut WriteBatch,
    ) -> Result<u64, super::Error> {
        let new_seq = last_seq + 1;

        Self::stage_upsert(db, DBInt(new_seq), DBSerde(value), batch);

        Ok(new_seq)
    }
}

#[derive(Clone)]
pub struct RollDB {
    db: Arc<DB>,
    pub tip_change: Arc<tokio::sync::Notify>,
    wal_seq: u64,
    k_param: u64,
}

impl Wal {
    pub fn open(path: impl AsRef<Path>, k_param: u64) -> Result<Self, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let db = DB::open_cf(&opts, path, [WalKV::CF_NAME]).map_err(|_| Error::IO)?;

        let wal_seq = wal::WalKV::initialize(&db)?;

        let out = Self {
            db: Arc::new(db),
            tip_change: Arc::new(tokio::sync::Notify::new()),
            wal_seq,
            k_param,
        };

        Ok(out)
    }

    pub fn roll_forward(
        &mut self,
        slot: BlockSlot,
        hash: BlockHash,
        body: BlockBody,
    ) -> Result<(), Error> {
        let mut batch = WriteBatch::default();

        // keep track of the new block body
        BlockKV::stage_upsert(&self.db, DBHash(hash), DBBytes(body), &mut batch);

        // advance the WAL to the new point
        let new_seq =
            WalKV::stage_append((db, last_seq, Value(WalAction::Apply, slot, hash), batch))?;

        self.db.write(batch).map_err(|_| Error::IO)?;
        self.wal_seq = new_seq;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn roll_back(&mut self, until: BlockSlot) -> Result<(), Error> {
        let mut batch = WriteBatch::default();

        let mut new_seq = self.wal_seq;

        let iter = WalKV::iter_values(db, IteratorMode::End);

        for step in iter {
            let value = step.map_err(|_| super::Error::IO)?.0;

            if value.slot() <= until {
                last_seq = Self::stage_append(db, last_seq, value.into_mark().unwrap(), batch)?;
                break;
            }

            match value.into_undo() {
                Some(undo) => {
                    last_seq = Self::stage_append(db, last_seq, undo, batch)?;
                }
                None => continue,
            };
        }

        self.db.write(batch).map_err(|_| Error::IO)?;
        self.wal_seq = new_seq;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn roll_back_origin(&mut self) -> Result<(), Error> {
        let mut batch = WriteBatch::default();

        let mut new_seq = self.wal_seq;

        let iter = WalKV::iter_values(db, IteratorMode::End);

        for step in iter {
            let value = step.map_err(|_| super::Error::IO)?.0;

            if value.is_origin() {
                break;
            }

            match value.into_undo() {
                Some(undo) => {
                    new_seq = Self::stage_append(db, last_seq, undo, batch)?;
                }
                None => continue,
            };
        }

        BlockKV::reset(&self.db)?;

        self.db.write(batch).map_err(|_| Error::IO)?;
        self.wal_seq = new_seq;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn find_tip(db: &DB) -> Result<Option<(super::BlockSlot, super::BlockHash)>, super::Error> {
        let iter = WalKV::iter_values(db, IteratorMode::End);

        for value in iter {
            if let Value(WalAction::Apply | WalAction::Mark, slot, hash) = value?.0 {
                return Ok(Some((slot, hash)));
            }
        }

        Ok(None)
    }

    pub fn intersect_options(
        &self,
        max_items: usize,
    ) -> Result<Vec<(BlockSlot, BlockHash)>, Error> {
        let mut iter = wal::WalKV::iter_values(&self.db, rocksdb::IteratorMode::End)
            .filter_map(|res| res.ok())
            .filter(|v| !v.is_undo());

        let mut out = Vec::with_capacity(max_items);

        // crawl the wal exponentially
        while let Some(val) = iter.next() {
            out.push((val.slot(), *val.hash()));

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

    pub fn crawl_after(&self, seq: Option<u64>) -> wal::WalIterator {
        if let Some(seq) = seq {
            let seq = Box::<[u8]>::from(DBInt(seq));
            let from = rocksdb::IteratorMode::From(&seq, rocksdb::Direction::Forward);
            let mut iter = wal::WalKV::iter_entries(&self.db, from);

            // skip current
            iter.next();

            wal::WalIterator(iter)
        } else {
            let from = rocksdb::IteratorMode::Start;
            let iter = wal::WalKV::iter_entries(&self.db, from);
            wal::WalIterator(iter)
        }
    }

    pub fn find_wal_seq(&self, block: Option<(BlockSlot, BlockHash)>) -> Result<wal::Seq, Error> {
        if block.is_none() {
            return Ok(0);
        }

        let (slot, hash) = block.unwrap();

        // TODO: Not sure this is 100% accurate:
        // i.e Apply(X), Apply(cursor), Undo(cursor), Mark(x)
        // We want to start at Apply(cursor) or Mark(cursor), but even then,
        // what if we have more than one Apply(cursor), how do we know
        // which is correct?
        let found = WalKV::scan_until(&self.db, rocksdb::IteratorMode::End, |v| {
            v.slot() == slot && v.hash().eq(&hash) && v.is_apply()
        })?;

        match found {
            Some(DBInt(seq)) => Ok(seq),
            None => Err(Error::NotFound),
        }
    }

    /// Prune the WAL of entries with slot values over `k_param` from the tip
    pub fn prune_wal(&self) -> Result<(), Error> {
        let tip = wal::WalKV::find_tip(&self.db)?
            .map(|(slot, _)| slot)
            .unwrap_or_default();

        // iterate through all values in Wal from start
        let mut iter = wal::WalKV::iter_entries(&self.db, rocksdb::IteratorMode::Start);

        let mut batch = WriteBatch::default();

        while let Some(Ok((wal_key, value))) = iter.next() {
            // get the number of slots that have passed since the wal point
            let slot_delta = tip - value.slot();

            if slot_delta <= self.k_param {
                break;
            } else {
                wal::WalKV::stage_delete(&self.db, wal_key, &mut batch);
            }
        }

        self.db.write(batch).map_err(|_| Error::IO)?;

        Ok(())
    }

    pub fn destroy(path: impl AsRef<Path>) -> Result<(), Error> {
        DB::destroy(&Options::default(), path).map_err(|_| Error::IO)
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockBody, BlockHash, BlockSlot, RollDB};

    fn with_tmp_db<T>(k_param: u64, op: fn(db: RollDB) -> T) {
        let path = tempfile::tempdir().unwrap().into_path();
        let db = RollDB::open(path.clone(), k_param).unwrap();

        op(db);

        RollDB::destroy(path).unwrap();
    }

    fn dummy_block(slot: u64) -> (BlockSlot, BlockHash, BlockBody) {
        let hash = pallas::crypto::hash::Hasher::<256>::hash(slot.to_be_bytes().as_slice());
        (slot, hash, slot.to_be_bytes().to_vec())
    }

    #[test]
    fn test_origin_event() {
        with_tmp_db(30, |db| {
            let mut iter = db.crawl_wal_after(None);

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
    fn test_roll_forward_blackbox() {
        with_tmp_db(30, |mut db| {
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
            let (chain_slot, chain_hash) = db.crawl_chain().next().unwrap().unwrap();
            assert_eq!(chain_slot, slot);
            assert_eq!(chain_hash, hash);
        });
    }

    #[test]
    fn test_roll_back_blackbox() {
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
            let mut chain = db.crawl_chain();

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
    fn test_prune_linear() {
        with_tmp_db(30, |mut db| {
            for i in 0..100 {
                let (slot, hash, body) = dummy_block(i * 10);
                db.roll_forward(slot, hash, body).unwrap();
            }

            db.prune_wal().unwrap();

            let mut chain = db.crawl_chain();

            for i in 0..100 {
                let (slot, _) = chain.next().unwrap().unwrap();
                assert_eq!(i * 10, slot)
            }

            assert!(chain.next().is_none());

            let mut wal = db.crawl_wal_after(None);

            for i in 96..100 {
                let (_, val) = wal.next().unwrap().unwrap();
                assert_eq!(val.slot(), i * 10);
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

            let mut chain = db.crawl_chain();

            for i in 0..=80 {
                let (slot, _) = chain.next().unwrap().unwrap();
                assert_eq!(i * 10, slot)
            }

            assert!(chain.next().is_none());

            let mut wal = db.crawl_wal_after(None);

            for i in 77..100 {
                let (_, val) = wal.next().unwrap().unwrap();
                assert!(val.is_apply());
                assert_eq!(val.slot(), i * 10);
            }

            for i in (81..100).rev() {
                let (_, val) = wal.next().unwrap().unwrap();
                assert!(val.is_undo());
                assert_eq!(val.slot(), i * 10);
            }

            let (_, val) = wal.next().unwrap().unwrap();
            assert!(val.is_mark());
            assert_eq!(val.slot(), 800);

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
}
