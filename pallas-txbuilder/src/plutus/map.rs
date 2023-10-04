use indexmap::IndexMap;
use pallas_primitives::babbage::PlutusData;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct PlutusDataMap {
    inner: IndexMap<PlutusData, PlutusData>,
}

impl From<PlutusDataMap> for PlutusData {
    fn from(value: PlutusDataMap) -> Self {
        Self::Map(value.inner.into_iter().collect::<Vec<_>>().into())
    }
}

impl PlutusDataMap {
    pub fn item(mut self, key: impl Into<PlutusData>, value: impl Into<PlutusData>) -> Self {
        self.inner.insert(key.into(), value.into());
        self
    }
}

pub fn map() -> PlutusDataMap {
    PlutusDataMap::default()
}
