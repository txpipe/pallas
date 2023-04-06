#![feature(async_fn_in_trait)]

pub mod agents;
pub mod bearers;
pub mod demux;
pub mod mux;

#[cfg(feature = "std")]
mod std;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "std")]
pub use crate::std::*;

pub type Payload = Vec<u8>;

pub type Message = (u16, Payload);
