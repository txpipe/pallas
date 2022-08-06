use pallas_codec::utils::MaybeIndefArray;
use pallas_primitives::{alonzo::{self, VKeyWitness, Redeemer, PlutusData, BootstrapWitness, NativeScript}, babbage::{self, PlutusV2Script}};

use crate::MultiEraWitnesses;

impl<'b> MultiEraWitnesses<'b> {
    pub fn as_alonzo(&self) -> Option<&alonzo::TransactionWitnessSet> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    pub fn as_babbage(&self) -> Option<&babbage::TransactionWitnessSet> {
        match self {
            Self::Babbage(x) => Some(x),
            _ => None,
        }
    }

    pub fn vkeywitness(&self) -> Option<&MaybeIndefArray<VKeyWitness>> {
        match self {
            Self::AlonzoCompatible(x) => x.vkeywitness.as_ref(),
            Self::Babbage(x) => x.vkeywitness.as_ref(),
            _ => None,
        }
    }

    pub fn native_script(&self) -> Option<&MaybeIndefArray<NativeScript>> {
        match self {
            Self::AlonzoCompatible(x) => x.native_script.as_ref(),
            Self::Babbage(x) => x.native_script.as_ref(),
            _ => None,
        }
    }

    pub fn bootstrap_witness(&self) -> Option<&MaybeIndefArray<BootstrapWitness>> {
        match self {
            Self::AlonzoCompatible(x) => x.bootstrap_witness.as_ref(),
            Self::Babbage(x) => x.bootstrap_witness.as_ref(),
            _ => None,
        }
    }

    pub fn plutus_v1_script(&self) -> Option<&MaybeIndefArray<alonzo::PlutusScript>> {
        match self {
            Self::AlonzoCompatible(x) => x.plutus_script.as_ref(),
            Self::Babbage(x) => x.plutus_v1_script.as_ref(),
            _ => None,
        }
    }

    pub fn plutus_data(&self) -> Option<&MaybeIndefArray<PlutusData>> {
        match self {
            Self::AlonzoCompatible(x) => x.plutus_data.as_ref(),
            Self::Babbage(x) => x.plutus_data.as_ref(),
            _ => None,
        }
    }

    pub fn redeemer(&self) -> Option<&MaybeIndefArray<Redeemer>> {
        match self {
            Self::AlonzoCompatible(x) => x.redeemer.as_ref(),
            Self::Babbage(x) => x.redeemer.as_ref(),
            _ => None,
        }
    }

    pub fn plutus_v2_script(&self) -> Option<&MaybeIndefArray<PlutusV2Script>> {
        match self {
            Self::Babbage(x) => x.plutus_v2_script.as_ref(),
            _ => None,
        }
    }

    pub fn cbor(&self) -> &[u8] {
        match self {
            MultiEraWitnesses::AlonzoCompatible(x) => x.raw_cbor(),
            MultiEraWitnesses::Babbage(x) => x.raw_cbor(),
            MultiEraWitnesses::Byron(x) => x.raw_cbor(),
        }
    }
}
