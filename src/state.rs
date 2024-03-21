use embedded_graphics::framebuffer::{buffer_size, Framebuffer};
use embedded_graphics::pixelcolor::raw::{LittleEndian, RawU2};
use embedded_graphics::pixelcolor::Gray2;

const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const BUFFER_SIZE: usize = buffer_size::<Gray2>(WIDTH, HEIGHT);

pub struct State {
    pub(crate) frame: Framebuffer<Gray2, RawU2, LittleEndian, WIDTH, HEIGHT, BUFFER_SIZE>,
}

impl State {
    pub fn new() -> Self {
        Self {
            frame: Framebuffer::new(),
        }
    }
}
