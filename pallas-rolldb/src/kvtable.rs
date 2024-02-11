use pallas_crypto::hash::Hash;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    IO,

    #[error("serde error")]
    Serde,

    #[error("not found")]
    NotFound,
}

pub struct DBHash(pub Hash<32>);

impl From<Box<[u8]>> for DBHash {
    fn from(value: Box<[u8]>) -> Self {
        let inner: [u8; 32] = value[0..32].try_into().unwrap();
        let inner = Hash::<32>::from(inner);
        Self(inner)
    }
}

impl From<DBHash> for Box<[u8]> {
    fn from(value: DBHash) -> Self {
        let b = value.0.to_vec();
        b.into()
    }
}

impl From<Hash<32>> for DBHash {
    fn from(value: Hash<32>) -> Self {
        DBHash(value)
    }
}

impl From<DBHash> for Hash<32> {
    fn from(value: DBHash) -> Self {
        value.0
    }
}

pub struct DBInt(pub u64);

impl From<DBInt> for Box<[u8]> {
    fn from(value: DBInt) -> Self {
        let b = value.0.to_be_bytes();
        Box::new(b)
    }
}

impl From<Box<[u8]>> for DBInt {
    fn from(value: Box<[u8]>) -> Self {
        let inner: [u8; 8] = value[0..8].try_into().unwrap();
        let inner = u64::from_be_bytes(inner);
        Self(inner)
    }
}

impl From<u64> for DBInt {
    fn from(value: u64) -> Self {
        DBInt(value)
    }
}

impl From<DBInt> for u64 {
    fn from(value: DBInt) -> Self {
        value.0
    }
}

pub struct DBBytes(pub Vec<u8>);

impl From<DBBytes> for Box<[u8]> {
    fn from(value: DBBytes) -> Self {
        value.0.into()
    }
}

impl From<Box<[u8]>> for DBBytes {
    fn from(value: Box<[u8]>) -> Self {
        Self(value.into())
    }
}

impl<V> From<DBSerde<V>> for DBBytes
where
    V: Serialize,
{
    fn from(value: DBSerde<V>) -> Self {
        let inner = bincode::serialize(&value.0).unwrap();
        DBBytes(inner)
    }
}

#[derive(Debug)]
pub struct DBSerde<V>(pub V);

impl<V> std::ops::Deref for DBSerde<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> From<DBSerde<V>> for Box<[u8]>
where
    V: Serialize,
{
    fn from(v: DBSerde<V>) -> Self {
        bincode::serialize(&v.0)
            .map(|x| x.into_boxed_slice())
            .unwrap()
    }
}

impl<V> From<Box<[u8]>> for DBSerde<V>
where
    V: DeserializeOwned,
{
    fn from(value: Box<[u8]>) -> Self {
        let inner = bincode::deserialize(&value).unwrap();
        DBSerde(inner)
    }
}

impl<V> From<DBBytes> for DBSerde<V>
where
    V: DeserializeOwned,
{
    fn from(value: DBBytes) -> Self {
        let inner = bincode::deserialize(&value.0).unwrap();
        DBSerde(inner)
    }
}

impl<V> Clone for DBSerde<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct WithDBIntPrefix<T>(pub u64, pub T);

impl<T> From<WithDBIntPrefix<T>> for Box<[u8]>
where
    Box<[u8]>: From<T>,
{
    fn from(value: WithDBIntPrefix<T>) -> Self {
        let prefix: Box<[u8]> = DBInt(value.0).into();
        let after: Box<[u8]> = value.1.into();

        [prefix, after].concat().into()
    }
}

impl<T> From<Box<[u8]>> for WithDBIntPrefix<T> {
    fn from(_value: Box<[u8]>) -> Self {
        todo!()
    }
}

type RocksIterator<'a> = rocksdb::DBIteratorWithThreadMode<'a, rocksdb::DB>;

pub struct ValueIterator<'a, V>(RocksIterator<'a>, PhantomData<V>);

impl<'a, V> ValueIterator<'a, V> {
    pub fn new(inner: RocksIterator<'a>) -> Self {
        Self(inner, Default::default())
    }
}

impl<'a, V> Iterator for ValueIterator<'a, V>
where
    V: From<Box<[u8]>>,
{
    type Item = Result<V, Error>;

    fn next(&mut self) -> Option<Result<V, Error>> {
        match self.0.next() {
            Some(Ok((_, value))) => Some(Ok(V::from(value))),
            Some(Err(err)) => {
                tracing::error!(?err);
                Some(Err(Error::IO))
            }
            None => None,
        }
    }
}

pub struct KeyIterator<'a, K>(RocksIterator<'a>, PhantomData<K>);

impl<'a, K> KeyIterator<'a, K> {
    pub fn new(inner: RocksIterator<'a>) -> Self {
        Self(inner, Default::default())
    }
}

impl<'a, K> Iterator for KeyIterator<'a, K>
where
    K: From<Box<[u8]>>,
{
    type Item = Result<K, Error>;

    fn next(&mut self) -> Option<Result<K, Error>> {
        match self.0.next() {
            Some(Ok((key, _))) => Some(Ok(K::from(key))),
            Some(Err(err)) => {
                tracing::error!(?err);
                Some(Err(Error::IO))
            }
            None => None,
        }
    }
}

pub struct EntryIterator<'a, K, V>(RocksIterator<'a>, PhantomData<(K, V)>);

impl<'a, K, V> EntryIterator<'a, K, V> {
    pub fn new(inner: RocksIterator<'a>) -> Self {
        Self(inner, Default::default())
    }
}

impl<'a, K, V> Iterator for EntryIterator<'a, K, V>
where
    K: From<Box<[u8]>>,
    V: From<Box<[u8]>>,
{
    type Item = Result<(K, V), Error>;

    fn next(&mut self) -> Option<Result<(K, V), Error>> {
        match self.0.next() {
            Some(Ok((key, value))) => {
                let key_out = K::from(key);
                let value_out = V::from(value);

                Some(Ok((key_out, value_out)))
            }
            Some(Err(err)) => {
                tracing::error!(?err);
                Some(Err(Error::IO))
            }
            None => None,
        }
    }
}

pub trait KVTable<K, V>
where
    Box<[u8]>: From<K>,
    Box<[u8]>: From<V>,
    K: From<Box<[u8]>>,
    V: From<Box<[u8]>>,
{
    const CF_NAME: &'static str;

    fn cf(db: &rocksdb::DB) -> rocksdb::ColumnFamilyRef {
        db.cf_handle(Self::CF_NAME).unwrap()
    }

    fn reset(db: &rocksdb::DB) -> Result<(), Error> {
        db.drop_cf(Self::CF_NAME).map_err(|_| Error::IO)?;

        db.create_cf(Self::CF_NAME, &rocksdb::Options::default())
            .map_err(|_| Error::IO)?;

        Ok(())
    }

    fn get_by_key(db: &rocksdb::DB, k: K) -> Result<Option<V>, Error> {
        let cf = Self::cf(db);
        let raw_key = Box::<[u8]>::from(k);
        let raw_value = db
            .get_cf(&cf, raw_key)
            .map_err(|_| Error::IO)?
            .map(|x| Box::from(x.as_slice()));

        match raw_value {
            Some(x) => {
                let out = <V>::from(x);
                Ok(Some(out))
            }
            None => Ok(None),
        }
    }

    fn stage_upsert(db: &rocksdb::DB, k: K, v: V, batch: &mut rocksdb::WriteBatch) {
        let cf = Self::cf(db);

        let k_raw = Box::<[u8]>::from(k);
        let v_raw = Box::<[u8]>::from(v);

        batch.put_cf(&cf, k_raw, v_raw);
    }

    fn is_empty(db: &rocksdb::DB) -> bool {
        // HACK: can't find an easy way to size the num of keys, so we'll start an
        // iterator and see if we have at least one value. If someone know a better way
        // to accomplish this, please refactor.
        let mut iter = Self::iter_keys(db, rocksdb::IteratorMode::Start);
        iter.next().is_none()
    }

    fn iter_keys<'a>(db: &'a rocksdb::DB, mode: rocksdb::IteratorMode) -> KeyIterator<'a, K> {
        let cf = Self::cf(db);
        let inner = db.iterator_cf(&cf, mode);
        KeyIterator::new(inner)
    }

    fn iter_keys_start(db: &rocksdb::DB) -> KeyIterator<'_, K> {
        Self::iter_keys(db, rocksdb::IteratorMode::Start)
    }

    fn iter_keys_from(db: &rocksdb::DB, from: K) -> KeyIterator<'_, K> {
        let from_raw = Box::<[u8]>::from(from);
        let mode = rocksdb::IteratorMode::From(&from_raw, rocksdb::Direction::Forward);

        Self::iter_keys(db, mode)
    }

    fn iter_values<'a>(db: &'a rocksdb::DB, mode: rocksdb::IteratorMode) -> ValueIterator<'a, V> {
        let cf = Self::cf(db);
        let inner = db.iterator_cf(&cf, mode);
        ValueIterator::new(inner)
    }

    fn iter_values_start(db: &rocksdb::DB) -> ValueIterator<'_, V> {
        Self::iter_values(db, rocksdb::IteratorMode::Start)
    }

    fn iter_values_from(db: &rocksdb::DB, from: K) -> ValueIterator<'_, V> {
        let from_raw = Box::<[u8]>::from(from);
        let mode = rocksdb::IteratorMode::From(&from_raw, rocksdb::Direction::Forward);

        Self::iter_values(db, mode)
    }

    fn iter_entries<'a>(
        db: &'a rocksdb::DB,
        mode: rocksdb::IteratorMode,
    ) -> EntryIterator<'a, K, V> {
        let cf = Self::cf(db);
        let inner = db.iterator_cf(&cf, mode);
        EntryIterator::new(inner)
    }

    fn iter_entries_start(db: &rocksdb::DB) -> EntryIterator<'_, K, V> {
        Self::iter_entries(db, rocksdb::IteratorMode::Start)
    }

    fn iter_entries_from(db: &rocksdb::DB, from: K) -> EntryIterator<'_, K, V> {
        let from_raw = Box::<[u8]>::from(from);
        let mode = rocksdb::IteratorMode::From(&from_raw, rocksdb::Direction::Forward);

        Self::iter_entries(db, mode)
    }

    fn last_key(db: &rocksdb::DB) -> Result<Option<K>, Error> {
        let mut iter = Self::iter_keys(db, rocksdb::IteratorMode::End);

        match iter.next() {
            None => Ok(None),
            Some(x) => Ok(Some(x?)),
        }
    }

    fn last_value(db: &rocksdb::DB) -> Result<Option<V>, Error> {
        let mut iter = Self::iter_values(db, rocksdb::IteratorMode::End);

        match iter.next() {
            None => Ok(None),
            Some(x) => Ok(Some(x?)),
        }
    }

    fn last_entry(db: &rocksdb::DB) -> Result<Option<(K, V)>, Error> {
        let mut iter = Self::iter_entries(db, rocksdb::IteratorMode::End);

        match iter.next() {
            None => Ok(None),
            Some(x) => Ok(Some(x?)),
        }
    }

    fn scan_until<F>(db: &rocksdb::DB, mode: rocksdb::IteratorMode, predicate: F) -> Option<K>
    where
        F: Fn(&V) -> bool,
    {
        // TODO: Is flatten really safe? We are essentially skipping errors
        for (k, v) in Self::iter_entries(db, mode).flatten() {
            if predicate(&v) {
                return Some(k);
            }
        }

        None
    }

    fn iter_after_predicate<'a, F>(
        db: &'a rocksdb::DB,
        mode: rocksdb::IteratorMode<'a>,
        predicate: F,
    ) -> Result<Option<EntryIterator<'a, K, V>>, Error>
    where
        F: Fn(&V) -> bool,
    {
        let mut iter = Self::iter_entries(db, mode);

        while let Some(entry) = iter.next() {
            let (_, v) = entry?;
            if predicate(&v) {
                return Ok(Some(iter));
            }
        }

        Ok(None)
    }

    fn stage_delete(db: &rocksdb::DB, key: K, batch: &mut rocksdb::WriteBatch) {
        let cf = Self::cf(db);
        let k_raw = Box::<[u8]>::from(key);
        batch.delete_cf(&cf, k_raw);
    }
}
