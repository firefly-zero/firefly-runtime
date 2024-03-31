use crate::state::State;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;

/// Convert on the fly the Gray2 color into RGB using the palette.
pub(crate) struct ColorAdapter<'a, C, D>
where
    C: RgbColor + FromRGB,
    D: DrawTarget<Color = C> + OriginDimensions,
{
    pub state:  &'a State,
    pub target: &'a mut D,
}

/// Required by the DrawTarget trait.
impl<'a, C, D> OriginDimensions for ColorAdapter<'a, C, D>
where
    C: RgbColor + FromRGB,
    D: DrawTarget<Color = C> + OriginDimensions,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<'a, C, D> DrawTarget for ColorAdapter<'a, C, D>
where
    C: RgbColor + FromRGB,
    D: DrawTarget<Color = C> + OriginDimensions,
{
    type Color = Gray2;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        panic!("not implemented, use fill_contiguous instead")
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let iter = colors.into_iter().map(|c: Gray2| -> C {
            let index = c.into_storage();
            debug_assert!(index < 4);
            let rgb888 = self.state.palette[index as usize];
            let r = rgb888.r() as u32 * C::MAX_R as u32 / Rgb888::MAX_R as u32;
            let g = rgb888.g() as u32 * C::MAX_G as u32 / Rgb888::MAX_G as u32;
            let b = rgb888.b() as u32 * C::MAX_B as u32 / Rgb888::MAX_B as u32;
            debug_assert!(r < 256);
            debug_assert!(g < 256);
            debug_assert!(b < 256);
            C::from_rgb(r as u8, g as u8, b as u8)
        });
        self.target.fill_contiguous(area, iter)
    }
}

// Do not write pixels of a certain color.
pub(crate) struct TransparencyAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    pub target: &'a mut D,

    /// The color to skip.
    pub transparent: Gray2,
}

/// Required by the DrawTarget trait.
impl<'a, D> OriginDimensions for TransparencyAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<'a, D> DrawTarget for TransparencyAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    type Color = Gray2;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let iter = pixels.into_iter().filter(|p| p.1 != self.transparent);
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
