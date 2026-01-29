#![cfg_attr(not(test), no_std)]
#![deny(clippy::nursery)]
#![allow(
    clippy::new_without_default,
    clippy::too_many_arguments,
    clippy::missing_const_for_fn,
    clippy::redundant_pub_crate,
    clippy::option_if_let_else
)]

extern crate alloc;

mod battery;
mod canvas;
mod color;
mod config;
mod error;
mod error_scene;
mod frame_buffer;
mod host;
mod image;
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
