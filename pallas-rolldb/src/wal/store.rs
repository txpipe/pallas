use rocksdb::Options;
use rocksdb::{IteratorMode, WriteBatch, DB};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

use crate::kvtable::*;

use super::{BlockBody, BlockHash, BlockSlot, Seq};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Log {
    Apply(BlockSlot, BlockHash, BlockBody),
    Undo(BlockSlot, BlockHash, BlockBody),
    Mark(BlockSlot, BlockHash, BlockBody),
    Origin,
}

impl Log {
    pub fn into_apply(
        slot: impl Into<BlockSlot>,
        hash: impl Into<BlockHash>,
        block: impl Into<BlockBody>,
    ) -> Self {
        Self::Apply(slot.into(), hash.into(), block.into())
    }

    pub fn slot(&self) -> Option<BlockSlot> {
        match self {
            Log::Apply(s, _, _) => Some(*s),
            Log::Undo(s, _, _) => Some(*s),
            Log::Mark(s, _, _) => Some(*s),
            Log::Origin => None,
        }
    }

    pub fn hash(&self) -> Option<&BlockHash> {
        match self {
            Log::Apply(_, h, _) => Some(h),
            Log::Undo(_, h, _) => Some(h),
            Log::Mark(_, h, _) => Some(h),
            Log::Origin => None,
        }
    }

    pub fn body(&self) -> Option<&BlockBody> {
        match self {
            Log::Apply(_, _, b) => Some(b),
            Log::Undo(_, _, b) => Some(b),
            Log::Mark(_, _, b) => Some(b),
            Log::Origin => None,
        }
    }

    pub fn into_undo(self) -> Option<Self> {
        match self {
            Self::Apply(s, h, b) => Some(Self::Undo(s, h, b)),
            _ => None,
        }
    }

    pub fn into_mark(self) -> Option<Self> {
        match self {
            Log::Apply(s, h, b) => Some(Log::Mark(s, h, b)),
            Log::Mark(s, h, b) => Some(Log::Mark(s, h, b)),
            Log::Origin => Some(Log::Origin),
            Log::Undo(..) => None,
        }
    }

    pub fn is_apply(&self) -> bool {
        matches!(self, Log::Apply(..))
    }

    pub fn is_mark(&self) -> bool {
        matches!(self, Log::Mark(..))
    }

    pub fn is_undo(&self) -> bool {
        matches!(self, Log::Undo(..))
    }

    pub fn is_origin(&self) -> bool {
        matches!(self, Log::Origin)
    }
}

// slot => block hash
pub struct WalKV;

impl KVTable<DBInt, DBSerde<Log>> for WalKV {
    const CF_NAME: &'static str = "WalKV";
}

pub struct WalIterator<'a>(pub EntryIterator<'a, DBInt, DBSerde<Log>>);

impl Iterator for WalIterator<'_> {
    type Item = Result<(u64, Log), Error>;

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
        let v = DBSerde(Log::Origin);
        Self::stage_upsert(db, k, v, &mut batch);

        db.write(batch).map_err(|_| Error::IO)
    }
}

pub struct RollBatch<'a>(&'a DB, WriteBatch, Seq);

impl<'a> RollBatch<'a> {
    fn new(db: &'a DB, last_seq: Seq) -> Self {
        Self(db, Default::default(), last_seq)
    }

    fn stage_append(&mut self, log: Log) {
        let new_seq = self.2 + 1;
        WalKV::stage_upsert(&self.0, DBInt(new_seq), DBSerde(log), &mut self.1);
        self.2 = new_seq;
    }

    fn apply(self) -> Result<Seq, Error> {
        self.0.write(self.1).map_err(|_| Error::IO)?;
        Ok(self.2)
    }
}

#[derive(Clone)]
pub struct Store {
    db: Arc<DB>,
    pub tip_change: Arc<tokio::sync::Notify>,
    wal_seq: u64,
    k_param: u64,
}

impl Store {
    pub fn open(path: impl AsRef<Path>, k_param: u64) -> Result<Self, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let db = DB::open_cf(&opts, path, [WalKV::CF_NAME]).map_err(|_| Error::IO)?;

        let wal_seq = WalKV::initialize(&db)?;

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
        let mut batch = RollBatch::new(&mut self.db, self.wal_seq);

        batch.stage_append(Log::Apply(slot, hash, body));

        self.wal_seq = batch.apply()?;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn roll_back(&mut self, until: BlockSlot) -> Result<(), Error> {
        let mut batch = RollBatch::new(&self.db, self.wal_seq);

        let iter = WalKV::iter_values(&self.db, IteratorMode::End);

        for step in iter {
            let value = step.map_err(|_| Error::IO)?.0;

            if value.slot().unwrap_or(0) <= until {
                batch.stage_append(value.into_mark().unwrap());
                break;
            }

            match value.into_undo() {
                Some(undo) => {
                    batch.stage_append(undo);
                }
                None => continue,
            };
        }

        self.wal_seq = batch.apply()?;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn roll_back_origin(&mut self) -> Result<(), Error> {
        let mut batch = RollBatch::new(&self.db, self.wal_seq);

        let iter = WalKV::iter_values(&self.db, IteratorMode::End);

        for step in iter {
            let value = step.map_err(|_| Error::IO)?.0;

            if value.is_origin() {
                break;
            }

            match value.into_undo() {
                Some(undo) => {
                    batch.stage_append(undo);
                }
                None => continue,
            };
        }

        self.wal_seq = batch.apply()?;
        self.tip_change.notify_waiters();

        Ok(())
    }

    pub fn find_tip(&self) -> Result<Option<(BlockSlot, BlockHash)>, Error> {
        let iter = WalKV::iter_values(&self.db, IteratorMode::End);

        for value in iter {
            let value = value?;

            if value.is_apply() || value.is_mark() {
                let slot = value.slot().unwrap();
                let hash = *value.hash().unwrap();
                return Ok(Some((slot, hash)));
            }
        }

        Ok(None)
    }

    pub fn intersect_options(
        &self,
        max_items: usize,
    ) -> Result<Vec<(BlockSlot, BlockHash)>, Error> {
        let mut iter = WalKV::iter_values(&self.db, rocksdb::IteratorMode::End)
            .filter_map(|res| res.ok())
            .filter(|v| !v.is_undo());

        let mut out = Vec::with_capacity(max_items);

        // crawl the wal exponentially
        while let Some(val) = iter.next() {
            if !val.is_apply() && !val.is_mark() {
                continue;
            }

            out.push((val.slot().unwrap(), *val.hash().unwrap()));

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

    pub fn crawl_after(&self, seq: Option<u64>) -> WalIterator {
        if let Some(seq) = seq {
            let seq = Box::<[u8]>::from(DBInt(seq));
            let from = rocksdb::IteratorMode::From(&seq, rocksdb::Direction::Forward);
            let mut iter = WalKV::iter_entries(&self.db, from);

            // skip current
            iter.next();

            WalIterator(iter)
        } else {
            let from = rocksdb::IteratorMode::Start;
            let iter = WalKV::iter_entries(&self.db, from);
            WalIterator(iter)
        }
    }

    pub fn find_wal_seq(&self, block: Option<(BlockSlot, BlockHash)>) -> Result<Seq, Error> {
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
            (v.is_mark() || v.is_apply())
                && v.slot().is_some_and(|s| s == slot)
                && v.hash().is_some_and(|h| h.eq(&hash))
        })?;

        match found {
            Some(DBInt(seq)) => Ok(seq),
            None => Err(Error::NotFound),
        }
    }

    /// Prune the WAL of entries with slot values over `k_param` from the tip
    pub fn prune_wal(&self) -> Result<(), Error> {
        let tip = self.find_tip()?.map(|(slot, _)| slot).unwrap_or_default();

        // iterate through all values in Wal from start
        let mut iter = WalKV::iter_entries(&self.db, rocksdb::IteratorMode::Start);

        let mut batch = WriteBatch::default();

        while let Some(Ok((wal_key, value))) = iter.next() {
            // get the number of slots that have passed since the wal point
            let slot_delta = tip - value.slot().unwrap_or(0);

            if slot_delta <= self.k_param {
                break;
            } else {
                WalKV::stage_delete(&self.db, wal_key, &mut batch);
            }
        }

        self.db.write(batch).map_err(|_| Error::IO)?;

        Ok(())
    }

    pub fn destroy(path: impl AsRef<Path>) -> Result<(), Error> {
        DB::destroy(&Options::default(), path).map_err(|_| Error::IO)
    }
}
