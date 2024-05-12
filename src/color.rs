use core::marker::PhantomData;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::Pixel;

// Convert 1/2/4 BPP image into 4 BPP ([`Gray4`]) color.
pub(crate) struct BPPAdapter<'a, D, C>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
    C: PixelColor + IntoStorage<Storage = u8>,
{
    target: &'a mut D,
    swaps:  [Option<Gray4>; 16],
    color:  PhantomData<C>,
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
impl<'a, D, C> OriginDimensions for BPPAdapter<'a, D, C>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
    C: PixelColor + IntoStorage<Storage = u8>,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<'a, D, C> DrawTarget for BPPAdapter<'a, D, C>
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

/// Create RGB (or BGR) color from R, G, and B components.
///
/// All RGB colors implemented in embedded_graphics provide exactly the same
/// new `method` but this method is not part of any trait.
/// So, we have to make our own.
pub trait FromRGB {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self;
}

impl FromRGB for Rgb555 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

impl FromRGB for Bgr555 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

impl FromRGB for Rgb565 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

impl FromRGB for Bgr565 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

impl FromRGB for Rgb666 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

impl FromRGB for Bgr666 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

impl FromRGB for Rgb888 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

impl FromRGB for Bgr888 {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }
}

#[allow(clippy::get_first)]
fn parse_swaps(transp: u8, swaps: &[u8]) -> [Option<Gray4>; 16] {
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
