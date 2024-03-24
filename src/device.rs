use crate::color::FromRGB;
use core::marker::PhantomData;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point};
use embedded_graphics::pixelcolor::RgbColor;
use fugit::{Instant, MillisDurationU32};

pub struct Device<D, C, T, I, S, R>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
    T: Timer,
    I: Input,
    S: Storage<R>,
    R: embedded_io::Read + wasmi::Read,
{
    pub display: D,
    pub timer:   T,
    pub input:   I,
    pub storage: S,
    pub reader:  PhantomData<R>,
}

pub trait Timer {
    /// The current time.
    ///
    /// Should be precise enough for adjusting the delay between frames.
    ///
    /// Usually implemented as [rtic_time.Monotonic].
    /// May also sometimes be implemented as [rtic_monotonic.Monotonic].
    ///
    /// [rtic_time.Monotonic]: https://docs.rs/rtic-time/latest/rtic_time/trait.Monotonic.html
    /// [rtic_monotonic.Monotonic]: https://docs.rs/rtic-monotonic/latest/rtic_monotonic/trait.Monotonic.html
    fn now(&self) -> Instant<u32, 1, 1_000>;

    /// Suspends the current thread for the given duration.
    ///
    /// Should be precise enough for adjusting the delay between frames.
    ///
    /// Usually implemented as [embedded_hal.DelayNs].
    ///
    /// [embedded_hal.DelayNs]: https://docs.rs/embedded-hal/1.0.0/embedded_hal/delay/trait.DelayNs.html
    fn delay(&self, d: MillisDurationU32);
}

/// File system abstraction.
///
/// Designed to work nicely with [embedded_sdmmc] and the stdlib filesystem.
///
/// [embedded_sdmmc]: https://github.com/rust-embedded-community/embedded-sdmmc-rs
pub trait Storage<R: embedded_io::Read + wasmi::Read> {
    /// Open a file for reading.
    ///
    /// The file path is given as a slice of path components.
    /// There are at least 3 components: the first one is the root directory
    /// (either "roms" or "data"), the second is game ID, and the last is file name.
    /// Everything in between game ID and file name are directory names
    /// if the file is nested.
    ///
    /// The runtime ensures that the path is relative and never goes up the tree.
    fn open_file(&self, path: &[&str]) -> Option<R>;
}

pub trait Input {
    fn read_state(&self) -> InputState;
}

pub struct InputState {
    pub left:  Point,
    pub right: Point,
    pub menu:  bool,
    pub lock:  bool,
}
