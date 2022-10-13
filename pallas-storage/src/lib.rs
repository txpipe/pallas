use std::{
    convert::{TryFrom, TryInto},
    path::Path,
};

use rocksdb::{Direction, IteratorMode, DB};
use thiserror::Error;

#[derive(Error, Debug)]
enum Error {
    #[error("serde error {0}")]
    Serde(String),

    #[error("storage error {0}")]
    Storage(String),
}

impl Error {
    fn serde(error: impl ToString) -> Self {
        Self::Serde(error.to_string())
    }

    fn storage(error: impl ToString) -> Self {
        Self::Storage(error.to_string())
    }
}

type Slot = u64;
type Hash = String;
type Point = (Slot, Hash);
type RawKV = (Box<[u8]>, Box<[u8]>);

type RocksIterator<'a> = rocksdb::DBIteratorWithThreadMode<'a, rocksdb::DB>;

struct Iterator<'a>(RocksIterator<'a>);

impl<'a> Iterator<'a> {
    pub fn next(&mut self) -> Option<Result<Entry, Error>> {
        match self.0.next() {
            Some(Ok(kv)) => Some(Entry::try_from(kv)),
            Some(Err(err)) => Some(Err(Error::storage(err))),
            None => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SeqNo(u64);

impl SeqNo {
    fn inc(&self) -> Self {
        SeqNo(self.0 + 1)
    }
}

impl TryFrom<Box<[u8]>> for SeqNo {
    type Error = Error;

    fn try_from(value: Box<[u8]>) -> Result<Self, Self::Error> {
        let value = <[u8; 8]>::try_from(value.as_ref()).map_err(Error::serde)?;
        let value = u64::from_be_bytes(value);
        Ok(SeqNo(value))
    }
}

impl From<SeqNo> for [u8; 8] {
    fn from(x: SeqNo) -> Self {
        x.0.to_be_bytes()
    }
}

#[derive(Debug)]
struct Value(bool, Point);

impl TryFrom<Box<[u8]>> for Value {
    type Error = Error;

    fn try_from(value: Box<[u8]>) -> Result<Self, Self::Error> {
        let (is_apply, point): (bool, Point) = bincode::deserialize(value.as_ref()).map_err(Error::serde)?;
        Ok(Value(is_apply, point))
    }
}

impl TryInto<Vec<u8>> for Value {
    type Error = Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        let inner = (self.0, self.1);
        bincode::serialize(&inner).map_err(Error::serde)
    }
}

#[derive(Debug)]
struct Entry(SeqNo, Value);

impl TryFrom<RawKV> for Entry {
    type Error = Error;

    fn try_from((k, v): RawKV) -> Result<Self, Self::Error> {
        let seq = SeqNo::try_from(k)?;
        let value = Value::try_from(v)?;

        Ok(Entry(seq, value))
    }
}

struct RollDB {
    db: DB,
    seq: SeqNo,
}

/// Apply(slot0,hashA)
/// Apply(slot1,hashB)
/// Apply(slot2,hashC)
/// Apply(slot3,hashD)
/// <= rollback to slot1,hash1 requested
/// Undo(slot3,hashD)
/// Undo(slot2,hashC)
/// Apply(slot2,hashX)
/// Apply(slot3,hashY)
/// Apply(slot4,hashZ)

impl RollDB {
    fn find_last_seq(db: &DB) -> Result<Option<SeqNo>, Error> {
        match db.iterator(IteratorMode::End).next() {
            Some(Ok((k, _))) => Ok(Some(SeqNo::try_from(k)?)),
            Some(Err(err)) => Err(Error::storage(err)),
            None => Ok(None),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let db = DB::open_default(path).map_err(Error::storage)?;
        let seq = Self::find_last_seq(&db)?.unwrap_or_default();
        Ok(Self { db, seq })
    }

    /// Extends the queue with a newly received block
    pub fn extend(&mut self, point: Point) -> Result<(), Error> {
        let new_seq = self.seq.inc();
        let key: [u8; 8] = new_seq.into();
        let value: Vec<u8> = Value(true, point).try_into()?;

        self.db
            .put(key, value)
            .map_err(Error::storage)
            .and_then(|_| {
                self.seq = new_seq;
                Ok(())
            })
    }

    /// Reverts blocks since a certain point
    pub fn undo(&mut self, since: Point) -> Result<(), Error> {
        let mut iter = self.db.iterator(IteratorMode::End);

        while let Some(Ok((k, v))) = iter.next() {
            let v = Value::try_from(v)
        }

        Ok(())
    }

    /// Clears blocks since a certain point
    pub fn rollback(since: Point) -> Result<(), Error> {
        todo!()
    }

    /// Returns the next entry in the queue
    ///
    /// Uses as stateful (but volatile) iterator over the messages in the queue.
    /// Each call will return the next entry to be processed. If the RollDB
    /// instance is restarted, this iterator will start from the beininig of the
    /// queue, yielding all non-committed entries.
    pub fn read(&self, since: SeqNo) -> Iterator {
        let k: [u8; 8] = since.into();
        let inner = self.db.iterator(IteratorMode::From(&k, Direction::Forward));
        Iterator(inner)
    }

    /// Marks entries as done until certain point
    ///
    /// This should be called when the downstream process has finished
    /// processing up to a certain point. This operation will remove points from
    /// the queue so that they aren't processed again.
    fn commit(&mut self, until: Entry) -> Result<(), Error> {
        todo!()
    }
}

#[test]
fn test_rocks() {
    // NB: db is automatically closed at end of lifetime

    let mut db = RollDB::open("./tmp2").unwrap();

    db.extend((0, "abc".into())).unwrap();
    db.extend((1, "def".into())).unwrap();
    db.extend((2, "ghi".into())).unwrap();

    {
        let mut iter = db.read(SeqNo(0));

        while let Some(point) = iter.next() {
            dbg!(point);
        }
    }

    db.extend((3, "jkm".into())).unwrap();
    db.extend((4, "nop".into())).unwrap();
    db.extend((5, "qrs".into())).unwrap();

    {
        let mut iter = db.read(SeqNo(2));

        while let Some(point) = iter.next() {
            dbg!(point);
        }
    }

    //let _ = DB::destroy(&Options::default(), path);
}
