use embedded_graphics::draw_target::DrawTarget;

pub struct Device<Display, Delay>
where
    Display: DrawTarget,
    Delay: Fn(u32),
{
    pub display:  Display,
    pub delay_ms: Delay,
}
