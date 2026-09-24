use std::{borrow::Cow, ops::Deref};

use pallas_codec::utils::KeepRaw;
use pallas_primitives::{
    Hash, PlutusData, PlutusScript,
    alonzo::{self, BootstrapWitness, VKeyWitness},
    conway,
};

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{
    ComputeHash as _, Era, MultiEraNativeClause, MultiEraNativeScript, MultiEraRedeemer,
    MultiEraRedeemerTag, MultiEraTx, OriginalHash as _,
};

/// Reads the six clauses every era's native script type names, under the era
/// module given, wrapping a held script list with the helper given. An era
/// that names more clauses passes them as further arms, so each match stays
/// exhaustive over its own type.
macro_rules! shared_clauses {
    ($script:expr, $era:ident, $wrap:ident $(, $pattern:pat => $clause:expr)* $(,)?) => {
        match $script {
            $era::NativeScript::ScriptPubkey(x) => MultiEraNativeClause::Pubkey(x),
            $era::NativeScript::ScriptAll(x) => MultiEraNativeClause::All($wrap(x)),
            $era::NativeScript::ScriptAny(x) => MultiEraNativeClause::Any($wrap(x)),
            $era::NativeScript::ScriptNOfK(k, scripts) => {
                MultiEraNativeClause::NOfK(*k, $wrap(scripts))
            }
            $era::NativeScript::InvalidBefore(slot) => MultiEraNativeClause::InvalidBefore(*slot),
            $era::NativeScript::InvalidHereafter(slot) => {
                MultiEraNativeClause::InvalidHereafter(*slot)
            }
            $($pattern => $clause,)*
        }
    };
}

impl<'b> MultiEraNativeScript<'b> {
    pub fn from_alonzo_compatible(script: &'b KeepRaw<'b, alonzo::NativeScript>) -> Self {
        Self::AlonzoCompatible(Cow::Borrowed(script))
    }

    /// Read a script that reached this crate already decoded, so that it has
    /// no bytes of its own to hash.
    pub fn from_decoded_alonzo_compatible(script: &alonzo::NativeScript) -> Self {
        Self::AlonzoCompatible(Cow::Owned(KeepRaw::from(script.clone())))
    }

    #[cfg(feature = "unstable")]
    pub fn from_dijkstra(script: &'b KeepRaw<'b, dijkstra::NativeScript>) -> Self {
        Self::Dijkstra(Cow::Borrowed(script))
    }

    #[cfg(feature = "unstable")]
    pub fn from_decoded_dijkstra(script: &dijkstra::NativeScript) -> Self {
        Self::Dijkstra(Cow::Owned(KeepRaw::from(script.clone())))
    }

    pub fn as_alonzo_compatible(&self) -> Option<&alonzo::NativeScript> {
        match self {
            Self::AlonzoCompatible(x) => Some(x.deref().deref()),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(_) => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::NativeScript> {
        match self {
            Self::Dijkstra(x) => Some(x.deref().deref()),
            Self::AlonzoCompatible(_) => None,
        }
    }

    /// Returns the era whose native script type carries this value. The Alonzo
    /// type serves Shelley through Conway, so that variant names Alonzo.
    pub fn era(&self) -> Era {
        match self {
            Self::AlonzoCompatible(_) => Era::Alonzo,
            #[cfg(feature = "unstable")]
            Self::Dijkstra(_) => Era::Dijkstra,
        }
    }

    /// Returns the script hash, blake2b-224 behind a zero language tag.
    pub fn hash(&self) -> Hash<28> {
        match self {
            Self::AlonzoCompatible(x) => {
                if x.raw_cbor().is_empty() {
                    x.deref().deref().compute_hash()
                } else {
                    x.deref().original_hash()
                }
            }
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => {
                if x.raw_cbor().is_empty() {
                    x.deref().deref().compute_hash()
                } else {
                    x.deref().original_hash()
                }
            }
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::AlonzoCompatible(x) => {
                pallas_codec::minicbor::to_vec(x.deref()).expect("to_vec is infallible")
            }
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => {
                pallas_codec::minicbor::to_vec(x.deref()).expect("to_vec is infallible")
            }
        }
    }

    /// Returns the clause at the root of this script, with any script it holds
    /// reported in the same era variant as this one.
    pub fn clause(&self) -> MultiEraNativeClause<'_> {
        match self {
            Self::AlonzoCompatible(x) => {
                shared_clauses!(x.deref().deref(), alonzo, alonzo_compatible_scripts)
            }
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => shared_clauses!(
                x.deref().deref(),
                dijkstra,
                dijkstra_scripts,
                dijkstra::NativeScript::ScriptRequireGuard(credential) =>
                    MultiEraNativeClause::RequireGuard(credential),
            ),
        }
    }
}

fn alonzo_compatible_scripts<'a>(
    scripts: &[alonzo::NativeScript],
) -> Vec<MultiEraNativeScript<'a>> {
    scripts
        .iter()
        .map(MultiEraNativeScript::from_decoded_alonzo_compatible)
        .collect()
}

#[cfg(feature = "unstable")]
fn dijkstra_scripts<'a>(scripts: &[dijkstra::NativeScript]) -> Vec<MultiEraNativeScript<'a>> {
    scripts
        .iter()
        .map(MultiEraNativeScript::from_decoded_dijkstra)
        .collect()
}

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
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => x
                .transaction_witness_set
                .vkeywitness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => x
                .transaction_witness_set
                .vkeywitness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    pub fn multi_era_native_scripts(&self) -> Vec<MultiEraNativeScript<'_>> {
        match self {
            Self::Byron(_) => vec![],
            Self::AlonzoCompatible(x, _) => x
                .transaction_witness_set
                .native_script
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraNativeScript::from_alonzo_compatible)
                .collect(),
            Self::Babbage(x) => x
                .transaction_witness_set
                .native_script
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraNativeScript::from_alonzo_compatible)
                .collect(),
            Self::Conway(x) => x
                .transaction_witness_set
                .native_script
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraNativeScript::from_alonzo_compatible)
                .collect(),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => x
                .transaction_witness_set
                .native_script
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraNativeScript::from_dijkstra)
                .collect(),
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => x
                .transaction_witness_set
                .native_script
                .iter()
                .flat_map(|x| x.iter())
                .map(MultiEraNativeScript::from_dijkstra)
                .collect(),
        }
    }

    #[deprecated(
        since = "1.5.0",
        note = "use multi_era_native_scripts. This method cannot represent Dijkstra scripts"
    )]
    pub fn native_scripts(&self) -> &[KeepRaw<'b, alonzo::NativeScript>] {
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
            #[cfg(feature = "unstable")]
            Self::Dijkstra(..) | Self::DijkstraSub(..) => &[],
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
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => x
                .transaction_witness_set
                .bootstrap_witness
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => x
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
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => x
                .transaction_witness_set
                .plutus_v1_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => x
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
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => x
                .transaction_witness_set
                .plutus_data
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => x
                .transaction_witness_set
                .plutus_data
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }

    pub fn find_plutus_data(&self, hash: &Hash<32>) -> Option<&KeepRaw<'b, PlutusData>> {
        self.plutus_data()
            .iter()
            .find(|x| x.original_hash() == *hash)
    }

    pub fn redeemers(&self) -> Vec<MultiEraRedeemer<'_>> {
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
            Self::Conway(x) => match x.transaction_witness_set.redeemer.as_deref() {
                Some(conway::Redeemers::Map(x)) => x
                    .iter()
                    .map(|(k, v)| MultiEraRedeemer::from_conway(k, v))
                    .collect(),
                Some(conway::Redeemers::List(x)) => x
                    .iter()
                    .map(MultiEraRedeemer::from_conway_deprecated)
                    .collect(),
                _ => vec![],
            },
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => match x.transaction_witness_set.redeemer.as_deref() {
                Some(x) => x
                    .iter()
                    .map(|(k, v)| MultiEraRedeemer::from_dijkstra(k, v))
                    .collect(),
                None => vec![],
            },
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => match x.transaction_witness_set.redeemer.as_deref() {
                Some(x) => x
                    .iter()
                    .map(|(k, v)| MultiEraRedeemer::from_dijkstra(k, v))
                    .collect(),
                None => vec![],
            },
        }
    }

    pub fn find_spend_redeemer(&self, input_order: u32) -> Option<MultiEraRedeemer<'_>> {
        self.redeemers()
            .into_iter()
            .find(|r| r.multi_era_tag() == MultiEraRedeemerTag::Spend && r.index() == input_order)
    }

    pub fn find_mint_redeemer(&self, mint_order: u32) -> Option<MultiEraRedeemer<'_>> {
        self.redeemers()
            .into_iter()
            .find(|r| r.multi_era_tag() == MultiEraRedeemerTag::Mint && r.index() == mint_order)
    }

    pub fn find_withdrawal_redeemer(&self, withdrawal_order: u32) -> Option<MultiEraRedeemer<'_>> {
        self.redeemers().into_iter().find(|r| {
            r.multi_era_tag() == MultiEraRedeemerTag::Reward && r.index() == withdrawal_order
        })
    }

    pub fn find_certificate_redeemer(
        &self,
        certificate_order: u32,
    ) -> Option<MultiEraRedeemer<'_>> {
        self.redeemers().into_iter().find(|r| {
            r.multi_era_tag() == MultiEraRedeemerTag::Cert && r.index() == certificate_order
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
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => x
                .transaction_witness_set
                .plutus_v2_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => x
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
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => x
                .transaction_witness_set
                .plutus_v3_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
            #[cfg(feature = "unstable")]
            Self::DijkstraSub(x) => x
                .transaction_witness_set
                .plutus_v3_script
                .as_ref()
                .map(|x| x.as_ref())
                .unwrap_or(&[]),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Era, MultiEraNativeClause, MultiEraTx, testing};
    use pallas_crypto::hash::Hasher;

    #[test]
    fn a_witness_set_script_hashes_the_bytes_it_arrived_in() {
        let wire = testing::native_script_invalid_before_long_form(5);
        let canonical = testing::native_script_invalid_before(5);
        assert_ne!(
            Hasher::<224>::hash_tagged(&wire, 0),
            Hasher::<224>::hash_tagged(&canonical, 0),
            "the two encodings of this script must hash differently, or this test proves nothing"
        );

        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&wire),
            None,
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();
        assert_eq!(scripts.len(), 1, "the witness set carries one script");

        assert_eq!(
            scripts[0].hash(),
            Hasher::<224>::hash_tagged(&wire, 0),
            "a witness set script is keyed by the bytes it arrived in"
        );
        assert_eq!(
            hex::encode(scripts[0].encode()),
            hex::encode(&wire),
            "and it re-encodes to those same bytes"
        );
    }

    #[test]
    fn an_auxiliary_data_script_hashes_this_crates_own_encoding() {
        let wire = testing::native_script_invalid_before_long_form(5);
        let canonical = testing::native_script_invalid_before(5);

        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            Some(&testing::post_alonzo_aux_data(&wire, None)),
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_aux_native_scripts();
        assert_eq!(scripts.len(), 1, "the auxiliary data carries one script");

        assert_eq!(
            scripts[0].hash(),
            Hasher::<224>::hash_tagged(&canonical, 0),
            "the auxiliary data type keeps no bytes, so only this crate's own encoding is available"
        );
        assert_ne!(
            scripts[0].hash(),
            Hasher::<224>::hash_tagged(&wire, 0),
            "and the hash the ledger keyed this script by is not recoverable here"
        );
    }

    /// Auxiliary data with a native script, a V2 script at key 3 and a V3 at key 4.
    fn aux_data_with_v2_and_v3() -> Vec<u8> {
        testing::post_alonzo_aux_data_with_plutus(
            &testing::native_script_pubkey(0x5c),
            &[(3, PLUTUS_V2_SCRIPT), (4, PLUTUS_V3_SCRIPT)],
        )
    }

    const PLUTUS_V2_SCRIPT: &[u8] = &[0x02, 0x22, 0x22];
    const PLUTUS_V3_SCRIPT: &[u8] = &[0x03, 0x33];

    #[test]
    fn the_three_dijkstra_auxiliary_plutus_accessors_answer_empty_for_conway() {
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            Some(&aux_data_with_v2_and_v3()),
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");

        assert_eq!(
            tx.multi_era_aux_native_scripts().len(),
            1,
            "the auxiliary data this transaction carries is readable, so an empty answer below is about the key and not about the data"
        );
        assert!(tx.aux_plutus_v2_scripts().is_empty());
        assert!(tx.aux_plutus_v3_scripts().is_empty());
        assert!(tx.aux_plutus_v4_scripts().is_empty());
    }

    /// Conway decodes the same bytes through the Alonzo type, which has no key 3 or 4.
    #[cfg(feature = "unstable")]
    #[test]
    fn dijkstra_answers_the_auxiliary_plutus_v2_and_v3_scripts_conway_drops() {
        let cbor = testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            Some(&aux_data_with_v2_and_v3()),
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");

        assert_eq!(
            tx.multi_era_aux_native_scripts().len(),
            1,
            "the same auxiliary data reaches this era too"
        );

        assert_eq!(
            tx.aux_plutus_v2_scripts().len(),
            1,
            "key 3 is one V2 script"
        );
        assert_eq!(
            tx.aux_plutus_v2_scripts()[0].0.as_slice(),
            PLUTUS_V2_SCRIPT,
            "and it carries the bytes it arrived in"
        );

        assert_eq!(
            tx.aux_plutus_v3_scripts().len(),
            1,
            "key 4 is one V3 script"
        );
        assert_eq!(
            tx.aux_plutus_v3_scripts()[0].0.as_slice(),
            PLUTUS_V3_SCRIPT,
            "and it carries the bytes it arrived in"
        );

        assert!(
            tx.aux_plutus_v4_scripts().is_empty(),
            "key 5 is absent, so no V2 or V3 script may be reported as a V4 one"
        );
    }

    #[test]
    fn a_conway_native_script_witness_is_read_in_the_shared_type() {
        let script = testing::native_script_pubkey(0x5c);
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&script),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();

        assert_eq!(scripts.len(), 1, "the witness set carries one script");
        assert_eq!(scripts[0].era(), Era::Alonzo);
        assert!(scripts[0].as_alonzo_compatible().is_some());
        assert_eq!(
            hex::encode(scripts[0].encode()),
            hex::encode(&script),
            "the script re-encodes to the bytes it was read from"
        );
    }

    #[test]
    fn conway_auxiliary_data_scripts_are_read_in_the_shared_type() {
        let script = testing::native_script_pubkey(0x5c);
        let aux = testing::post_alonzo_aux_data(&script, None);
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            Some(&aux),
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_aux_native_scripts();

        assert_eq!(scripts.len(), 1, "the auxiliary data carries one script");
        assert_eq!(scripts[0].era(), Era::Alonzo);
        assert!(scripts[0].as_alonzo_compatible().is_some());
        assert_eq!(
            hex::encode(scripts[0].encode()),
            hex::encode(&script),
            "the script re-encodes to the bytes it was read from"
        );
        assert!(
            tx.multi_era_native_scripts().is_empty(),
            "a script in the auxiliary data is not a witness"
        );
    }

    #[test]
    fn a_conway_transaction_with_no_script_reports_none() {
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        assert!(tx.multi_era_native_scripts().is_empty());
        assert!(tx.multi_era_aux_native_scripts().is_empty());
    }

    /// A script whose root clause holds one of every other clause the Alonzo
    /// type names, the fourth of them written in the long form.
    fn script_of_every_alonzo_clause() -> Vec<u8> {
        let any = testing::native_script_any(&[&testing::native_script_pubkey(0x22)]);
        let n_of_k = testing::native_script_n_of_k(1, &[&testing::native_script_pubkey(0x33)]);

        testing::native_script_all(&[
            &testing::native_script_pubkey(0x11),
            &any,
            &n_of_k,
            &testing::native_script_invalid_before_long_form(7),
            &testing::native_script_invalid_hereafter(9),
        ])
    }

    #[test]
    fn a_conway_native_script_reads_every_clause_it_can_hold() {
        let script = script_of_every_alonzo_clause();
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&script),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();
        assert_eq!(scripts.len(), 1, "the witness set carries one script");

        let items = match scripts[0].clause() {
            MultiEraNativeClause::All(items) => items,
            other => panic!("the root clause is script_all, found {other:?}"),
        };
        assert_eq!(items.len(), 5, "the root clause holds five scripts");
        assert_eq!(
            items[0].era(),
            Era::Alonzo,
            "a nested script is reported in the same era variant as its parent"
        );

        match items[0].clause() {
            MultiEraNativeClause::Pubkey(h) => assert_eq!(h.as_ref(), [0x11; 28]),
            other => panic!("the first clause is script_pubkey, found {other:?}"),
        }

        match items[1].clause() {
            MultiEraNativeClause::Any(inner) => match inner[0].clause() {
                MultiEraNativeClause::Pubkey(h) => assert_eq!(h.as_ref(), [0x22; 28]),
                other => panic!("the clause under script_any is script_pubkey, found {other:?}"),
            },
            other => panic!("the second clause is script_any, found {other:?}"),
        }

        match items[2].clause() {
            MultiEraNativeClause::NOfK(k, inner) => {
                assert_eq!(k, 1, "one of the scripts listed satisfies the clause");
                match inner[0].clause() {
                    MultiEraNativeClause::Pubkey(h) => assert_eq!(h.as_ref(), [0x33; 28]),
                    other => {
                        panic!("the clause under script_n_of_k is script_pubkey, found {other:?}")
                    }
                }
            }
            other => panic!("the third clause is script_n_of_k, found {other:?}"),
        }

        assert_eq!(items[3].clause(), MultiEraNativeClause::InvalidBefore(7));
        assert_ne!(
            items[3].clause(),
            MultiEraNativeClause::InvalidHereafter(7),
            "the two slot clauses differ by one tag, so neither may read as the other"
        );
        assert_eq!(items[4].clause(), MultiEraNativeClause::InvalidHereafter(9));
        assert_ne!(
            items[4].clause(),
            MultiEraNativeClause::InvalidBefore(9),
            "the two slot clauses differ by one tag, so neither may read as the other"
        );
    }

    #[test]
    fn a_negative_n_of_k_threshold_reads_back_signed() {
        let script = testing::native_script_n_of_k(-1, &[&testing::native_script_pubkey(0x44)]);
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&script),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();
        assert_eq!(scripts.len(), 1, "the witness set carries one script");

        let (k, inner) = match scripts[0].clause() {
            MultiEraNativeClause::NOfK(k, inner) => (k, inner),
            other => panic!("the root clause is script_n_of_k, found {other:?}"),
        };
        assert_eq!(
            k, -1,
            "the ledger types the threshold signed, so a negative one reads back as itself"
        );
        match inner[0].clause() {
            MultiEraNativeClause::Pubkey(h) => assert_eq!(h.as_ref(), [0x44; 28]),
            other => panic!("the clause under script_n_of_k is script_pubkey, found {other:?}"),
        }
    }

    #[test]
    fn a_nested_script_carries_no_bytes_of_its_own() {
        let script = script_of_every_alonzo_clause();
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&script),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();
        let items = match scripts[0].clause() {
            MultiEraNativeClause::All(items) => items,
            other => panic!("the root clause is script_all, found {other:?}"),
        };

        assert_eq!(
            scripts[0].hash(),
            Hasher::<224>::hash_tagged(&script, 0),
            "the script read from the witness set hashes the bytes it arrived in"
        );

        let long_form = testing::native_script_invalid_before_long_form(7);
        assert_eq!(
            hex::encode(items[3].encode()),
            hex::encode(testing::native_script_invalid_before(7)),
            "a nested script re-encodes canonically"
        );
        assert_ne!(
            hex::encode(items[3].encode()),
            hex::encode(&long_form),
            "and so not to the form it arrived in"
        );
        assert_ne!(
            items[3].hash(),
            Hasher::<224>::hash_tagged(&long_form, 0),
            "so a nested script hashes its re-encoding rather than the bytes on the wire"
        );
        assert_eq!(
            items[3].hash(),
            Hasher::<224>::hash_tagged(&testing::native_script_invalid_before(7), 0),
            "which is the hash of the canonical form"
        );
    }
}

#[cfg(all(test, feature = "unstable"))]
mod dijkstra_tests {
    use super::*;
    use crate::{MultiEraBlock, testing};

    #[test]
    fn a_dijkstra_native_script_witness_is_readable() {
        let guard = testing::native_script_require_guard(0x7a);
        let cbor = testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&guard),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();

        assert_eq!(scripts.len(), 1, "the witness set carries one script");
        assert_eq!(scripts[0].era(), Era::Dijkstra);
        assert!(
            scripts[0].as_alonzo_compatible().is_none(),
            "a Dijkstra script must not be reported in the shared type"
        );

        match scripts[0]
            .as_dijkstra()
            .expect("readable as Dijkstra's type")
        {
            dijkstra::NativeScript::ScriptRequireGuard(
                pallas_primitives::StakeCredential::AddrKeyhash(h),
            ) => assert_eq!(h.as_ref(), [0x7a; 28]),
            other => panic!("expected a guard clause, found {other:?}"),
        }

        assert_eq!(
            hex::encode(scripts[0].encode()),
            hex::encode(&guard),
            "the script re-encodes to the bytes it was read from"
        );
    }

    #[test]
    fn a_guard_clause_nested_in_a_script_all_is_read_as_a_guard() {
        let guard = testing::native_script_require_guard(0x7a);
        let script = testing::native_script_all(&[
            &guard,
            &testing::native_script_pubkey(0x44),
            &testing::native_script_invalid_hereafter(9),
        ]);
        let cbor = testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&script),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();
        assert_eq!(scripts.len(), 1, "the witness set carries one script");

        let items = match scripts[0].clause() {
            MultiEraNativeClause::All(items) => items,
            other => panic!("the root clause is script_all, found {other:?}"),
        };
        assert_eq!(items.len(), 3, "the root clause holds three scripts");
        assert_eq!(
            items[0].era(),
            Era::Dijkstra,
            "a nested script keeps its era"
        );

        match items[0].clause() {
            MultiEraNativeClause::RequireGuard(
                pallas_primitives::StakeCredential::AddrKeyhash(h),
            ) => assert_eq!(h.as_ref(), [0x7a; 28]),
            other => panic!("the first clause is script_require_guard, found {other:?}"),
        }

        let keyhash: Hash<28> = [0x7a; 28].into();
        assert_ne!(
            items[0].clause(),
            MultiEraNativeClause::Pubkey(&keyhash),
            "a guard on a key credential must not read as a pubkey clause"
        );

        match items[1].clause() {
            MultiEraNativeClause::Pubkey(h) => assert_eq!(h.as_ref(), [0x44; 28]),
            other => panic!("the second clause is script_pubkey, found {other:?}"),
        }

        assert_eq!(items[2].clause(), MultiEraNativeClause::InvalidHereafter(9));
    }

    #[test]
    fn a_conway_native_script_witness_is_not_reported_as_a_dijkstra_one() {
        let script = testing::native_script_pubkey(0x5c);
        let cbor = testing::conway_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&script),
            None,
            true,
        );

        let tx = MultiEraTx::decode_for_era(Era::Conway, &cbor).expect("must decode");
        let scripts = tx.multi_era_native_scripts();

        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].era(), Era::Alonzo);
        assert!(scripts[0].as_dijkstra().is_none());
        assert!(scripts[0].as_alonzo_compatible().is_some());
    }

    #[test]
    fn a_transaction_with_no_script_witness_reports_none() {
        let cbor = testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            None,
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");
        assert!(tx.multi_era_native_scripts().is_empty());

        let cbor = hex::decode(include_str!("../../test_data/dijkstra3.block")).unwrap();
        let block = MultiEraBlock::decode(&cbor).unwrap();
        for tx in block.txs() {
            assert!(tx.multi_era_native_scripts().is_empty());
        }
    }

    #[test]
    fn dijkstra_auxiliary_data_scripts_are_readable() {
        let guard = testing::native_script_require_guard(0x3b);
        let aux = testing::post_alonzo_aux_data(&guard, Some(&[0xd8, 0x79, 0x80]));

        let cbor = testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            Some(&aux),
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");

        let scripts = tx.multi_era_aux_native_scripts();
        assert_eq!(scripts.len(), 1, "the auxiliary data carries one script");
        assert_eq!(scripts[0].era(), Era::Dijkstra);
        assert!(scripts[0].as_dijkstra().is_some());
        assert!(
            scripts[0].as_alonzo_compatible().is_none(),
            "a Dijkstra script must not be reported in the shared type"
        );

        assert_eq!(tx.aux_plutus_v4_scripts().len(), 1, "and one V4 script");
        assert_eq!(tx.aux_plutus_v4_scripts()[0].0.as_ref(), [0xd8, 0x79, 0x80]);

        assert!(tx.multi_era_native_scripts().is_empty());
        assert!(tx.plutus_v1_scripts().is_empty());
    }

    #[test]
    fn auxiliary_data_with_no_plutus_v4_script_reports_none() {
        let guard = testing::native_script_require_guard(0x3b);
        let aux = testing::post_alonzo_aux_data(&guard, None);

        let cbor = testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::empty_witness_set(),
            Some(&aux),
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");

        assert_eq!(tx.multi_era_aux_native_scripts().len(), 1);
        assert!(tx.aux_plutus_v4_scripts().is_empty());
        assert!(tx.aux_plutus_v2_scripts().is_empty());
        assert!(tx.aux_plutus_v3_scripts().is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn the_legacy_script_readers_answer_a_dijkstra_transaction_empty() {
        let script = testing::native_script_pubkey(0x5c);
        let guard = testing::native_script_require_guard(0x3b);
        let aux = testing::post_alonzo_aux_data(&guard, None);
        let cbor = testing::dijkstra_block_tx(
            &testing::minimal_body(),
            &testing::witness_set_with_native_script(&script),
            Some(&aux),
            true,
        );
        let tx = MultiEraTx::decode_for_era(Era::Dijkstra, &cbor).expect("must decode");

        assert_eq!(
            tx.multi_era_native_scripts().len(),
            1,
            "the witness set carries one script"
        );
        assert_eq!(
            tx.multi_era_aux_native_scripts().len(),
            1,
            "the auxiliary data carries one script"
        );
        assert!(tx.native_scripts().is_empty());
        assert!(tx.aux_native_scripts().is_empty());
    }
}
