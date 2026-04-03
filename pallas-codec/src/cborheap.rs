use std::ops::Deref;

use arrayvec::ArrayVec;
use minicbor::{Decode, Encode};
use slotmap::DefaultKey;

pub trait CborHeapContext {
    fn key(&self) -> Option<DefaultKey>;
}

impl CborHeapContext for () {
    fn key(&self) -> Option<DefaultKey> {
        None
    }
}

impl CborHeapContext for DefaultKey {
    fn key(&self) -> Option<DefaultKey> {
        Some(*self)
    }
}

pub struct CborHeap<const ITEM_SIZE: usize> {
    heap: slotmap::SlotMap<DefaultKey, ArrayVec<u8, ITEM_SIZE>>,
}

impl<const ITEM_SIZE: usize> CborHeap<ITEM_SIZE> {
    pub fn new(capacity: usize) -> Self {
        Self {
            heap: slotmap::SlotMap::with_capacity(capacity),
        }
    }

    fn get_slice(&self, ref_: &CborRef) -> Option<&[u8]> {
        self.heap
            .get(ref_.0)
            .map(|entry| entry.as_slice())
            .and_then(|slice| slice.get(ref_.1.clone()))
    }

    pub fn find_cbor<T>(&self, value: &KeepCbor<T>) -> Option<&[u8]> {
        let ref_ = value.cbor_ref.as_ref()?;
        self.get_slice(ref_)
    }

    pub fn decode<'b, T>(
        &'b mut self,
        data: &[u8],
    ) -> Result<(T, DefaultKey), minicbor::decode::Error>
    where
        T: Decode<'b, DefaultKey>,
    {
        let data = ArrayVec::try_from(data).unwrap();
        let mut key = self.heap.insert(data);

        let mut decoder = minicbor::Decoder::new(self.heap[key].as_slice());

        let value = decoder.decode_with(&mut key)?;
        Ok((value, key))
    }

    pub fn forget(&mut self, key: DefaultKey) {
        self.heap.remove(key);
    }
}

pub type CborBlockHeap = CborHeap<32>;

pub type CborTxHeap = CborHeap<1024>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CborRef(DefaultKey, std::ops::Range<usize>);

#[derive(Debug, Clone, PartialEq, Eq)]

pub struct KeepCbor<T> {
    inner: T,
    cbor_ref: Option<CborRef>,
}

impl<T> KeepCbor<T> {
    pub fn original_cbor<'b>(&self, heap: &'b CborHeap<1024>) -> Option<&'b [u8]> {
        let ref_ = self.cbor_ref.as_ref()?;
        heap.get_slice(ref_)
    }
}

impl<T> From<T> for KeepCbor<T> {
    fn from(inner: T) -> Self {
        Self {
            inner,
            cbor_ref: None,
        }
    }
}

impl<T: Deref> Deref for KeepCbor<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'b, Ctx, T> Decode<'b, Ctx> for KeepCbor<T>
where
    Ctx: CborHeapContext,
    T: Decode<'b, Ctx>,
{
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut Ctx,
    ) -> Result<Self, minicbor::decode::Error> {
        let start_pos = d.position();
        let value = T::decode(d, ctx)?;
        let end_pos = d.position();

        Ok(KeepCbor {
            inner: value,
            cbor_ref: ctx.key().map(|key| CborRef(key, start_pos..end_pos)),
        })
    }
}

impl<Ctx, T> Encode<Ctx> for KeepCbor<T>
where
    Ctx: CborHeapContext,
    T: Encode<Ctx>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut Ctx,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        self.inner.encode(e, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Decode, Encode, Debug, PartialEq, Eq, Clone)]
    #[cbor(map, context_bound = "CborHeapContext")]
    pub struct ExampleStruct {
        #[n(0)]
        pub simple_field: Option<u64>,
        #[n(1)]
        pub hashable_value: KeepCbor<String>,
        #[n(2)]
        pub other_simple_field: Option<bool>,
    }

    fn owning_function(_: ExampleStruct) {
        // do anything
    }

    #[test]
    fn test_happy_path() {
        // this is the store for cbor bytes. The item size multiplied by the capacity
        // defines the pre-allocated memory.
        let mut heap = CborHeap::<1024>::new(1);

        // lets say that we get some CBOR from the network that we want to decode and
        // remember (simplified here using a hardcoded value)
        let cbor = hex::decode("a3000101613202f5").unwrap();

        // we ask the heap the decode the CBOR. This step will enter the CBOR into the
        // heap and decorate the decoded structure with a lightweight pointer to the
        // slice in the heap.
        //
        // The returned tuple has the decoded structure and a guard that is used to
        // forget the CBOR from the heap once we are done with it.
        let (plain_struct, cbor_guard) = heap.decode::<ExampleStruct>(&cbor).unwrap();

        // Let's say we need to access the cbor for one of the fields in the struct, we
        // can ask the heap to retrieve that particular CBOR slice. This search is very
        // efficient, is just one index lookup in the hep and a range lookup over the
        // full bytes of the CBOR.
        let cbor_fragment = heap.find_cbor(&plain_struct.hashable_value);

        assert_eq!(hex::encode(cbor_fragment.unwrap()), "6132");

        // when we're done doing all of the hashing, we can forget the CBOR from the
        // heap.
        heap.forget(cbor_guard);

        // but the plain structure is still valid and doesn't have any lifetimes or
        // dependencies. It can be moved as value to other functions or threads.
        owning_function(plain_struct);
    }
}
