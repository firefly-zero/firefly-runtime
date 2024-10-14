use core::convert::Infallible;

use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;
use embedded_graphics::Pixel;

use crate::state::State;

const PPB: usize = 2;

/// A draw target backed by the guest memory.
#[derive(Clone)]
pub struct Canvas {
    start: usize,
    end: usize,
    width: usize,
}

impl Canvas {
    pub fn new(start: u32, size: u32, width: u32) -> Self {
        Self {
            start: start as usize,
            end: (start + size) as usize,
            width: width as usize,
        }
    }

    /// Make a draw target that modifies the data inside the canvas.
    pub fn as_target<'a>(&self, caller: &'a mut wasmi::Caller<'_, State>) -> CanvasBuffer<'a> {
        let state = caller.data();
        // safety: memory presence is ensured in set_canvas
        let memory = state.memory.unwrap();
        let memory = memory.data_mut(caller);
        let data = &mut memory[self.start..self.end];
        let height = data.len() * 2 / self.width;
        CanvasBuffer {
            data,
            width: self.width,
            height,
        }
    }
}

/// A wrapper for drawing onto the canvas.
///
/// It works just like the frame buffer but the width and height are defined at runtime.
pub struct CanvasBuffer<'a> {
    data: &'a mut [u8],
    width: usize,
    height: usize,
}

impl<'a> OriginDimensions for CanvasBuffer<'a> {
    fn size(&self) -> Size {
        Size {
            width: self.width as u32,
            height: self.height as u32,
        }
    }
}

impl<'a> DrawTarget for CanvasBuffer<'a> {
    type Color = Gray4;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            self.set_pixel(pixel)
        }
        Ok(())
    }
}

impl<'a> CanvasBuffer<'a> {
    fn set_pixel(&mut self, pixel: Pixel<Gray4>) {
        let Pixel(point, color) = pixel;
        let x = point.x as usize;
        let y = point.y as usize;
        if y >= self.height || x >= self.width {
            return; // the pixel is out of bounds
        }
        let pixel_index = y * self.width + x;
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
