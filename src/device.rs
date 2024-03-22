pub struct Device<D, C, T, S>
where
    D: embedded_graphics::draw_target::DrawTarget<Color = C>,
    C: embedded_graphics::pixelcolor::RgbColor,
    T: Timer,
    S: embedded_storage::Storage,
{
    pub display: D,
    pub timer: T,
    pub storage: S,
}

pub trait Timer {
    /// Pause the game execution (in ms).
    fn sleep(&self, ms: u64);

    /// Time passed since the last reboot (in ms).
    fn uptime(&self) -> u64;
}
