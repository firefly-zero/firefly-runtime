#![cfg_attr(not(test), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::new_without_default)]
mod color;
mod config;
mod error;
mod frame_buffer;
mod fs;
mod graphics;
mod graphics_tests;
mod input;
mod linking;
mod misc;
mod runtime;
mod state;
mod sudo;

pub use config::{FullID, RuntimeConfig};
pub use error::Error;
pub use frame_buffer::{HEIGHT, WIDTH};
pub use runtime::Runtime;
