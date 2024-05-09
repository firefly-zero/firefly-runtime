use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;

// Convert 1 bit per pixel image into 2 bits per pixel.
pub(crate) struct BPPAdapter<'a, D>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
{
    pub target: &'a mut D,
}

/// Required by the DrawTarget trait.
impl<'a, D> OriginDimensions for BPPAdapter<'a, D>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<'a, D> DrawTarget for BPPAdapter<'a, D>
where
    D: DrawTarget<Color = Gray4> + OriginDimensions,
{
    type Color = BinaryColor;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        panic!("not implemented, use BPPAdapter.fill_contiguous instead")
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let iter = colors.into_iter().map(|c| Gray4::new(c.into_storage()));
        self.target.fill_contiguous(area, iter)
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
