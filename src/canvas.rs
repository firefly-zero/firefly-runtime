use crate::state::State;
use alloc::boxed::Box;
use core::convert::Infallible;
use embedded_graphics::Pixel;
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;

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
    pub fn as_target<'a>(&self, caller: &'a mut wasmi::Caller<'_, Box<State>>) -> CanvasBuffer<'a> {
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

impl OriginDimensions for CanvasBuffer<'_> {
    fn size(&self) -> Size {
        Size {
            width: self.width as u32,
            height: self.height as u32,
        }
    }
}

impl DrawTarget for CanvasBuffer<'_> {
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

impl CanvasBuffer<'_> {
    pub(crate) fn set_pixel(&mut self, point: Point, luma: u8) {
        let x = point.x as usize;
        let y = point.y as usize;
        if y >= self.height || x >= self.width {
            return; // the pixel is out of bounds
        }
        let pixel_index = y * self.width + x;
        let byte_index = pixel_index / PPB;
        let shift = if pixel_index.is_multiple_of(2) { 4 } else { 0 };
        let mask = !(0b1111 << shift);
        // Safety: if y within self.width and self.height (which we checked),
        // the byte_index is is within the buffer.
        let byte = unsafe { self.data.get_unchecked_mut(byte_index) };
        debug_assert!(luma < 16);
        *byte = (luma << shift) | (*byte & mask);
    }
}
