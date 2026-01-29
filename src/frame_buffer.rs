use crate::color::{FromRGB, Rgb16};
use alloc::boxed::Box;
use core::convert::Infallible;
use core::marker::PhantomData;
use embedded_graphics::pixelcolor::Gray4;
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

// https://lospec.com/palette-list/sweetie-16
// https://github.com/nesbox/TIC-80/wiki/Palette
const DEFAULT_PALETTE: [Rgb16; 16] = [
    Rgb16::from_rgb(0x1a, 0x1c, 0x2c), // #1a1c2c, black
    Rgb16::from_rgb(0x5d, 0x27, 0x5d), // #5d275d, purple
    Rgb16::from_rgb(0xb1, 0x3e, 0x53), // #b13e53, red
    Rgb16::from_rgb(0xef, 0x7d, 0x57), // #ef7d57, orange
    Rgb16::from_rgb(0xff, 0xcd, 0x75), // #ffcd75, yellow
    Rgb16::from_rgb(0xa7, 0xf0, 0x70), // #a7f070, light green
    Rgb16::from_rgb(0x38, 0xb7, 0x64), // #38b764, green
    Rgb16::from_rgb(0x25, 0x71, 0x79), // #257179, dark green
    Rgb16::from_rgb(0x29, 0x36, 0x6f), // #29366f, dark blue
    Rgb16::from_rgb(0x3b, 0x5d, 0xc9), // #3b5dc9, blue
    Rgb16::from_rgb(0x41, 0xa6, 0xf6), // #41a6f6, light blue
    Rgb16::from_rgb(0x73, 0xef, 0xf7), // #73eff7, cyan
    Rgb16::from_rgb(0xf4, 0xf4, 0xf4), // #f4f4f4, white
    Rgb16::from_rgb(0x94, 0xb0, 0xc2), // #94b0c2, light gray
    Rgb16::from_rgb(0x56, 0x6c, 0x86), // #566c86, gray
    Rgb16::from_rgb(0x33, 0x3c, 0x57), // #333c57, dark gray
];

pub trait RenderFB {
    type Error;
    fn render_fb(&mut self, frame: &mut FrameBuffer) -> Result<(), Self::Error>;
}

pub struct FrameBuffer {
    /// Tightly packed pixel data, 4 bits per pixel (2 pixels per byte).
    pub(crate) data: Box<[u8; BUFFER_SIZE]>,
    /// The color palette. Maps 16-color packed pixels to RGB colors.
    pub(crate) palette: [Rgb16; 16],
    pub(crate) dirty: bool,
}

impl FrameBuffer {
    pub(crate) fn new() -> Self {
        Self {
            data: Box::new([0; BUFFER_SIZE]),
            palette: DEFAULT_PALETTE,
            dirty: false,
        }
    }

    pub fn iter_pairs(&self) -> impl Iterator<Item = (Rgb16, Rgb16)> + use<'_> {
        self.data.iter().map(|b| {
            let right = self.palette[usize::from(b & 0xf)];
            let left = self.palette[usize::from(b >> 4) & 0xf];
            (right, left)
        })
    }

    /// Optimized rendering of horizontal line.
    ///
    /// Must behave exactly like embedded-graphics with the same parameters
    /// but damn faster.
    pub(crate) fn draw_hline(&mut self, x1: i32, x2: i32, y: i32, w: u32, c: Gray4) {
        let mut left = x1;
        let mut right = x2;
        let mut y = y - (w / 2) as i32;
        if left > right {
            (left, right) = (right, left);
            if w.is_multiple_of(2) {
                y += 1;
            }
        }
        let area = Rectangle {
            top_left: Point::new(left, y),
            size: Size::new((right - left + 1) as u32, w),
        };
        _ = self.fill_solid(&area, c);
    }

    /// Optimized rendering of vertical line.
    ///
    /// Must behave exactly like embedded-graphics with the same parameters
    /// but damn faster.
    pub(crate) fn draw_vline(&mut self, x: i32, y1: i32, y2: i32, w: u32, c: Gray4) {
        let mut top = y1;
        let mut down = y2;
        let mut x = x - (w / 2) as i32;
        if top > down {
            (down, top) = (top, down);
            if w.is_multiple_of(2) {
                x += 1;
            }
        }
        let area = Rectangle {
            top_left: Point::new(x, top),
            size: Size::new(w, (down - top + 1) as u32),
        };
        _ = self.fill_solid(&area, c);
    }

    /// Render 1-pixel-wide vertical line.
    ///
    /// SAFETY: HEIGHT > bottom_y >= top_y.
    fn draw_vline1(&mut self, x: usize, top_y: usize, bottom_y: usize, c: Gray4) {
        let color = c.into_storage();
        debug_assert!(color < 16);
        let shift = if x.is_multiple_of(2) { 0 } else { 4 };
        let mask = !(0b1111 << shift);
        let start_i = (top_y * WIDTH + x) / PPB;
        let end_i = (bottom_y * WIDTH + x) / PPB;
        for pixel_index in (start_i..end_i).step_by(WIDTH / PPB) {
            let byte_index = pixel_index;
            // Safety: It's up to the caller to ensure that
            // y within WIDTH and HEIGHT.
            let byte = unsafe { self.data.get_unchecked_mut(byte_index) };
            *byte = (color << shift) | (*byte & mask);
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
        self.dirty = true;
        for pixel in pixels {
            let Pixel(point, color) = pixel;
            self.set_pixel(point, color);
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

        let left_x = area.top_left.x.clamp(0, WIDTH as _) as usize;
        let right_x = area.top_left.x + area.size.width as i32;
        let right_x = right_x.clamp(0, WIDTH as _) as usize;

        let top_y = area.top_left.y.clamp(0, HEIGHT as _) as usize;
        let bottom_y = area.top_left.y + area.size.height as i32;
        let bottom_y = bottom_y.clamp(0, HEIGHT as _) as usize;

        if right_x - left_x <= 4 {
            for x in left_x..right_x {
                self.draw_vline1(x, top_y, bottom_y, color);
            }
            return Ok(());
        }

        let left_fract = left_x % 2 == 1;
        let right_fract = right_x % 2 == 1;
        let start_x = left_x + usize::from(left_fract);
        let end_x = right_x - usize::from(right_fract);
        let width = end_x - start_x;
        debug_assert_eq!(width % 2, 0);

        for y in top_y..bottom_y {
            let start_i = y * WIDTH / 2 + start_x / 2;
            let end_i = start_i + width / 2;
            self.data[start_i..end_i].fill(new_byte);
        }

        if left_fract {
            self.draw_vline1(left_x, top_y, bottom_y, color);
        }
        if right_fract {
            self.draw_vline1(right_x - 1, top_y, bottom_y, color);
        }
        Ok(())
    }

    // TODO(@orsinium): Optimize. Used by draw_qr.
    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.draw_iter(
            area.points()
                .zip(colors)
                .map(|(pos, color)| Pixel(pos, color)),
        )
    }
}

impl FrameBuffer {
    /// Draw the framebuffer on an RGB screen.
    pub fn draw<D, C, E>(&mut self, target: &mut D) -> Result<(), E>
    where
        C: RgbColor + FromRGB,
        D: DrawTarget<Color = C, Error = E>,
    {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        let colors = ColorIter {
            data: &self.data,
            palette: &self.palette,
            index: 0,
            color: PhantomData,
        };
        let area = Rectangle::new(Point::zero(), Size::new(WIDTH as u32, HEIGHT as u32));
        target.fill_contiguous(&area, colors)
    }

    /// Set color of a single pixel at the given coordinates.
    ///
    /// Does NOT mark the buffer as dirty. This must be done by the caller.
    pub(crate) fn set_pixel(&mut self, point: Point, color: Gray4) {
        // Negative values will be wrapped and filtered out
        // because any wrapped value is bigger than WIDTH/HEIGHT.
        let x = point.x as usize;
        let y = point.y as usize;
        if y >= HEIGHT || x >= WIDTH {
            return; // the pixel is out of bounds
        }
        let pixel_index = y * WIDTH + x;
        let byte_index = pixel_index / PPB;
        let shift = if pixel_index.is_multiple_of(2) { 0 } else { 4 };
        let mask = !(0b1111 << shift);
        // Safety: if y within WIDTH and HEIGHT (which we checked),
        // the byte_index is is within the buffer.
        let byte = unsafe { self.data.get_unchecked_mut(byte_index) };
        let color = color.luma();
        debug_assert!(color < 16);
        *byte = (color << shift) | (*byte & mask);
    }
}

struct ColorIter<'a, C>
where
    C: RgbColor + FromRGB,
{
    data: &'a [u8; BUFFER_SIZE],
    palette: &'a [Rgb16; 16],
    index: usize,
    color: PhantomData<C>,
}

impl<C> Iterator for ColorIter<'_, C>
where
    C: RgbColor + FromRGB,
{
    type Item = C;

    fn next(&mut self) -> Option<Self::Item> {
        let byte_index = self.index / PPB;
        let byte = self.data.get(byte_index)?;
        let shift = self.index % PPB;
        let luma = (byte >> (shift * BPP)) & 0b1111;
        debug_assert!(luma < 16);
        self.index += 1;
        let rgb16 = self.palette[luma as usize];
        Some(C::from_rgb(rgb16))
    }
}

/// Duplicate the color and pack into 1 byte.
fn color_to_byte(c: &Gray4) -> u8 {
    let luma = c.luma();
    luma | (luma << 4)
}
