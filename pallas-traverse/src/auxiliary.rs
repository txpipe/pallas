use std::ops::Deref;

use pallas_codec::utils::Nullable;
use pallas_primitives::{alonzo, babbage, conway};

use crate::MultiEraTx;

impl<'b> MultiEraTx<'b> {
    pub fn aux_plutus_v1_scripts(&self) -> &[alonzo::PlutusScript] {
        match self {
            MultiEraTx::Byron(_) => &[],
            MultiEraTx::AlonzoCompatible(x, _) => {
                if let Nullable::Some(ad) = &x.auxiliary_data {
                    match ad.deref() {
                        alonzo::AuxiliaryData::PostAlonzo(y) => {
                            if let Some(scripts) = y.plutus_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        _ => &[],
                    }
                } else {
                    &[]
                }
            }
            MultiEraTx::Babbage(x) => {
                if let Nullable::Some(ad) = &x.auxiliary_data {
                    match ad.deref() {
                        babbage::AuxiliaryData::PostAlonzo(y) => {
                            if let Some(scripts) = y.plutus_v1_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        _ => &[],
                    }
                } else {
                    &[]
                }
            }
            MultiEraTx::Conway(x) => {
                if let Nullable::Some(ad) = &x.auxiliary_data {
                    match ad.deref() {
                        conway::AuxiliaryData::PostAlonzo(y) => {
                            if let Some(scripts) = y.plutus_v1_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        _ => &[],
                    }
                } else {
                    &[]
                }
            }
        }
    }

    pub fn aux_native_scripts(&self) -> &[alonzo::NativeScript] {
        match self {
            MultiEraTx::Byron(_) => &[],
            MultiEraTx::AlonzoCompatible(x, _) => {
                if let Nullable::Some(ad) = &x.auxiliary_data {
                    match ad.deref() {
                        alonzo::AuxiliaryData::ShelleyMa(y) => {
                            if let Some(scripts) = y.auxiliary_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        alonzo::AuxiliaryData::PostAlonzo(y) => {
                            if let Some(scripts) = y.native_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        _ => &[],
                    }
                } else {
                    &[]
                }
            }
            MultiEraTx::Babbage(x) => {
                if let Nullable::Some(ad) = &x.auxiliary_data {
                    match ad.deref() {
                        babbage::AuxiliaryData::ShelleyMa(y) => {
                            if let Some(scripts) = y.auxiliary_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        babbage::AuxiliaryData::PostAlonzo(y) => {
                            if let Some(scripts) = y.native_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        _ => &[],
                    }
                } else {
                    &[]
                }
            }
            MultiEraTx::Conway(x) => {
                if let Nullable::Some(ad) = &x.auxiliary_data {
                    match ad.deref() {
                        conway::AuxiliaryData::ShelleyMa(y) => {
                            if let Some(scripts) = y.auxiliary_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        conway::AuxiliaryData::PostAlonzo(y) => {
                            if let Some(scripts) = y.native_scripts.as_ref() {
                                scripts.as_ref()
                            } else {
                                &[]
                            }
                        }
                        _ => &[],
                    }
                } else {
                    &[]
                }
            }
        }
    }
}
