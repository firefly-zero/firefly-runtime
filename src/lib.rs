#![no_std]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::new_without_default)]

mod graphics;
mod linking;
mod state;

pub use linking::link;
pub use state::State;
