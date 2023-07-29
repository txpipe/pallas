use pallas_codec::utils::KeepRaw;
use pallas_primitives::{
    alonzo::{self, BootstrapWitness, NativeScript, PlutusData, Redeemer, VKeyWitness},
    babbage::PlutusV2Script,
};

use crate::MultiEraTx;

impl<'b> MultiEraTx<'b> {
    pub fn vkey_witnesses(&self) -> &[VKeyWitness] {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .vkeywitness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            Self::Babbage(x) => x
                .transaction_witness_set
                .vkeywitness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    pub fn native_scripts(&self) -> &[NativeScript] {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .native_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            Self::Babbage(x) => x
                .transaction_witness_set
                .native_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    pub fn bootstrap_witnesses(&self) -> &[BootstrapWitness] {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .bootstrap_witness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            Self::Babbage(x) => x
                .transaction_witness_set
                .bootstrap_witness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    pub fn plutus_v1_scripts(&self) -> &[alonzo::PlutusScript] {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .plutus_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            Self::Babbage(x) => x
                .transaction_witness_set
                .plutus_v1_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    pub fn plutus_data(&self) -> &[KeepRaw<'b, PlutusData>] {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .plutus_data
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            Self::Babbage(x) => x
                .transaction_witness_set
                .plutus_data
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    pub fn redeemers(&self) -> &[Redeemer] {
        match self {
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .redeemer
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            Self::Babbage(x) => x
                .transaction_witness_set
                .redeemer
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    pub fn plutus_v2_scripts(&self) -> &[PlutusV2Script] {
        match self {
            Self::Babbage(x) => x
                .transaction_witness_set
                .plutus_v2_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            _ => &[],
        }
    }
}
