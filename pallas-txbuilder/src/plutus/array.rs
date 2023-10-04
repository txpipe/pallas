
use pallas_primitives::babbage::{PlutusData};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct PlutusDataArray {
    inner: Vec<PlutusData>,
}

impl From<PlutusDataArray> for PlutusData {
    fn from(value: PlutusDataArray) -> Self {
        Self::Array(value.inner)
    }
}

impl PlutusDataArray {
    pub fn item(mut self, item: impl Into<PlutusData>) -> Self {
        self.inner.push(item.into());
        self
    }
}

pub fn array() -> PlutusDataArray {
    PlutusDataArray::default()
}
