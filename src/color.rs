use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;

/// Replace colors.
pub(crate) struct ColorReplaceAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    pub target: &'a mut D,

    /// Replacement colors.
    pub colors: [Option<Gray2>; 4],
}

/// Required by the DrawTarget trait.
impl<'a, D> OriginDimensions for ColorReplaceAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<'a, D> DrawTarget for ColorReplaceAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    type Color = Gray2;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let iter = pixels
            .into_iter()
            .filter_map(|Pixel(point, color)| -> Option<Pixel<Gray2>> {
                let raw = color.into_storage();
                debug_assert!(raw < 4);
                let color = self.colors[raw as usize]?;
                Some(Pixel(point, color))
            });
        self.target.draw_iter(iter)
    }
}

// Convert 1 bit per pixel image into 2 bits par pixel.
pub(crate) struct BPPAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    pub target: &'a mut D,
}

/// Required by the DrawTarget trait.
impl<'a, D> OriginDimensions for BPPAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<'a, D> DrawTarget for BPPAdapter<'a, D>
where
    D: DrawTarget<Color = Gray2> + OriginDimensions,
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
        let iter = colors.into_iter().map(|c| Gray2::new(c.into_storage()));
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
