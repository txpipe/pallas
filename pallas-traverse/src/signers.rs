use pallas_crypto::hash::Hash;
use pallas_primitives::alonzo;

#[cfg(feature = "unstable")]
use pallas_primitives::{StakeCredential, dijkstra};

use crate::MultiEraSigners;

impl MultiEraSigners<'_> {
    pub fn as_alonzo(&self) -> Option<&alonzo::RequiredSigners> {
        match self {
            Self::AlonzoCompatible(x) => Some(x),
            _ => None,
        }
    }

    #[cfg(feature = "unstable")]
    pub fn as_dijkstra(&self) -> Option<&dijkstra::Guards> {
        match self {
            Self::Dijkstra(x) => Some(x),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::AlonzoCompatible(x) => x.is_empty(),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => match x {
                dijkstra::Guards::AddrKeyhashes(x) => x.is_empty(),
                dijkstra::Guards::Credentials(x) => x.is_empty(),
            },
            Self::NotApplicable | Self::Empty => true,
        }
    }

    /// Collect the signing key hashes named by this value. A Dijkstra credential
    /// set yields its key credentials, and its script credentials are dropped.
    pub fn collect<'a, T>(&'a self) -> T
    where
        T: FromIterator<&'a Hash<28>>,
    {
        match self {
            Self::NotApplicable => std::iter::empty().collect(),
            Self::Empty => std::iter::empty().collect(),
            Self::AlonzoCompatible(x) => x.iter().collect(),
            #[cfg(feature = "unstable")]
            Self::Dijkstra(x) => match x {
                dijkstra::Guards::AddrKeyhashes(x) => x.iter().collect(),
                dijkstra::Guards::Credentials(x) => x
                    .iter()
                    .filter_map(|c| match c {
                        StakeCredential::AddrKeyhash(h) => Some(h),
                        StakeCredential::ScriptHash(_) => None,
                    })
                    .collect(),
            },
        }
    }
}
