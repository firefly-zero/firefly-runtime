//! Definitions for host-defined wasm functions.
//!
//! In other words, this is the API that we provide
//! to the apps in the runtime.

pub(crate) mod audio;
pub(crate) mod fs;
pub(crate) mod graphics;
pub(crate) mod input;
pub(crate) mod menu;
pub(crate) mod misc;
pub(crate) mod net;
pub(crate) mod sudo;
pub(crate) mod wasip1;

#[cfg(test)]
pub(crate) mod graphics_test;

#[cfg(test)]
pub(crate) mod misc_test;
