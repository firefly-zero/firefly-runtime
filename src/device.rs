pub struct Device<Display, Delay, Storage>
where
    Display: embedded_graphics::draw_target::DrawTarget,
    Delay: Fn(u32),
    Storage: embedded_storage::Storage,
{
    pub display:  Display,
    pub delay_ms: Delay,
    pub storage:  Storage,
}
