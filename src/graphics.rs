use crate::state::State;
use core::convert::Infallible;
use embedded_graphics::pixelcolor::{Gray2, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

type C<'a> = wasmi::Caller<'a, State>;

/// Set every pixel of the frame buffer to the given color.
pub(crate) fn clear(mut caller: C, color: u32) {
    let state = caller.data_mut();
    let color = Gray2::new(color as u8);
    never_fails(state.frame.clear(color));
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
    never_fails(line.draw_styled(&style, &mut state.frame));
}

/// Statically ensure that the given Result cannot have an error.
fn never_fails(_: Result<(), Infallible>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;

    #[test]
    fn test_clear() {
        let engine = wasmi::Engine::default();
        let state = State::new();
        let mut store = <wasmi::Store<State>>::new(&engine, state);
        let func = wasmi::Func::wrap(&mut store, clear);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in state.frame.data() {
            assert_eq!(byte, &0b_0000_0000);
        }

        let inputs = [wasmi::Value::I32(1)];
        let mut outputs = Vec::new();
        func.call(&mut store, &inputs, &mut outputs).unwrap();
        assert_eq!(outputs.len(), 0);

        // check that all pixel in the frame buffer are set to 1.
        let state = store.data();
        for byte in state.frame.data() {
            assert_eq!(byte, &0b_0101_0101);
        }
    }
}
