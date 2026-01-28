use crate::{FrameBuffer, HEIGHT, WIDTH};
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub struct ParsedImage<'a> {
    pub bpp: u8,
    pub bytes: &'a [u8],
    pub width: u32,
    pub swaps: &'a [u8],
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
        let ppb = (8 / self.bpp) as usize;
        let swaps = parse_swaps(self.transp, self.swaps);
        let mut p = point;
        let mut image = self.bytes;

        // Cut the top out-of-bounds part of the image.
        if p.y < 0 {
            let start_i = (-p.y * self.width as i32) as usize / ppb;
            let Some(sub_image) = image.get(start_i..) else {
                return;
            };
            image = sub_image;
            p.y = 0;
        }

        // Cut the bottom out-of-bounds part of the image.
        let height = (image.len() * ppb) as i32 / self.width as i32;
        let bottom_y = p.y + height;
        if bottom_y > HEIGHT as i32 {
            let new_height = height - (bottom_y - HEIGHT as i32);
            let end_i = (new_height * self.width as i32) as usize / ppb;
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
            skip = skip_px / ppb;
            right_x = WIDTH as i32 + (skip_px % ppb) as i32;
        }

        // Skip the left out-of-bounds part of the image.
        let mut left_x = point.x;
        if left_x < 0 {
            let skip_px = -left_x as usize;
            skip += skip_px / ppb;
            left_x = -((skip_px % ppb) as i32);
        }

        let mut i = 0;
        let bpp = self.bpp as u32;
        let mask = match self.bpp {
            1 => 0b1,
            2 => 0b11,
            _ => 0b1111,
        };
        while i < image.len() {
            let mut byte = image[i];
            for _ in 0..ppb {
                byte = byte.rotate_left(bpp);
                let c1 = usize::from(byte & mask);
                if let Some(c1) = swaps[c1] {
                    frame.set_pixel(p, c1);
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
        let bpp = self.bpp as usize;
        let ppb = 8 / bpp;

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
        let img_height = self.bytes.len() * ppb / self.width as usize;
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

        let mask = match bpp {
            1 => 0b1,
            2 => 0b11,
            _ => 0b1111,
        };
        let swaps = parse_swaps(self.transp, self.swaps);
        for iy in top..bottom {
            for ix in left..right {
                let offset = (iy * self.width as i32 + ix) as usize;
                let bytes_offset = offset / ppb;
                let byte = self.bytes[bytes_offset];
                let pixel_offset = 8 - bpp * (1 + offset % ppb);
                let color_idx = (byte >> pixel_offset) & mask;
                if let Some(color) = swaps[color_idx as usize] {
                    let fx = p.x + (ix - left);
                    let fy = p.y + (iy - top);
                    frame.set_pixel(Point::new(fx, fy), color);
                };
            }
        }
        frame.dirty = true;
    }
}

#[allow(clippy::get_first)]
pub(crate) fn parse_swaps(transp: u8, swaps: &[u8]) -> [Option<Gray4>; 16] {
    [
        // 0-4
        parse_color_l(transp, swaps.get(0)),
        parse_color_r(transp, swaps.get(0)),
        parse_color_l(transp, swaps.get(1)),
        parse_color_r(transp, swaps.get(1)),
        // 4-8
        parse_color_l(transp, swaps.get(2)),
        parse_color_r(transp, swaps.get(2)),
        parse_color_l(transp, swaps.get(3)),
        parse_color_r(transp, swaps.get(3)),
        // 8-12
        parse_color_l(transp, swaps.get(4)),
        parse_color_r(transp, swaps.get(4)),
        parse_color_l(transp, swaps.get(5)),
        parse_color_r(transp, swaps.get(5)),
        // 12-16
        parse_color_l(transp, swaps.get(6)),
        parse_color_r(transp, swaps.get(6)),
        parse_color_l(transp, swaps.get(7)),
        parse_color_r(transp, swaps.get(7)),
    ]
}

/// Parse the high bits of a byte as a color.
fn parse_color_r(transp: u8, c: Option<&u8>) -> Option<Gray4> {
    let c = c?;
    let c = c & 0b1111;
    if c == transp {
        return None;
    }
    Some(Gray4::new(c))
}

/// Parse the low bits of a byte as a color.
fn parse_color_l(transp: u8, c: Option<&u8>) -> Option<Gray4> {
    let c = c?;
    let c = (c >> 4) & 0b1111;
    if c == transp {
        return None;
    }
    Some(Gray4::new(c))
}
