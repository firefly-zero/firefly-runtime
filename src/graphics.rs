use crate::color::{BPPAdapter, TransparencyAdapter};
use crate::state::{State, HEIGHT, WIDTH};
use core::convert::Infallible;
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::pixelcolor::{BinaryColor, Gray2, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn get_screen_size(mut _caller: C) -> i32 {
    ((WIDTH as i32) << 16) | (HEIGHT as i32)
}

/// Set every pixel of the frame buffer to the given color.
pub(crate) fn clear(mut caller: C, color: u32) {
    let state = caller.data_mut();
    let color = Gray2::new(color as u8 - 1);
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
    let color = Gray2::new(color as u8 - 1);
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
    let color = Gray2::new(color as u8 - 1);
    let style = PrimitiveStyle::with_stroke(color, stroke_width);
    never_fails(line.draw_styled(&style, &mut state.frame));
}

/// Draw a rectangle.
pub(crate) fn draw_rect(
    mut caller: C,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let point = Point::new(x, y);
    let size = Size::new(width, height);
    let rect = Rectangle::new(point, size);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    never_fails(rect.draw_styled(&style, &mut state.frame));
}

// Draw a rectangle with rounded corners.
pub(crate) fn draw_rounded_rect(
    mut caller: C,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    corner_width: u32,
    corner_height: u32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let point = Point::new(x, y);
    let size = Size::new(width, height);
    let rect = Rectangle::new(point, size);
    let corner = Size::new(corner_width, corner_height);
    let rounded = RoundedRectangle::with_equal_corners(rect, corner);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    never_fails(rounded.draw_styled(&style, &mut state.frame));
}

/// Draw a circle.
pub(crate) fn draw_circle(
    mut caller: C,
    x: i32,
    y: i32,
    diameter: u32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let top_left = Point::new(x, y);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let circle = Circle::new(top_left, diameter);
    never_fails(circle.draw_styled(&style, &mut state.frame));
}

/// Draw an ellipse.
pub(crate) fn draw_ellipse(
    mut caller: C,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let top_left = Point::new(x, y);
    let size = Size::new(width, height);
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

/// Draw an arc.
pub(crate) fn draw_arc(
    mut caller: C,
    x: i32,
    y: i32,
    diameter: u32,
    angle_start: i32,
    angle_sweep: i32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let point = Point::new(x, y);
    let angle_start = Angle::from_degrees(angle_start as f32);
    let angle_sweep = Angle::from_degrees(angle_sweep as f32);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let arc = Arc::new(point, diameter, angle_start, angle_sweep);
    never_fails(arc.draw_styled(&style, &mut state.frame));
}

/// Draw a sector.
pub(crate) fn draw_sector(
    mut caller: C,
    x: i32,
    y: i32,
    diameter: u32,
    angle_start: i32,
    angle_sweep: i32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    let point = Point::new(x, y);
    let angle_start = Angle::from_degrees(angle_start as f32);
    let angle_sweep = Angle::from_degrees(angle_sweep as f32);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let arc = Sector::new(point, diameter, angle_start, angle_sweep);
    never_fails(arc.draw_styled(&style, &mut state.frame));
}

pub(crate) fn draw_sub_image(
    caller: C,
    ptr: i32,
    len: i32,
    x: i32,
    y: i32,
    width: u32,
    transp: i32,
    bpp: i32,
    sub_x: i32,
    sub_y: i32,
    sub_width: u32,
    sub_height: u32,
) {
    let sub_point = Point::new(sub_x, sub_y);
    let sub_size = Size::new(sub_width, sub_height);
    let sub = Rectangle::new(sub_point, sub_size);
    draw_image_inner(caller, ptr, len, x, y, width, transp, bpp, Some(sub))
}

pub(crate) fn draw_image(
    caller: C,
    x: i32,
    y: i32,
    ptr: i32,
    len: i32,
    width: u32,
    transp: i32,
    bpp: i32,
) {
    draw_image_inner(caller, ptr, len, x, y, width, transp, bpp, None)
}

pub(crate) fn draw_image_inner(
    mut caller: C,
    ptr: i32,
    len: i32,
    x: i32,
    y: i32,
    width: u32,
    transp: i32,
    bpp: i32,
    sub: Option<Rectangle>,
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
    let is_transp = transp != 0;
    match (bpp, is_transp) {
        // 1BPP, transparent
        (1, true) => {
            let mut adapter = TransparencyAdapter {
                target:      &mut state.frame,
                transparent: Gray2::new(transp as u8 - 1),
            };
            draw_1bpp(image_bytes, width, point, sub, &mut adapter);
        }
        // 1BPP, no transparency
        (1, false) => {
            draw_1bpp(image_bytes, width, point, sub, &mut state.frame);
        }
        // 2BPP, transparent
        (2, true) => {
            let mut adapter = TransparencyAdapter {
                target:      &mut state.frame,
                transparent: Gray2::new(transp as u8 - 1),
            };
            draw_2bpp(image_bytes, width, point, sub, &mut adapter);
        }
        // 2BPP, no transparency
        (2, false) => {
            draw_2bpp(image_bytes, width, point, sub, &mut state.frame);
        }
        // unexpected BPP
        (_, _) => {
            // TODO: log "bad BPP" error message
        }
    }
}

fn draw_1bpp<T>(
    image_bytes: &[u8],
    width: u32,
    point: Point,
    sub: Option<Rectangle>,
    target: &mut T,
) where
    T: DrawTarget<Color = Gray2, Error = Infallible> + OriginDimensions,
{
    let image_raw = ImageRawLE::<BinaryColor>::new(image_bytes, width);
    let mut adapter = BPPAdapter { target };
    match sub {
        Some(sub) => {
            let image_raw = image_raw.sub_image(&sub);
            let image = Image::new(&image_raw, point);
            never_fails(image.draw(&mut adapter));
        }
        None => {
            let image = Image::new(&image_raw, point);
            never_fails(image.draw(&mut adapter));
        }
    }
}

fn draw_2bpp<T>(
    image_bytes: &[u8],
    width: u32,
    point: Point,
    sub: Option<Rectangle>,
    target: &mut T,
) where
    T: DrawTarget<Color = Gray2, Error = Infallible> + OriginDimensions,
{
    let image_raw = ImageRawLE::<Gray2>::new(image_bytes, width);
    match sub {
        Some(sub) => {
            let image_raw = image_raw.sub_image(&sub);
            let image = Image::new(&image_raw, point);
            never_fails(image.draw(target));
        }
        None => {
            let image = Image::new(&image_raw, point);
            never_fails(image.draw(target));
        }
    }
}

fn get_shape_style(fill_color: u32, stroke_color: u32, stroke_width: u32) -> PrimitiveStyle<Gray2> {
    let mut style = PrimitiveStyle::new();
    if fill_color != 0 {
        let fill_color = Gray2::new(fill_color as u8 - 1);
        style.fill_color = Some(fill_color);
    }
    if stroke_color != 0 && stroke_width != 0 {
        let stroke_color = Gray2::new(stroke_color as u8 - 1);
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

        let inputs = [I32(2)];
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

        let inputs = [I32(2), I32(1), I32(4), I32(3), I32(3), I32(1)];
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
