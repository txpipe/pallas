use pallas_codec::utils::KeepRaw;
use pallas_primitives::{
    alonzo::{self, BootstrapWitness, NativeScript, VKeyWitness},
    PlutusData, PlutusScript,
};

use crate::{MultiEraRedeemer, MultiEraTx};

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

    pub fn plutus_v1_scripts(&self) -> &[alonzo::PlutusScript<1>] {
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

    pub fn redeemers(&self) -> Vec<MultiEraRedeemer> {
        match self {
            Self::Byron(_) => vec![],
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .redeemer
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraRedeemer::from_alonzo_compatible)
                .collect(),
            Self::Babbage(x) => x
                .transaction_witness_set
                .redeemer
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraRedeemer::from_alonzo_compatible)
                .collect(),
            Self::Conway(x) => x
                .transaction_witness_set
                .redeemer
                .iter()
                .flat_map(|x| x.iter())
                .map(|(k, v)| MultiEraRedeemer::from_conway(k, v))
                .collect(),
        }
    }

    pub fn find_spend_redeemer(&self, input_order: u32) -> Option<MultiEraRedeemer> {
        self.redeemers().into_iter().find(|r| {
            r.tag() == pallas_primitives::conway::RedeemerTag::Spend && r.index() == input_order
        })
    }

    pub fn find_mint_redeemer(&self, mint_order: u32) -> Option<MultiEraRedeemer> {
        self.redeemers().into_iter().find(|r| {
            r.tag() == pallas_primitives::conway::RedeemerTag::Mint && r.index() == mint_order
        })
    }

    pub fn find_withdrawal_redeemer(&self, withdrawal_order: u32) -> Option<MultiEraRedeemer> {
        self.redeemers().into_iter().find(|r| {
            r.tag() == pallas_primitives::conway::RedeemerTag::Reward
                && r.index() == withdrawal_order
        })
    }

    pub fn find_certificate_redeemer(&self, certificate_order: u32) -> Option<MultiEraRedeemer> {
        self.redeemers().into_iter().find(|r| {
            r.tag() == pallas_primitives::conway::RedeemerTag::Cert
                && r.index() == certificate_order
        })
    }

    pub fn plutus_v2_scripts(&self) -> &[PlutusScript<2>] {
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

    pub fn plutus_v3_scripts(&self) -> &[PlutusScript<3>] {
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
