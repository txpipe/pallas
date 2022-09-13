use pallas_codec::utils::KeepRaw;
use pallas_primitives::{
    alonzo::{self, BootstrapWitness, NativeScript, PlutusData, Redeemer, VKeyWitness},
    babbage::PlutusV2Script,
};

use crate::MultiEraTx;

impl<'b> MultiEraTx<'b> {
    pub fn vkey_witnesses(&self) -> Option<&[VKeyWitness]> {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .vkeywitness
                .as_ref()
                .map(|x| x.as_ref()),
            Self::Babbage(x) => x
                .transaction_witness_set
                .vkeywitness
                .as_ref()
                .map(|x| x.as_ref()),
            _ => None,
        }
    }

    pub fn native_scripts(&self) -> Option<&[NativeScript]> {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .native_script
                .as_ref()
                .map(|x| x.as_ref()),
            Self::Babbage(x) => x
                .transaction_witness_set
                .native_script
                .as_ref()
                .map(|x| x.as_ref()),
            _ => None,
        }
    }

    pub fn bootstrap_witnesses(&self) -> Option<&[BootstrapWitness]> {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .bootstrap_witness
                .as_ref()
                .map(|x| x.as_ref()),
            Self::Babbage(x) => x
                .transaction_witness_set
                .bootstrap_witness
                .as_ref()
                .map(|x| x.as_ref()),
            _ => None,
        }
    }

    pub fn plutus_v1_scripts(&self) -> Vec<&alonzo::PlutusScript> {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .plutus_script
                .iter()
                .flatten()
                .collect(),
            Self::Babbage(x) => x
                .transaction_witness_set
                .plutus_v1_script
                .iter()
                .flatten()
                .collect(),
            _ => vec![],
        }
    }

    pub fn plutus_data(&self) -> Vec<&KeepRaw<'b, PlutusData>> {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .plutus_data
                .iter()
                .flatten()
                .collect(),
            Self::Babbage(x) => x
                .transaction_witness_set
                .plutus_data
                .iter()
                .flatten()
                .collect(),
            _ => std::iter::empty().collect(),
        }
    }

    pub fn redeemers(&self) -> Option<&[Redeemer]> {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .redeemer
                .as_ref()
                .map(|x| x.as_ref()),
            Self::Babbage(x) => x
                .transaction_witness_set
                .redeemer
                .as_ref()
                .map(|x| x.as_ref()),
            _ => None,
        }
    }

    pub fn plutus_v2_scripts(&self) -> Option<&[PlutusV2Script]> {
        match self {
            Self::Babbage(x) => x
                .transaction_witness_set
                .plutus_v2_script
                .as_ref()
                .map(|x| x.as_ref()),
            _ => None,
        }
    }
}
