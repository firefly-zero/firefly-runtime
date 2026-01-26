use crate::{FrameBuffer, HEIGHT, WIDTH};
use core::convert::Infallible;
use core::marker::PhantomData;
use embedded_graphics::image::{Image, ImageRawLE};
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
        match self.bpp {
            1 => {
                let image_raw = ImageRawLE::<BinaryColor>::new(self.bytes, self.width);
                self.draw_bpp(image_raw, point, frame)
            }
            2 => {
                let image_raw = ImageRawLE::<Gray2>::new(self.bytes, self.width);
                self.draw_bpp(image_raw, point, frame)
            }
            4 => {
                frame.dirty = true;
                self.draw_4bpp_fast(point, frame);
            }
            _ => {}
        };
    }

    /// Draw the raw image at the given point into the target.
    ///
    /// The function takes care of the BPP differences between the given image
    /// and the target.
    fn draw_bpp<C, I, T>(&self, image_raw: I, point: Point, target: &mut T)
    where
        C: PixelColor + IntoStorage<Storage = u8>,
        I: ImageDrawable<Color = C>,
        T: DrawTarget<Color = Gray4, Error = Infallible> + OriginDimensions,
    {
        let mut adapter = BPPAdapter::<_, C>::new(target, self.transp, self.swaps);
        match self.sub {
            Some(sub) => {
                let image_raw = image_raw.sub_image(&sub);
                let image = Image::new(&image_raw, point);
                never_fails(image.draw(&mut adapter));
            }
            None => {
                let image = Image::new(&image_raw, point);
                never_fails(image.draw(&mut adapter));
            }
        }
    }

    /// Faster implementation of drawing of a 4 BPP image.
    ///
    /// Avoids going through embedded-graphics machinery and instead
    /// iterates over image bytes directly.
    fn draw_4bpp_fast(&self, point: Point, frame: &mut FrameBuffer) {
        const PPB: usize = 2;
        let swaps = parse_swaps(self.transp, self.swaps);
        let mut p = point;
        let mut image = self.bytes;

        // Variables for cutting the left and right sides of the image.
        let mut skip_px: u32 = 0;
        let mut left_x = point.x;
        let mut right_x = left_x + self.width as i32;
        let mut pending_skips = 0;

        if let Some(sub) = self.sub {
            // Cut image lines above sub-region.
            {
                let skip_px = sub.top_left.y * self.width as i32;
                let start_i = skip_px as usize / PPB;
                let Some(sub_image) = image.get(start_i..) else {
                    return;
                };
                image = sub_image;
            }

            // Cut image lines below sub-region.
            {
                let height = sub.size.height as i32;
                let skip_px = height * self.width as i32;
                let end_i = skip_px as usize / PPB;
                let Some(sub_image) = image.get(..end_i) else {
                    return;
                };
                image = sub_image;
            }

            // Cut image on the left and right from the sub-region.
            image = &image[sub.top_left.x as usize / PPB..];
            // pending_skips = (sub.top_left.x as usize % PPB) * 10;
            // left_x = 2 * (sub.top_left.x % PPB as i32);
            let sub_width = u32::min(sub.size.width, self.width);
            skip_px = self.width - sub_width;
            right_x = left_x + sub_width as i32;
        }

        // Cut image lines above the screen.
        if p.y < 0 {
            let start_i = (-p.y * self.width as i32) as usize / PPB;
            let Some(sub_image) = image.get(start_i..) else {
                return;
            };
            image = sub_image;
            p.y = 0;
        }

        // Cut image lines below the screen.
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

        // Skip the right out-of-bounds part of the image.
        if right_x > WIDTH as i32 {
            let skip_diff = right_x as u32 - WIDTH as u32;
            skip_px += skip_diff;
            right_x = WIDTH as i32 + (skip_diff % PPB as u32) as i32;
        }

        // Skip the left out-of-bounds part of the image.
        if left_x < 0 {
            let skip_diff = -left_x as u32;
            skip_px += skip_diff;
            left_x = -((skip_diff % PPB as u32) as i32);
        }

        let skip_px = skip_px as usize;
        let skip_extra = skip_px % PPB;
        let skip = skip_px / PPB;
        left_x -= skip_extra as i32;
        p.x = left_x;

        let mut i = 0;
        while i < image.len() {
            let byte = image[i];

            if pending_skips != 0 {
                pending_skips -= 1;
            } else {
                let c1 = usize::from((byte >> 4) & 0x0F);
                if let Some(c1) = swaps[c1] {
                    frame.set_pixel(p, c1);
                };
            }
            p.x += 1;
            if p.x >= right_x {
                p.x = left_x;
                p.y += 1;
                i += skip;
                pending_skips = skip_extra;
                continue;
            }

            if pending_skips != 0 {
                pending_skips -= 1;
            } else {
                let c2 = usize::from(byte & 0x0F);
                if let Some(c2) = swaps[c2] {
                    frame.set_pixel(p, c2);
                };
            }
            p.x += 1;
            if p.x >= right_x {
                p.x = left_x;
                p.y += 1;
                i += skip;
                pending_skips = skip_extra;
            }
            i += 1;
        }
    }
}

/// Convert 1/2/4 BPP image into 4 BPP ([`Gray4`]) color.
pub(crate) struct BPPAdapter<'a, D, C>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
    C: PixelColor + IntoStorage<Storage = u8>,
{
    target: &'a mut D,
    swaps: [Option<Gray4>; 16],
    color: PhantomData<C>,
}

impl<'a, D, C> BPPAdapter<'a, D, C>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
    C: PixelColor + IntoStorage<Storage = u8>,
{
    pub fn new(target: &'a mut D, transp: u8, swaps: &'_ [u8]) -> Self {
        Self {
            target,
            swaps: parse_swaps(transp, swaps),
            color: PhantomData,
        }
    }
}

/// Required by the DrawTarget trait.
impl<D, C> OriginDimensions for BPPAdapter<'_, D, C>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
    C: PixelColor + IntoStorage<Storage = u8>,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<D, C> DrawTarget for BPPAdapter<'_, D, C>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
    C: PixelColor + IntoStorage<Storage = u8>,
{
    type Color = C;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let iter = pixels.into_iter().filter_map(|Pixel(p, c)| {
            let c = c.into_storage();
            match self.swaps.get(c as usize) {
                Some(Some(c)) => Some(Pixel(p, *c)),
                _ => None,
            }
        });
        self.target.draw_iter(iter)
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

/// Statically ensure that the given Result cannot have an error.
fn never_fails<T>(_: Result<T, Infallible>) {}
