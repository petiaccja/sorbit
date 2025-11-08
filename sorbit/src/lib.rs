// Disable the [`std`] standard crate when the "std" feature is not enabled.
#![cfg_attr(not(feature = "std"), no_std)]

// Enable the [`alloc`] standard crate when the "alloc" feature is enabled.
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bit;
pub mod byte_order;
pub mod codegen;
pub mod deserialize;
pub mod error;
pub mod io;
pub mod serialize;
pub use sorbit_derive::{Deserialize, Serialize};

mod types;

extern crate self as sorbit;
