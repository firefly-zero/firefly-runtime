use crate::state::State;
use embedded_graphics::pixelcolor::{Gray2, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn set_color(mut caller: C, index: u32, r: u32, g: u32, b: u32) {
    let state = caller.data_mut();
    state.palette[index as usize] = Rgb888::new(r as u8, g as u8, b as u8)
}

pub(crate) fn draw_line(
    mut caller: C,
    p1_x: i32,
    p1_y: i32,
    p2_x: i32,
    p2_y: i32,
    color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let start = Point::new(p1_x, p1_y);
    let end = Point::new(p2_x, p2_y);
    let line = Line::new(start, end);
    let color = Gray2::new(color as u8);
    let style = PrimitiveStyle::with_stroke(color, stroke_width);
    log_error(line.draw_styled(&style, &mut state.frame));
}

fn log_error<T, E: core::fmt::Debug>(res: Result<T, E>) {
    if let Err(err) = res {
        // TODO: don't panic, write into serial
        panic!("{err:?}")
    }
}
