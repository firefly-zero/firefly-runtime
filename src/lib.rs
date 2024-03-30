#![cfg_attr(not(test), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::new_without_default)]

mod color;
mod device;
mod error;
mod graphics;
mod linking;
mod runtime;
mod state;

pub use device::*;
pub use error::Error;
pub use runtime::Runtime;
pub use state::{HEIGHT, WIDTH};
