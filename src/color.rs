use crate::state::State;
use core::marker::PhantomData;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::DrawTarget;

struct ColorAdapter<'a, C: RgbColor> {
    state: &'a State,
    _color: PhantomData<C>,
}

impl<'a, C: RgbColor> ColorAdapter<'a, C> {
    pub fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = C>,
    {
        todo!()
    }
}
