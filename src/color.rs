use core::marker::PhantomData;

use crate::state::State;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::{Gray2, IntoStorage, Rgb888, RgbColor};
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;

struct ColorAdapter<'a, C, D>
where
    C: RgbColor,
    D: DrawTarget<Color = C> + OriginDimensions,
{
    state: &'a State,
    target: D,
}

impl<'a, C, D> OriginDimensions for ColorAdapter<'a, C, D>
where
    C: RgbColor,
    D: DrawTarget<Color = C> + OriginDimensions,
{
    fn size(&self) -> embedded_graphics::prelude::Size {
        self.target.size()
    }
}

impl<'a, C, D> DrawTarget for ColorAdapter<'a, C, D>
where
    C: RgbColor,
    D: DrawTarget<Color = C> + OriginDimensions,
{
    type Color = Gray2;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        panic!("not implemented")
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let iter = colors.into_iter().map(|c: Gray2| -> C {
            let index = c.into_storage();
            let rgb888 = self.state.palette[index as usize];
            let r = rgb888.r() as u32 * C::MAX_R as u32 / Rgb888::MAX_R as u32;
            let g = rgb888.g() as u32 * C::MAX_G as u32 / Rgb888::MAX_G as u32;
            let b = rgb888.b() as u32 * C::MAX_B as u32 / Rgb888::MAX_B as u32;
            C::new(r, g, b) // oh no
        });
        self.target.fill_contiguous(area, iter)
    }
}
