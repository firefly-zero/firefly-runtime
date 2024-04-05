#![cfg_attr(not(test), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::new_without_default)]

mod color;
mod error;
mod fs;
mod graphics;
mod input;
mod linking;
mod misc;
mod runtime;
mod state;

pub use error::Error;
pub use runtime::Runtime;
pub use state::{HEIGHT, WIDTH};
