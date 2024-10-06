use embedded_graphics::{image::ImageRawLE, pixelcolor::Gray4};

/// A draw target backed by the guest memory.
pub struct Canvas {
    start: usize,
    end: usize,
    width: u32,
}

impl Canvas {
    pub fn new(start: u32, size: u32, width: u32) -> Self {
        Self {
            start: start as usize,
            end: (start + size) as usize,
            width,
        }
    }

    pub fn as_image<'a>(&self, memory: &'a [u8]) -> ImageRawLE<'a, Gray4> {
        let data = &memory[self.start..self.end];
        ImageRawLE::new(data, self.width)
    }
}
