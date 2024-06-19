#![cfg_attr(not(test), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::new_without_default)]

extern crate alloc;

mod color;
mod config;
mod error;
mod frame_buffer;
mod host;
mod linking;
mod menu;
mod runtime;
mod state;

pub use config::{FullID, RuntimeConfig};
pub use error::Error;
pub use frame_buffer::{HEIGHT, WIDTH};
pub use runtime::Runtime;
