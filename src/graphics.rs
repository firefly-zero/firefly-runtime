use crate::state::State;
use embedded_graphics::pixelcolor::{Gray2, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

type C<'a> = wasmi::Caller<'a, State>;

/// Set every pixel of the frame buffer to the given color.
pub(crate) fn clear(mut caller: C, color: u32) {
    let state = caller.data_mut();
    let color = Gray2::new(color as u8);
    log_error(state.frame.clear(color));
}

/// Set the given palette color.
pub(crate) fn set_color(mut caller: C, index: u32, r: u32, g: u32, b: u32) {
    let state = caller.data_mut();
    state.palette[index as usize] = Rgb888::new(r as u8, g as u8, b as u8);
}

/// Set all colors of the palette.
pub(crate) fn set_colors(
    mut caller: C,
    c1_r: u32,
    c1_g: u32,
    c1_b: u32,
    c2_r: u32,
    c2_g: u32,
    c2_b: u32,
    c3_r: u32,
    c3_g: u32,
    c3_b: u32,
    c4_r: u32,
    c4_g: u32,
    c4_b: u32,
) {
    let state = caller.data_mut();
    state.palette[0] = Rgb888::new(c1_r as u8, c1_g as u8, c1_b as u8);
    state.palette[1] = Rgb888::new(c2_r as u8, c2_g as u8, c2_b as u8);
    state.palette[2] = Rgb888::new(c3_r as u8, c3_g as u8, c3_b as u8);
    state.palette[3] = Rgb888::new(c4_r as u8, c4_g as u8, c4_b as u8);
}

/// Draw a line between two points.
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
