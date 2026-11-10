use crate::frame_buffer::FireflyDrawTarget;
use core::convert::Infallible;
use embedded_graphics::Pixel;
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;

const PPB: usize = 2;
const MASK: u8 = 0b1111;

/// A draw target backed by the guest memory.
///
/// # Safety
///
/// We're bypassing the compiler safety checks here by creating a static
/// reference to a dynamically allocate slice (the app's guest memory).
/// In theory, Canvas may outlive the app and start referencing random memory.
/// In practice, Canvas and Memory are both stored in State which is destroyed
/// when the app exits.
pub struct Canvas {
    data: &'static mut [u8],
    pub width: usize,
    pub height: usize,
}

impl Canvas {
    pub fn new(data: &mut [u8], width: u16) -> Self {
        let ptr = data.as_mut_ptr();
        let len = data.len();
        let data = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
        let width = width as usize;
        let height = data.len() * 2 / width;
        Self {
            data,
            width,
            height,
        }
    }
}

impl FireflyDrawTarget for Canvas {
    fn iter_mut(&mut self) -> core::slice::IterMut<'_, u8> {
        self.data.iter_mut()
    }

    fn set_pixel(&mut self, point: Point, luma: u8) {
        let x = point.x as usize;
        let y = point.y as usize;
        if y >= self.height || x >= self.width {
            return; // the pixel is out of bounds
        }
        let pixel_index = y * self.width + x;
        let byte_index = pixel_index / PPB;
        let shift: u8 = if pixel_index.is_multiple_of(2) { 4 } else { 0 };
        let mask = !(MASK << shift);
        // Safety: if y within self.width and self.height (which we checked),
        // the byte_index is is within the buffer.
        let byte = unsafe { self.data.get_unchecked_mut(byte_index) };
        debug_assert!(luma < 16);
        *byte = (luma << shift) | (*byte & mask);
    }

    fn get_pixel(&self, point: Point) -> Option<u8> {
        let x = point.x as usize;
        let y = point.y as usize;
        if y >= self.height || x >= self.width {
            return None; // the pixel is out of bounds
        }
        let pixel_index = y * self.width + x;
        let byte_index = pixel_index / PPB;
        // Safety: if y within self.width and self.height (which we checked),
        // the byte_index is is within the buffer.
        let byte = unsafe { self.data.get_unchecked(byte_index) };
        let c = if pixel_index.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & MASK
        };
        Some(c)
    }

    fn flip_x(&mut self) {
        let byte_w = self.width / 2;
        for y in 0..self.height {
            let line_offset = y * byte_w;
            let next_line_offset = line_offset + byte_w;
            for bx in 0..byte_w / 2 {
                let left_idx = line_offset + bx;
                let right_idx = next_line_offset - bx - 1;
                let left = self.data[left_idx];
                self.data[left_idx] = self.data[right_idx].rotate_left(4);
                self.data[right_idx] = left.rotate_left(4);
            }
        }
    }

    fn flip_y(&mut self) {
        let byte_w = self.width / 2;
        for y in 0..self.height / 2 {
            let top_offset = y * byte_w;
            let bottom_offset = self.data.len() - top_offset - byte_w;
            for bx in 0..byte_w {
                self.data.swap(top_offset + bx, bottom_offset + bx);
            }
        }
    }
}

impl OriginDimensions for Canvas {
    fn size(&self) -> Size {
        Size {
            width: self.width as u32,
            height: self.height as u32,
        }
    }
}

impl DrawTarget for Canvas {
    type Color = Gray4;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let Pixel(point, color) = pixel;
            let luma = color.into_storage();
            self.set_pixel(point, luma);
        }
        Ok(())
    }
}
