use crate::color::FromRGB;
use core::convert::Infallible;
use core::marker::PhantomData;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::image::GetPixel;
use embedded_graphics::pixelcolor::{Gray2, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;
/// Bits per pixel.
const BPP: usize = 2;
/// Pixels per byte.
const PPB: usize = 8 / BPP;
/// Bytes needed to store all pixels.
const BUFFER_SIZE: usize = WIDTH * HEIGHT / PPB;

pub(crate) struct FrameBuffer {
    pub(crate) data:    [u8; BUFFER_SIZE],
    pub(crate) palette: [Rgb888; 4],
}

impl FrameBuffer {
    pub(crate) fn new() -> Self {
        Self {
            data:    [0; BUFFER_SIZE],
            palette: [
                // https://lospec.com/palette-list/kirokaze-gameboy
                Rgb888::new(0x33, 0x2c, 0x50),
                Rgb888::new(0x46, 0x87, 0x8f),
                Rgb888::new(0x94, 0xe3, 0x44),
                Rgb888::new(0xe2, 0xf3, 0xe4),
            ],
        }
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl GetPixel for FrameBuffer {
    type Color = Gray2;

    fn pixel(&self, point: Point) -> Option<Self::Color> {
        if point.x < 0 || point.y < 0 {
            return None;
        }
        let x = point.x as usize;
        let y = point.y as usize;
        if x >= WIDTH || y >= HEIGHT {
            return None;
        }
        let pixel_index = y * WIDTH + x;
        let byte_index = pixel_index / PPB;
        let byte = self.data[byte_index];
        let shift = pixel_index % PPB;
        let luma = (byte >> (shift * BPP)) & 0b11;
        Some(Gray2::new(luma))
    }
}

/// Allow drawing 4-color elements on the framebuffer.
impl DrawTarget for FrameBuffer {
    type Color = Gray2;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            self.set_pixel(pixel);
        }
        Ok(())
    }
}

impl FrameBuffer {
    /// Draw the framebuffer on an RGB screen.
    pub(crate) fn draw<D, C, E>(&self, target: &mut D) -> Result<(), E>
    where
        C: RgbColor + FromRGB,
        D: DrawTarget<Color = C, Error = E>,
    {
        let colors = ColorIter {
            data:    &self.data,
            palette: &self.palette,
            index:   0,
            color:   PhantomData,
        };
        let area = Rectangle::new(Point::zero(), self.size());
        target.fill_contiguous(&area, colors)
    }

    /// Set color of a single pixel at the given coordinates.
    fn set_pixel(&mut self, pixel: Pixel<Gray2>) {
        let Pixel(point, color) = pixel;
        let x = point.x as usize;
        let y = point.y as usize;
        if y >= HEIGHT || x >= WIDTH {
            return; // the pixel is out of bounds
        }
        let pixel_index = y * WIDTH + x;
        let byte_index = pixel_index / PPB;
        let shift = (pixel_index as u8 & 0b11) << 1;
        let mask = !(0b11 << shift);
        let byte = self.data[byte_index];
        let color = color.into_storage();
        debug_assert!(color < 4);
        self.data[byte_index] = (color << shift) | (byte & mask);
    }
}

struct ColorIter<'a, C>
where
    C: RgbColor + FromRGB,
{
    data:    &'a [u8; BUFFER_SIZE],
    palette: &'a [Rgb888; 4],
    index:   usize,
    color:   PhantomData<C>,
}

impl<'a, C> Iterator for ColorIter<'a, C>
where
    C: RgbColor + FromRGB,
{
    type Item = C;

    fn next(&mut self) -> Option<Self::Item> {
        let byte_index = self.index / PPB;
        let byte = self.data.get(byte_index)?;
        let shift = self.index % PPB;
        let luma = (byte >> (shift * BPP)) & 0b11;
        debug_assert!(luma < 4);
        let rgb888 = self.palette[luma as usize];
        let r = rgb888.r() as u32 * C::MAX_R as u32 / Rgb888::MAX_R as u32;
        let g = rgb888.g() as u32 * C::MAX_G as u32 / Rgb888::MAX_G as u32;
        let b = rgb888.b() as u32 * C::MAX_B as u32 / Rgb888::MAX_B as u32;
        debug_assert!(r < 256);
        debug_assert!(g < 256);
        debug_assert!(b < 256);
        self.index += 1;
        Some(C::from_rgb(r as u8, g as u8, b as u8))
    }
}
