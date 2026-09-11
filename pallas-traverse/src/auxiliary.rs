use std::ops::Deref;

use pallas_primitives::{PlutusScript, alonzo};

#[cfg(feature = "unstable")]
use pallas_primitives::dijkstra;

use crate::{MultiEraNativeScript, MultiEraTx};

impl MultiEraTx<'_> {
    pub fn aux_plutus_v1_scripts(&self) -> &[alonzo::PlutusScript<1>] {
        #[cfg(feature = "unstable")]
        if let Some(aux_data) = self.dijkstra_aux_data()
            && let dijkstra::AuxiliaryData::PostAlonzo(x) = aux_data.deref()
            && let Some(plutus) = &x.plutus_v1_scripts
        {
            return plutus.as_ref();
        }

        if let Some(aux_data) = self.aux_data()
            && let alonzo::AuxiliaryData::PostAlonzo(x) = aux_data.deref()
            && let Some(plutus) = &x.plutus_scripts
        {
            return plutus.as_ref();
        }

        &[]
    }

    /// Returns the PlutusV2 scripts in the auxiliary data, which only a Dijkstra
    /// transaction carries, since every earlier era decodes its auxiliary data
    /// through the Alonzo type and that type has no key for them.
    pub fn aux_plutus_v2_scripts(&self) -> &[PlutusScript<2>] {
        #[cfg(feature = "unstable")]
        if let Some(aux_data) = self.dijkstra_aux_data()
            && let dijkstra::AuxiliaryData::PostAlonzo(x) = aux_data.deref()
            && let Some(plutus) = &x.plutus_v2_scripts
        {
            return plutus.as_ref();
        }

        &[]
    }

    /// Returns the PlutusV3 scripts in the auxiliary data, which only a Dijkstra
    /// transaction carries, since every earlier era decodes its auxiliary data
    /// through the Alonzo type and that type has no key for them.
    pub fn aux_plutus_v3_scripts(&self) -> &[PlutusScript<3>] {
        #[cfg(feature = "unstable")]
        if let Some(aux_data) = self.dijkstra_aux_data()
            && let dijkstra::AuxiliaryData::PostAlonzo(x) = aux_data.deref()
            && let Some(plutus) = &x.plutus_v3_scripts
        {
            return plutus.as_ref();
        }

        &[]
    }

    /// Returns the PlutusV4 scripts in the auxiliary data, or an empty slice for
    /// every era before Dijkstra, whose type is the only one modelling this key.
    pub fn aux_plutus_v4_scripts(&self) -> &[PlutusScript<4>] {
        #[cfg(feature = "unstable")]
        if let Some(aux_data) = self.dijkstra_aux_data()
            && let dijkstra::AuxiliaryData::PostAlonzo(x) = aux_data.deref()
            && let Some(plutus) = &x.plutus_v4_scripts
        {
            return plutus.as_ref();
        }

        &[]
    }

    pub fn aux_native_scripts(&self) -> Vec<MultiEraNativeScript<'_>> {
        #[cfg(feature = "unstable")]
        if let Some(aux_data) = self.dijkstra_aux_data() {
            return match aux_data.deref() {
                dijkstra::AuxiliaryData::PostAlonzo(x) => x
                    .native_scripts
                    .iter()
                    .flat_map(|s| s.iter())
                    .map(MultiEraNativeScript::from_decoded_dijkstra)
                    .collect(),
                dijkstra::AuxiliaryData::ShelleyMa(x) => x
                    .auxiliary_scripts
                    .iter()
                    .flat_map(|s| s.iter())
                    .map(MultiEraNativeScript::from_decoded_dijkstra)
                    .collect(),
                dijkstra::AuxiliaryData::Shelley(_) => vec![],
            };
        }

        if let Some(aux_data) = self.aux_data() {
            return match aux_data.deref() {
                alonzo::AuxiliaryData::PostAlonzo(x) => x
                    .native_scripts
                    .iter()
                    .flat_map(|s| s.iter())
                    .map(MultiEraNativeScript::from_decoded_alonzo_compatible)
                    .collect(),
                alonzo::AuxiliaryData::ShelleyMa(x) => x
                    .auxiliary_scripts
                    .iter()
                    .flat_map(|s| s.iter())
                    .map(MultiEraNativeScript::from_decoded_alonzo_compatible)
                    .collect(),
                alonzo::AuxiliaryData::Shelley(_) => vec![],
            };
        }

        vec![]
    }
}
