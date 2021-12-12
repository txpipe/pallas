use std::{ops::Deref};

use minicbor::{Decode, Encode};

/// Custom collection to ensure ordered pairs of values
///
/// Since the ordering of the entries requires a particular order to maintain
/// canonicalization for isomorphic decoding / encoding operators, we use a Vec
/// as the underlaying struct for storage of the items (as opposed to a BTreeMap
/// or HashMap).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyValuePairs<K, V>(Vec<(K, V)>);

impl<K, V> Deref for KeyValuePairs<K, V> {
    type Target = Vec<(K, V)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'b, K, V> minicbor::decode::Decode<'b> for KeyValuePairs<K, V>
where
    K: Encode + Decode<'b>,
    V: Encode + Decode<'b>,
{
    fn decode(d: &mut minicbor::Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let items: Result<Vec<_>, _> = d.map_iter::<K, V>()?.collect();
        let items = items?;
        Ok(KeyValuePairs(items))
    }
}

impl<K, V> minicbor::encode::Encode for KeyValuePairs<K, V>
where
    K: Encode,
    V: Encode,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(self.0.len() as u64)?;
        for (k, v) in &self.0 {
            k.encode(e)?;
            v.encode(e)?;
        }

        Ok(())
    }
}
