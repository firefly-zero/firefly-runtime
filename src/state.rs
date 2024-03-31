use embedded_graphics::framebuffer::{buffer_size, Framebuffer};
use embedded_graphics::pixelcolor::raw::{LittleEndian, RawU2};
use embedded_graphics::pixelcolor::{Gray2, Rgb888};

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;
const BUFFER_SIZE: usize = buffer_size::<Gray2>(WIDTH, HEIGHT);
pub(crate) type Frame = Framebuffer<Gray2, RawU2, LittleEndian, WIDTH, HEIGHT, BUFFER_SIZE>;

pub(crate) struct State {
    pub frame:   Frame,
    pub palette: [Rgb888; 4],
    pub memory:  Option<wasmi::Memory>,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            frame:   Framebuffer::new(),
            palette: [
                // https://lospec.com/palette-list/kirokaze-gameboy
                Rgb888::new(0x33, 0x2c, 0x50),
                Rgb888::new(0x46, 0x87, 0x8f),
                Rgb888::new(0x94, 0xe3, 0x44),
                Rgb888::new(0xe2, 0xf3, 0xe4),
            ],
            memory:  None,
        }
    }
}
