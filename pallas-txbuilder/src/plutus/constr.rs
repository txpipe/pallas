use pallas_primitives::babbage::{Constr, PlutusData};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct PlutusDataConstr {
    tag: u64,
    constr_index: Option<u64>,
    fields: Vec<PlutusData>,
}

impl From<PlutusDataConstr> for PlutusData {
    fn from(value: PlutusDataConstr) -> Self {
        Self::Constr(Constr {
            tag: value.tag,
            any_constructor: value.constr_index,
            fields: value.fields,
        })
    }
}

impl PlutusDataConstr {
    pub fn field(mut self, item: impl Into<PlutusData>) -> Self {
        self.fields.push(item.into());
        self
    }
}

pub fn constr(tag: u64, constr_index: u64) -> PlutusDataConstr {
    PlutusDataConstr {
        tag,
        constr_index: Some(constr_index),
        fields: vec![],
    }
}

pub fn any_constr(tag: u64) -> PlutusDataConstr {
    PlutusDataConstr {
        tag,
        constr_index: None,
        fields: vec![],
    }
}
