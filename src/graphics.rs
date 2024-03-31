use crate::color::TransparencyAdapter;
use crate::state::{State, HEIGHT, WIDTH};
use core::convert::Infallible;
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::pixelcolor::{Gray2, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn get_screen_size(mut _caller: C) -> i32 {
    ((WIDTH as i32) << 16) | (HEIGHT as i32)
}

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

/// Draw a single point.
///
/// Without scailing, sets a single pixel.
pub(crate) fn draw_point(mut caller: C, x: i32, y: i32, color: u32) {
    let state = caller.data_mut();
    let point = Point::new(x, y);
    let color = Gray2::new(color as u8);
    let pixel = Pixel(point, color);
    never_fails(pixel.draw(&mut state.frame));
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

/// Draw a rectangle.
pub(crate) fn draw_rect(
    mut caller: C,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let point = Point::new(x, y);
    let size = Size::new(width as u32, height as u32);
    let rect = Rectangle::new(point, size);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    never_fails(rect.draw_styled(&style, &mut state.frame));
}

/// Draw a circle.
pub(crate) fn draw_circle(
    mut caller: C,
    x: i32,
    y: i32,
    diameter: i32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let top_left = Point::new(x, y);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let circle = Circle::new(top_left, diameter as u32);
    never_fails(circle.draw_styled(&style, &mut state.frame));
}

/// Draw an ellipse.
pub(crate) fn draw_ellipse(
    mut caller: C,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let top_left = Point::new(x, y);
    let size = Size::new(width as u32, height as u32);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let ellipse = Ellipse::new(top_left, size);
    never_fails(ellipse.draw_styled(&style, &mut state.frame));
}

/// Draw a line between two points.
pub(crate) fn draw_triangle(
    mut caller: C,
    p1_x: i32,
    p1_y: i32,
    p2_x: i32,
    p2_y: i32,
    p3_x: i32,
    p3_y: i32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let vertex1 = Point::new(p1_x, p1_y);
    let vertex2 = Point::new(p2_x, p2_y);
    let vertex3 = Point::new(p3_x, p3_y);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let triangle = Triangle::new(vertex1, vertex2, vertex3);
    never_fails(triangle.draw_styled(&style, &mut state.frame));
}

pub(crate) fn draw_image(
    mut caller: C,
    x: i32,
    y: i32,
    ptr: i32,
    len: i32,
    width: i32,
    transp: i32,
) {
    // retrieve the raw data from memory
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        // TODO: log "no memory found" error
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let ptr = ptr as usize;
    let len = len as usize;
    let image_bytes = &data[ptr..len];

    let point = Point::new(x, y);
    let image_raw = ImageRawLE::<Gray2>::new(image_bytes, width as u32);
    let image = Image::new(&image_raw, point);
    if transp >= 4 {
        // Draw without transparency.
        never_fails(image.draw(&mut state.frame));
    } else {
        // Draw with transparency using adapter.
        let mut adapter = TransparencyAdapter {
            target:      &mut state.frame,
            transparent: Gray2::new(transp as u8),
        };
        never_fails(image.draw(&mut adapter));
    };
}

fn get_shape_style(fill_color: u32, stroke_color: u32, stroke_width: u32) -> PrimitiveStyle<Gray2> {
    let fill_color = Gray2::new(fill_color as u8);
    let mut style = PrimitiveStyle::with_fill(fill_color);
    if stroke_width != 0 {
        let stroke_color = Gray2::new(stroke_color as u8);
        style.stroke_color = Some(stroke_color);
        style.stroke_width = stroke_width;
    }
    style
}

/// Statically ensure that the given Result cannot have an error.
fn never_fails(_: Result<(), Infallible>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
    use embedded_graphics::mock_display::MockDisplay;
    use wasmi::Value::*;

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

        let inputs = [I32(1)];
        let mut outputs = Vec::new();
        func.call(&mut store, &inputs, &mut outputs).unwrap();
        assert_eq!(outputs.len(), 0);

        // check that all pixel in the frame buffer are set to 1.
        let state = store.data();
        for byte in state.frame.data() {
            assert_eq!(byte, &0b_0101_0101);
        }
    }

    #[test]
    fn test_draw_line() {
        let engine = wasmi::Engine::default();
        let state = State::new();
        let mut store = <wasmi::Store<State>>::new(&engine, state);
        let func = wasmi::Func::wrap(&mut store, draw_line);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in state.frame.data() {
            assert_eq!(byte, &0b_0000_0000);
        }

        let inputs = [I32(2), I32(1), I32(4), I32(3), I32(2), I32(1)];
        let mut outputs = Vec::new();
        func.call(&mut store, &inputs, &mut outputs).unwrap();
        assert_eq!(outputs.len(), 0);

        let mut display = MockDisplay::<Gray2>::new();
        display.set_allow_out_of_bounds_drawing(true);
        let state = store.data();
        let area = Rectangle::new(Point::zero(), Size::new(6, 5));
        let image = state.frame.as_image();
        let image = image.sub_image(&area);
        image.draw(&mut display).unwrap();
        display.assert_pattern(&[
            "000000", // y=0
            "002000", // y=1
            "000200", // y=2
            "000020", // y=3
            "000000", // y=4
        ]);
    }
}
