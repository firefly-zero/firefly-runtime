use crate::color::FromRGB;
use alloc::boxed::Box;
use core::convert::Infallible;
use core::marker::PhantomData;
use embedded_graphics::pixelcolor::{Gray4, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;
/// Bits per pixel.
const BPP: usize = 4;
/// Pixels per byte.
const PPB: usize = 8 / BPP;
/// Bytes needed to store all pixels.
const BUFFER_SIZE: usize = WIDTH * HEIGHT / PPB;

pub(crate) struct FrameBuffer {
    /// Tightly packed pixel data, 4 bits per pixel (2 pixels per byte).
    pub(crate) data: Box<[u8; BUFFER_SIZE]>,
    /// The color palette. Maps 16-color packed pixels to RGB colors.
    pub(crate) palette: [Rgb888; 16],
    dirty: bool,
}

impl FrameBuffer {
    pub(crate) fn new() -> Self {
        Self {
            data: Box::new([0; BUFFER_SIZE]),
            palette: [
                // https://lospec.com/palette-list/sweetie-16
                // https://github.com/nesbox/TIC-80/wiki/Palette
                Rgb888::new(0x1a, 0x1c, 0x2c), // black
                Rgb888::new(0x5d, 0x27, 0x5d), // purple
                Rgb888::new(0xb1, 0x3e, 0x53), // red
                Rgb888::new(0xef, 0x7d, 0x57), // orange
                Rgb888::new(0xff, 0xcd, 0x75), // yellow
                Rgb888::new(0xa7, 0xf0, 0x70), // light green
                Rgb888::new(0x38, 0xb7, 0x64), // green
                Rgb888::new(0x25, 0x71, 0x79), // dark green
                Rgb888::new(0x29, 0x36, 0x6f), // dark blue
                Rgb888::new(0x3b, 0x5d, 0xc9), // blue
                Rgb888::new(0x41, 0xa6, 0xf6), // light blue
                Rgb888::new(0x73, 0xef, 0xf7), // cyan
                Rgb888::new(0xf4, 0xf4, 0xf4), // white
                Rgb888::new(0x94, 0xb0, 0xc2), // light gray
                Rgb888::new(0x56, 0x6c, 0x86), // gray
                Rgb888::new(0x33, 0x3c, 0x57), // dark gray
            ],
            dirty: false,
        }
    }
}

/// Required by the [DrawTarget] trait.
impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

/// Allow drawing 16-color elements on the framebuffer.
impl DrawTarget for FrameBuffer {
    type Color = Gray4;
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
        self.dirty = true;
        let new_byte = color_to_byte(&color);
        self.data.fill(new_byte);
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.dirty = true;
        let new_byte = color_to_byte(&color);

        let left_x = area.top_left.x as usize;
        let left_x = left_x.clamp(0, WIDTH);
        let right_x = left_x + area.size.width as usize;
        let right_x = right_x.clamp(0, WIDTH);
        let top_y = area.top_left.y as usize;
        let top_y = top_y.clamp(0, HEIGHT);
        let bottom_y = top_y + area.size.height as usize;
        let bottom_y = bottom_y.clamp(0, HEIGHT);

        if right_x - left_x <= 4 {
            for point in area.points() {
                self.set_pixel(Pixel(point, color));
            }
            return Ok(());
        }

        let left_fract = left_x % 2 == 1;
        let right_fract = right_x % 2 == 1;
        let start_x = left_x + if left_fract { 1 } else { 0 };
        let end_x = right_x - if right_fract { 1 } else { 0 };
        let width = end_x - start_x;
        debug_assert_eq!(width % 2, 0);

        for y in top_y..bottom_y {
            let start_i = y * WIDTH / 2 + start_x / 2;
            let end_i = start_i + width / 2;
            self.data[start_i..end_i].fill(new_byte);
        }

        if left_fract {
            for y in top_y..bottom_y {
                let x = left_x as i32;
                let y = y as i32;
                self.set_pixel(Pixel(Point { x, y }, color));
            }
        }

        if right_fract {
            for y in top_y..bottom_y {
                let x = right_x as i32 - 1;
                let y = y as i32;
                self.set_pixel(Pixel(Point { x, y }, color));
            }
        }

        Ok(())
    }
}

impl FrameBuffer {
    /// Draw the framebuffer on an RGB screen.
    ///
    /// Draws only the region with changed pixels. After all changed pixels are written,
    /// the whole buffer is marked as clean. The next call to draw won't draw anything
    /// unless the frame buffer is updated.
    pub(crate) fn draw<D, C, E>(&mut self, target: &mut D) -> Result<(), E>
    where
        C: RgbColor + FromRGB,
        D: DrawTarget<Color = C, Error = E>,
    {
        self.draw_range(target, 0, HEIGHT + 1)
    }

    pub(crate) fn draw_range<D, C, E>(
        &mut self,
        target: &mut D,
        min_y: usize,
        max_y: usize,
    ) -> Result<(), E>
    where
        C: RgbColor + FromRGB,
        D: DrawTarget<Color = C, Error = E>,
    {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        // If the range is empty, don't update the screen.
        if min_y > max_y {
            return Ok(());
        }
        let colors = ColorIter {
            data: &self.data,
            palette: &self.palette,
            // start iteration from the first line in range
            index: WIDTH * min_y,
            // end iteration at the last line in range
            max_y,
            color: PhantomData,
        };
        let area = Rectangle::new(
            Point::new(0, min_y as i32),
            Size::new(WIDTH as u32, max_y as u32),
        );
        target.fill_contiguous(&area, colors)
    }

    /// Set color of a single pixel at the given coordinates.
    fn set_pixel(&mut self, pixel: Pixel<Gray4>) {
        // TODO: move it to caller funcs
        self.dirty = true;
        let Pixel(point, color) = pixel;
        let x = point.x as usize;
        let y = point.y as usize;
        if y >= HEIGHT || x >= WIDTH {
            return; // the pixel is out of bounds
        }
        let pixel_index = y * WIDTH + x;
        let byte_index = pixel_index / PPB;
        let shift = if pixel_index % 2 == 0 { 0 } else { 4 };
        let mask = !(0b1111 << shift);
        let byte = self.data[byte_index];
        let color = color.into_storage();
        debug_assert!(color < 16);
        let new_byte = (color << shift) | (byte & mask);
        if new_byte == byte {
            return;
        }
        self.data[byte_index] = new_byte
    }
}

struct ColorIter<'a, C>
where
    C: RgbColor + FromRGB,
{
    data: &'a [u8; BUFFER_SIZE],
    palette: &'a [Rgb888; 16],
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
        let luma = (byte >> (shift * BPP)) & 0b1111;
        debug_assert!(luma < 16);
        self.index += 1;
        Some(convert_color(self.palette, luma))
    }
}

/// Convert 16-color gray luma into the target RGB color.
fn convert_color<C>(palette: &[Rgb888; 16], luma: u8) -> C
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

/// Duplicate the color and pack into 1 byte.
fn color_to_byte(c: &Gray4) -> u8 {
    let mut new_byte = 0;
    let luma = c.into_storage();
    for _ in 0..PPB {
        new_byte = (new_byte << BPP) | luma;
    }
    new_byte
}
