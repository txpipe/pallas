use pallas_codec::utils::KeepRaw;
use pallas_primitives::{
    alonzo::{self, BootstrapWitness, NativeScript, PlutusData, VKeyWitness},
    babbage::{PlutusV2Script, Redeemer},
    conway::{self, PlutusV3Script},
};

use crate::MultiEraTx;

impl<'b> MultiEraTx<'b> {
    pub fn vkey_witnesses(&self) -> &[VKeyWitness] {
        match self {
            Self::Byron(_) => &[],
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
            Self::Conway(x) => x
                .transaction_witness_set
                .vkeywitness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    pub fn native_scripts(&self) -> &[KeepRaw<'b, NativeScript>] {
        match self {
            Self::Byron(_) => &[],
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
            Self::Conway(x) => x
                .transaction_witness_set
                .native_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    pub fn bootstrap_witnesses(&self) -> &[BootstrapWitness] {
        match self {
            Self::Byron(_) => &[],
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
            Self::Conway(x) => x
                .transaction_witness_set
                .bootstrap_witness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    pub fn plutus_v1_scripts(&self) -> &[alonzo::PlutusScript] {
        match self {
            Self::Byron(_) => &[],
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
            Self::Conway(x) => x
                .transaction_witness_set
                .plutus_v1_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    pub fn plutus_data(&self) -> &[KeepRaw<'b, PlutusData>] {
        match self {
            Self::Byron(_) => &[],
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
            Self::Conway(x) => x
                .transaction_witness_set
                .plutus_data
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    // TODO: MultiEraRedeemer?
    pub fn redeemers(&self) -> &[Redeemer] {
        match self {
            Self::Byron(_) => &[],
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
            Self::Conway(_) => todo!(),
        }
    }

    pub fn plutus_v2_scripts(&self) -> &[PlutusV2Script] {
        match self {
            Self::Byron(_) => &[],
            Self::AlonzoCompatible(_, _) => &[],
            Self::Babbage(x) => x
                .transaction_witness_set
                .plutus_v2_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            Self::Conway(x) => x
                .transaction_witness_set
                .plutus_v2_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    pub fn plutus_v3_scripts(&self) -> &[PlutusV3Script] {
        match self {
            Self::Byron(_) => &[],
            Self::AlonzoCompatible(_, _) => &[],
            Self::Babbage(_) => &[],
            Self::Conway(x) => x
                .transaction_witness_set
                .plutus_v3_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }
}
