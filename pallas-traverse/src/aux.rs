use std::ops::Deref;

use pallas_primitives::alonzo;

use crate::MultiEraTx;

impl<'b> MultiEraTx<'b> {
    pub fn aux_plutus_v1_scripts(&self) -> &[alonzo::PlutusScript] {
        if let Some(aux_data) = self.aux_data() {
            if let alonzo::AuxiliaryData::PostAlonzo(x) = aux_data.deref() {
                if let Some(plutus) = &x.plutus_scripts {
                    return plutus.as_ref();
                }
            }
        }

        &[]
    }

    pub fn aux_native_scripts(&self) -> &[alonzo::NativeScript] {
        if let Some(aux_data) = self.aux_data() {
            match aux_data.deref() {
                alonzo::AuxiliaryData::PostAlonzo(x) => {
                    if let Some(scripts) = &x.native_scripts {
                        return scripts.as_ref();
                    }
                }
                alonzo::AuxiliaryData::ShelleyMa(x) => {
                    if let Some(scripts) = &x.auxiliary_scripts {
                        return scripts.as_ref();
                    }
                }
                _ => (),
            }
        }

        &[]
    }
}
