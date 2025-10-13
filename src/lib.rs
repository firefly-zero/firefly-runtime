#![cfg_attr(not(test), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::new_without_default)]

extern crate alloc;

mod battery;
mod canvas;
mod color;
mod config;
mod error;
mod error_scene;
mod frame_buffer;
mod host;
mod linking;
mod menu;
mod net;
mod runtime;
mod state;
mod stats;
mod utils;

pub use color::Rgb16;
pub use config::{FullID, FullIDError, RuntimeConfig};
pub use error::Error;
pub use frame_buffer::{FrameBuffer, RenderFB, HEIGHT, WIDTH};
pub use runtime::Runtime;
pub use state::NetHandler;
