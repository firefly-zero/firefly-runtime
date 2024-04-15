use crate::color::FromRGB;
use core::convert::Infallible;
use core::marker::PhantomData;
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
    /// Tightly packed pixel data, 2 bits per pixel (4 pixels per byte).
    pub(crate) data: [u8; BUFFER_SIZE],
    /// The color palette. Maps 4-color packed pixels to 4 RGB colors.
    pub(crate) palette: [Rgb888; 4],
    /// The lowest (by value) Y value of all updated lines.
    pub(crate) dirty_from: usize,
    /// The highest (by value) Y value of all updated lines.
    pub(crate) dirty_to: usize,
}

impl FrameBuffer {
    pub(crate) fn new() -> Self {
        Self {
            data: [0; BUFFER_SIZE],
            // For the first frame, consider all lines dirty.
            dirty_from: 0,
            dirty_to: HEIGHT,
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

/// Required by the [DrawTarget] trait.
impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
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

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let new_byte = color_to_byte(&color);
        for y in 0..HEIGHT {
            if check_line_dirty(&self.data, y, &new_byte) {
                self.dirty_from = self.dirty_from.min(y);
                self.dirty_to = self.dirty_to.max(y);
            }
        }
        self.data.fill(new_byte);
        Ok(())
    }
}

impl FrameBuffer {
    /// Draw the framebuffer on an RGB screen.
    ///
    /// Draws only the region with changed pixels.
    /// After all changed pixels are written,
    /// the whole buffer is marked as clean.
    /// The next call to draw won't draw anything
    /// unless the frame buffer is updated.
    pub(crate) fn draw<D, C, E>(&mut self, target: &mut D) -> Result<(), E>
    where
        C: RgbColor + FromRGB,
        D: DrawTarget<Color = C, Error = E>,
    {
        // If no dirty lines, don't update the screen.
        if self.dirty_from > self.dirty_to {
            self.mark_clean();
            return Ok(());
        }
        let colors = ColorIter {
            data: &self.data,
            palette: &self.palette,
            // start iteration from the first dirty line
            index: WIDTH * self.dirty_from,
            // end iteration at the last dirty line
            max_y: self.dirty_to,
            color: PhantomData,
        };
        let area = Rectangle::new(
            Point::new(0, self.dirty_from as i32),
            Size::new(WIDTH as u32, self.dirty_to as u32),
        );
        let result = target.fill_contiguous(&area, colors);
        // As soon as all lines are rendered on the screen,
        // mark all lines a "clean" so that the next frame knows
        // which lines are updated.
        self.mark_clean();
        result
    }

    /// Mark all lines as clean ("non-dirty").
    ///
    /// The next render won't redraw anything
    /// unless something new is drawn on the buffer.
    pub(crate) fn mark_clean(&mut self) {
        self.dirty_from = HEIGHT;
        self.dirty_to = 0;
    }

    /// Mark all lines as dirty.
    ///
    /// If called, the next frame will render all lines.
    pub(crate) fn mark_dirty(&mut self) {
        self.dirty_from = 0;
        self.dirty_to = HEIGHT;
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
        let new_byte = (color << shift) | (byte & mask);
        if new_byte == byte {
            return;
        }
        self.dirty_from = self.dirty_from.min(y);
        self.dirty_to = self.dirty_to.max(y);
        self.data[byte_index] = new_byte
    }
}

struct ColorIter<'a, C>
where
    C: RgbColor + FromRGB,
{
    data: &'a [u8; BUFFER_SIZE],
    palette: &'a [Rgb888; 4],
    index: usize,
    max_y: usize,
    color: PhantomData<C>,
}

impl<'a, C> Iterator for ColorIter<'a, C>
where
    C: RgbColor + FromRGB,
{
    type Item = C;

    fn next(&mut self) -> Option<Self::Item> {
        let y = self.index / WIDTH;
        if y > self.max_y {
            return None;
        }
        let byte_index = self.index / PPB;
        let byte = self.data.get(byte_index)?;
        let shift = self.index % PPB;
        let luma = (byte >> (shift * BPP)) & 0b11;
        debug_assert!(luma < 4);
        self.index += 1;
        Some(convert_color(self.palette, luma))
    }
}

/// Convert 4-color gray luma into the target RGB color.
fn convert_color<C>(palette: &[Rgb888; 4], luma: u8) -> C
where
    C: RgbColor + FromRGB,
{
    let rgb888 = palette[luma as usize];
    let r = rgb888.r() as u32 * C::MAX_R as u32 / Rgb888::MAX_R as u32;
    let g = rgb888.g() as u32 * C::MAX_G as u32 / Rgb888::MAX_G as u32;
    let b = rgb888.b() as u32 * C::MAX_B as u32 / Rgb888::MAX_B as u32;
    debug_assert!(r < 256);
    debug_assert!(g < 256);
    debug_assert!(b < 256);
    C::from_rgb(r as u8, g as u8, b as u8)
}

/// Duplicate the color 4 times and pack into 1 byte.
fn color_to_byte(c: &Gray2) -> u8 {
    let mut new_byte = 0;
    let luma = c.into_storage();
    for _ in 0..PPB {
        new_byte = (new_byte << BPP) | luma;
    }
    new_byte
}

/// Check the whole horizontal line if it should be marked dirty.
fn check_line_dirty(data: &[u8], y: usize, new_byte: &u8) -> bool {
    let line_start = WIDTH * y / PPB;
    let line_end = line_start + y / PPB;
    let line = &data[line_start..=line_end];
    for old_byte in line {
        if new_byte != old_byte {
            return true;
        }
    }
    false
}
