use crate::{FrameBuffer, HEIGHT, WIDTH};
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

const BPP: usize = 4;
const PPB: usize = 2;

pub struct ParsedImage<'a> {
    pub bytes: &'a [u8],
    pub width: u32,
    pub transp: u8,
    pub sub: Option<Rectangle>,
}

impl ParsedImage<'_> {
    pub fn render(&self, point: Point, frame: &mut FrameBuffer) {
        if let Some(sub) = self.sub {
            self.draw_sub_fast(point, sub, frame);
        } else {
            self.draw_fast(point, frame);
        }
    }

    /// Faster implementation of drawing of a 4 BPP image.
    ///
    /// Avoids going through embedded-graphics machinery and instead
    /// iterates over image bytes directly.
    fn draw_fast(&self, point: Point, frame: &mut FrameBuffer) {
        let mut p = point;
        let mut image = self.bytes;

        // Cut the top out-of-bounds part of the image.
        if p.y < 0 {
            let start_i = (-p.y * self.width as i32) as usize / PPB;
            let Some(sub_image) = image.get(start_i..) else {
                return;
            };
            image = sub_image;
            p.y = 0;
        }

        // Cut the bottom out-of-bounds part of the image.
        let height = (image.len() * PPB) as i32 / self.width as i32;
        let bottom_y = p.y + height;
        if bottom_y > HEIGHT as i32 {
            let new_height = height - (bottom_y - HEIGHT as i32);
            let end_i = (new_height * self.width as i32) as usize / PPB;
            let Some(sub_image) = image.get(..end_i) else {
                return;
            };
            image = sub_image;
        }

        let mut skip: usize = 0;

        // Skip the right out-of-bounds part of the image.
        let mut right_x = point.x + self.width as i32;
        if right_x > WIDTH as i32 {
            let skip_px = (right_x - WIDTH as i32) as usize;
            skip = skip_px / PPB;
            right_x = WIDTH as i32 + (skip_px % PPB) as i32;
        }

        // Skip the left out-of-bounds part of the image.
        let mut left_x = point.x;
        if left_x < 0 {
            let skip_px = -left_x as usize;
            skip += skip_px / PPB;
            left_x = -((skip_px % PPB) as i32);
        }

        // Check if the image bytes are aligned with frame buffer bytes
        // and no image bytes are cut in half.
        let is_aligned = point.x % 2 == 0 && left_x % 2 == 0 && right_x % 2 == 0;

        // A faster implementation for when
        // no transparency is used and the image is aligned.
        if self.transp > 15 && is_aligned && p.x >= 0 {
            let line_bytes = (right_x - left_x) as usize / PPB;
            let mut target = &mut frame.data[..];
            let target_offset = (p.y as usize * WIDTH + p.x as usize) / PPB;
            target = &mut target[target_offset..];
            while !image.is_empty() {
                for (i, byte) in image[..line_bytes].iter().enumerate() {
                    target[i] = byte.rotate_right(4);
                }
                if target.len() < WIDTH / PPB {
                    break;
                }
                target = &mut target[WIDTH / PPB..];
                image = &image[self.width as usize / PPB..]
            }
            frame.dirty = true;
        }

        let mut i = 0;
        while i < image.len() {
            let mut byte = image[i];
            for _ in 0..PPB {
                byte = byte.rotate_left(BPP as u32);
                let luma = byte & 0b1111;
                if luma != self.transp {
                    let color = Gray4::new(luma);
                    frame.set_pixel(p, color);
                };
                p.x += 1;
                if p.x >= right_x {
                    p.x = left_x;
                    p.y += 1;
                    i += skip;
                }
            }
            i += 1;
        }
        frame.dirty = true;
    }

    fn draw_sub_fast(&self, point: Point, sub: Rectangle, frame: &mut FrameBuffer) {
        let mut p = point;
        let mut top = sub.top_left.y;
        let mut left = sub.top_left.x;
        let mut width = sub.size.width as i32;
        let mut height = sub.size.height as i32;

        // Adjust top boundaries.
        if p.y < 0 {
            top -= p.y;
            height += p.y;
            p.y = 0;
        }

        // Adjust bottom boundaries.
        let img_height = self.bytes.len() * PPB / self.width as usize;
        let max_height = img_height as i32 - top;
        if height > max_height {
            height = max_height;
        }
        let oob_bottom = (p.y + height) - HEIGHT as i32;
        if oob_bottom > 0 {
            height -= oob_bottom;
            if height <= 0 {
                return;
            }
        }
        let bottom = top + height;
        if bottom < 0 {
            return;
        }

        // Adjust left boundaries.
        if p.x < 0 {
            left -= p.x;
            width += p.x;
            p.x = 0;
        }

        // Adjust right boundaries.
        let max_width = self.width as i32 - left;
        if width > max_width {
            width = max_width;
        }
        let oob_right = (p.y + width) - WIDTH as i32;
        if oob_right > 0 {
            width -= oob_right;
            if width <= 0 {
                return;
            }
        }
        let right = left + width;
        if right < 0 {
            return;
        }

        for iy in top..bottom {
            for ix in left..right {
                let offset = (iy * self.width as i32 + ix) as usize;
                let bytes_offset = offset / PPB;
                let byte = self.bytes[bytes_offset];
                let pixel_offset = 8 - BPP * (1 + offset % PPB);
                let luma = (byte >> pixel_offset) & 0b1111;
                if luma != self.transp {
                    let color = Gray4::new(luma);
                    let fx = p.x + (ix - left);
                    let fy = p.y + (iy - top);
                    frame.set_pixel(Point::new(fx, fy), color);
                }
            }
        }
        frame.dirty = true;
    }
}
