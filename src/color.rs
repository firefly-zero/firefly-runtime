use core::marker::PhantomData;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::Pixel;

/// Color optimized for fast rendering on the device.
#[derive(PartialEq, Clone, Copy)]
pub struct Rgb16(pub u8, pub u8);

impl Rgb16 {
    pub const fn from_rgb(r: u16, g: u16, b: u16) -> Self {
        let r = r >> 3;
        let g = g >> 2;
        let b = b >> 3;
        let raw = (b << 11) | ((g & 0b_0011_1111) << 5) | (r & 0b_0001_1111);
        let raw = raw.to_le_bytes();
        Self(!raw[0], !raw[1])
    }

    pub const fn into_rgb(self) -> (u8, u8, u8) {
        let raw = u16::from_le_bytes([!self.0, !self.1]);
        let r = (raw << 3) as u8;
        let g = ((raw >> 5) << 2) as u8;
        let b = ((raw >> 11) << 3) as u8;
        (r, g, b)
    }
}

impl PixelColor for Rgb16 {
    type Raw = RawU16;
}

impl RgbColor for Rgb16 {
    const MAX_R: u8 = 32;
    const MAX_G: u8 = 64;
    const MAX_B: u8 = 32;
    const BLACK: Self = Self::from_rgb(0, 0, 0);
    const RED: Self = Self::from_rgb(255, 0, 0);
    const GREEN: Self = Self::from_rgb(0, 255, 0);
    const BLUE: Self = Self::from_rgb(0, 0, 255);
    const YELLOW: Self = Self::from_rgb(255, 255, 0);
    const MAGENTA: Self = Self::from_rgb(255, 0, 255);
    const CYAN: Self = Self::from_rgb(0, 255, 255);
    const WHITE: Self = Self::from_rgb(255, 255, 255);

    fn r(&self) -> u8 {
        let (r, _, _) = self.into_rgb();
        r
    }

    fn g(&self) -> u8 {
        let (_, g, _) = self.into_rgb();
        g
    }

    fn b(&self) -> u8 {
        let (_, _, b) = self.into_rgb();
        b
    }
}

impl From<Rgb888> for Rgb16 {
    fn from(c: Rgb888) -> Self {
        let r = u16::from(c.r());
        let g = u16::from(c.g());
        let b = u16::from(c.b());
        Self::from_rgb(r, g, b)
    }
}

impl From<Rgb16> for Rgb888 {
    fn from(c: Rgb16) -> Self {
        let (mut r, mut g, mut b) = c.into_rgb();
        if r == 0b1111_1000 {
            r = 0xff;
        }
        if g == 0b1111_1100 {
            g = 0xff;
        }
        if b == 0b1111_1000 {
            b = 0xff;
        }
        Self::new(r, g, b)
    }
}

// Convert 1/2/4 BPP image into 4 BPP ([`Gray4`]) color.
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

/// Create RGB (or BGR) color from R, G, and B components in 0-255 range.
pub trait FromRGB {
    /// The white background color.
    const BG: Self;
    /// The primary black text color.
    const PRIMARY: Self;
    /// The dark blue accent color.
    const ACCENT: Self;
    /// The gray muted text color.
    const MUTED: Self;

    fn from_rgb(rgb: Rgb16) -> Self;
}

impl FromRGB for Rgb16 {
    const BG: Self = Self::from_rgb(0xf4, 0xf4, 0xf4);
    const PRIMARY: Self = Self::from_rgb(0x1a, 0x1c, 0x2c);
    const ACCENT: Self = Self::from_rgb(0x3b, 0x5d, 0xc9);
    const MUTED: Self = Self::from_rgb(0x94, 0xb0, 0xc2);

    fn from_rgb(rgb: Self) -> Self {
        rgb
    }
}

impl FromRGB for Rgb565 {
    const BG: Self = new_rgb565(0xf4, 0xf4, 0xf4);
    const PRIMARY: Self = new_rgb565(0x1a, 0x1c, 0x2c);
    const ACCENT: Self = new_rgb565(0x3b, 0x5d, 0xc9);
    const MUTED: Self = new_rgb565(0x94, 0xb0, 0xc2);

    fn from_rgb(rgb: Rgb16) -> Self {
        let (r, g, b) = rgb.into_rgb();
        Self::new(r, g, b)
    }
}

impl FromRGB for Rgb888 {
    const BG: Self = Self::new(0xf4, 0xf4, 0xf4);
    const PRIMARY: Self = Self::new(0x1a, 0x1c, 0x2c);
    const ACCENT: Self = Self::new(0x3b, 0x5d, 0xc9);
    const MUTED: Self = Self::new(0x94, 0xb0, 0xc2);

    fn from_rgb(rgb: Rgb16) -> Self {
        rgb.into()
    }
}

const fn new_rgb565(r: u8, g: u8, b: u8) -> Rgb565 {
    let r = r as u32 * Rgb565::MAX_R as u32 / Rgb888::MAX_R as u32;
    let g = g as u32 * Rgb565::MAX_G as u32 / Rgb888::MAX_G as u32;
    let b = b as u32 * Rgb565::MAX_B as u32 / Rgb888::MAX_B as u32;
    debug_assert!(r < 256);
    debug_assert!(g < 256);
    debug_assert!(b < 256);
    Rgb565::new(r as u8, g as u8, b as u8)
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
