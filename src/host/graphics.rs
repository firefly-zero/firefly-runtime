use crate::canvas::Canvas;
use crate::color::Rgb16;
use crate::error::HostError;
use crate::frame_buffer::{HEIGHT, WIDTH};
use crate::image::ParsedImage;
use crate::state::State;
use alloc::boxed::Box;
use core::convert::Infallible;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::{mapping, DecorationDimensions, MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

type C<'a, 'b> = wasmi::Caller<'a, Box<State<'b>>>;

/// Set every pixel of the frame buffer to the given color.
pub(crate) fn clear_screen(mut caller: C, color: i32) {
    let state = caller.data_mut();
    state.called = "graphics.clear_screen";
    if color == 0 {
        return;
    }
    let color = Gray4::new(color as u8 - 1);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        target.clear(color)
    } else {
        state.frame.clear(color)
    };
    never_fails(err);
}

/// Set the given palette color.
pub(crate) fn set_color(mut caller: C, index: u32, r: u32, g: u32, b: u32) {
    let state = caller.data_mut();
    state.called = "graphics.set_color";
    if index > 16 {
        state.log_error("color index out of range");
        return;
    }
    if index == 0 {
        state.log_error("cannot set color for transparency");
        return;
    }
    state.frame.palette[index as usize - 1] = Rgb16::from_rgb(r as u16, g as u16, b as u16);
}

/// Draw a single point.
///
/// Without scailing, sets a single pixel.
pub(crate) fn draw_point(mut caller: C, x: i32, y: i32, color: i32) {
    let state = caller.data_mut();
    state.called = "graphics.draw_point";
    if color == 0 {
        state.log_error(HostError::NoneColor);
        return;
    }
    let point = Point::new(x, y);
    if let Some(canvas) = &state.canvas {
        let color = Gray4::new(color as u8 - 1);
        let pixel = Pixel(point, color);
        let mut target = canvas.clone().as_target(&mut caller);
        never_fails(pixel.draw(&mut target));
    } else {
        state.frame.dirty = true;
        state.frame.set_pixel(point, color as u8 - 1);
    };
}

/// Draw a line between two points.
pub(crate) fn draw_line(
    mut caller: C,
    p1_x: i32,
    p1_y: i32,
    p2_x: i32,
    p2_y: i32,
    color: i32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_line";

    let Some(color) = parse_color(color) else {
        state.log_error(HostError::NoneColor);
        return;
    };
    if state.canvas.is_none() {
        let frame = &mut state.frame;
        if p1_y == p2_y {
            frame.draw_hline(p1_x, p2_x, p1_y, stroke_width, color);
            return;
        }
        if p1_x == p2_x {
            frame.draw_vline(p1_x, p1_y, p2_y, stroke_width, color);
            return;
        }
    }

    let start = Point::new(p1_x, p1_y);
    let end = Point::new(p2_x, p2_y);
    let line = Line::new(start, end);
    let style = PrimitiveStyle::with_stroke(color, stroke_width);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        line.draw_styled(&style, &mut target)
    } else {
        line.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
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
    state.called = "graphics.draw_rect";
    if x > WIDTH as i32 || y > HEIGHT as i32 {
        return;
    }
    let point = Point::new(x, y);
    let size = Size::new(width, height);
    let rect = Rectangle::new(point, size);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        rect.draw_styled(&style, &mut target)
    } else {
        rect.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
}

/// Draw a rectangle with rounded corners.
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
    state.called = "graphics.draw_rounded_rect";
    let point = Point::new(x, y);
    let size = Size::new(width, height);
    let rect = Rectangle::new(point, size);
    let corner = Size::new(corner_width, corner_height);
    let rounded = RoundedRectangle::with_equal_corners(rect, corner);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        rounded.draw_styled(&style, &mut target)
    } else {
        rounded.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
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
    state.called = "graphics.draw_circle";
    let top_left = Point::new(x, y);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let circle = Circle::new(top_left, diameter);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        circle.draw_styled(&style, &mut target)
    } else {
        circle.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
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
    state.called = "graphics.draw_ellipse";
    let top_left = Point::new(x, y);
    let size = Size::new(width, height);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let ellipse = Ellipse::new(top_left, size);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        ellipse.draw_styled(&style, &mut target)
    } else {
        ellipse.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
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
    state.called = "graphics.draw_triangle";
    let vertex1 = Point::new(p1_x, p1_y);
    let vertex2 = Point::new(p2_x, p2_y);
    let vertex3 = Point::new(p3_x, p3_y);
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let triangle = Triangle::new(vertex1, vertex2, vertex3);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        triangle.draw_styled(&style, &mut target)
    } else {
        triangle.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
}

/// Draw an arc.
pub(crate) fn draw_arc(
    mut caller: C,
    x: i32,
    y: i32,
    diameter: u32,
    angle_start: wasmi::F32,
    angle_sweep: wasmi::F32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_arc";
    let point = Point::new(x, y);
    let angle_start = Angle::from_radians(angle_start.into());
    let angle_sweep = Angle::from_radians(angle_sweep.into());
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let arc = Arc::new(point, diameter, angle_start, angle_sweep);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        arc.draw_styled(&style, &mut target)
    } else {
        arc.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
}

/// Draw a sector.
pub(crate) fn draw_sector(
    mut caller: C,
    x: i32,
    y: i32,
    diameter: u32,
    angle_start: wasmi::F32,
    angle_sweep: wasmi::F32,
    fill_color: u32,
    stroke_color: u32,
    stroke_width: u32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_sector";
    let point = Point::new(x, y);
    let angle_start = Angle::from_radians(angle_start.into());
    let angle_sweep = Angle::from_radians(angle_sweep.into());
    let style = get_shape_style(fill_color, stroke_color, stroke_width);
    let sector = Sector::new(point, diameter, angle_start, angle_sweep);
    let err = if let Some(canvas) = &state.canvas {
        let mut target = canvas.clone().as_target(&mut caller);
        sector.draw_styled(&style, &mut target)
    } else {
        sector.draw_styled(&style, &mut state.frame)
    };
    never_fails(err);
}

pub(crate) fn draw_qr(
    mut caller: C,
    text_ptr: u32,
    text_len: u32,
    x: i32,
    y: i32,
    black: i32,
    white: i32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_qr";

    // read text from the guest memory
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let text_ptr = text_ptr as usize;
    let text_len = text_len as usize;
    let Some(text_bytes) = &data.get(text_ptr..(text_ptr + text_len)) else {
        state.log_error(HostError::OomPointer);
        return;
    };

    // generate ASCII QR code
    let Ok(code) = tinyqr::QrCode::new(text_bytes) else {
        state.log_error("QR code cannot be constructed");
        return;
    };
    let ascii_img = code
        .render::<char>()
        .dark_color('#')
        .light_color(' ')
        .module_dimensions(1, 1)
        .build();

    // render QR code
    let width = ascii_img.find('\n').unwrap_or_default() as u32;
    let area = Rectangle {
        top_left: Point::new(x, y),
        size: Size {
            width,
            height: width,
        },
    };
    let black = if black != 0 {
        Some(Gray4::new(black as u8 - 1))
    } else {
        None
    };
    let white = if white != 0 {
        Some(Gray4::new(white as u8 - 1))
    } else {
        None
    };
    if let (Some(black), Some(white)) = (black, white) {
        let colors = ascii_img.chars().filter_map(|ch| match ch {
            '#' => Some(black),
            ' ' => Some(white),
            _ => None,
        });
        never_fails(state.frame.fill_contiguous(&area, colors));
    } else {
        let pixels = ascii_img
            .chars()
            .filter(|ch| *ch != '\n')
            .zip(area.points())
            .filter_map(|(ch, point)| {
                let color = if ch == '#' { black } else { white };
                color.map(|color| Pixel(point, color))
            });
        never_fails(state.frame.draw_iter(pixels));
    }
}

/// Draw a text message using the given font.
pub(crate) fn draw_text(
    mut caller: C,
    text_ptr: u32,
    text_len: u32,
    font_ptr: u32,
    font_len: u32,
    x: i32,
    y: i32,
    color: i32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_text";
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);

    let text_ptr = text_ptr as usize;
    let text_len = text_len as usize;
    let font_ptr = font_ptr as usize;
    let font_len = font_len as usize;

    // The slices must be within memory.
    // There also used to be a check that the slices don't intersect
    // but on practice if font is statically allocated, LLVM can optimize
    // the data section and find the text bytes within the font.
    let Some(text_bytes) = &data.get(text_ptr..(text_ptr + text_len)) else {
        state.log_error(HostError::OomPointer);
        return;
    };
    if font_ptr == 0 {
        state.log_error("font is a nil pointer: make sure you've loaded it");
        return;
    }
    let Some(font_bytes) = &data.get(font_ptr..(font_ptr + font_len)) else {
        state.log_error(HostError::OomPointer);
        return;
    };
    let font = match parse_font(font_bytes) {
        Ok(font) => font,
        Err(err) => {
            state.log_error(err);
            return;
        }
    };
    let Some(color) = parse_color(color) else {
        state.log_error(HostError::NoneColor);
        return;
    };
    let style = MonoTextStyle::new(&font, color);
    let point = Point::new(x, y);
    let Ok(text) = core::str::from_utf8(text_bytes) else {
        let msg = "the given text is not valid UTF-8";
        state.log_error(msg);
        return;
    };
    let text = Text::new(text, point, style);
    // TODO: support canvas
    never_fails(text.draw(&mut state.frame));
}

/// Set an image localted in the guest memory as the draw target for all graphic operations.
pub(crate) fn set_canvas(mut caller: C, ptr: u32, len: u32) {
    const HEADER: u32 = 4;

    let state = caller.data_mut();
    state.called = "graphics.set_canvas";

    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let uptr = ptr as usize;
    let ulen = len as usize;
    let maybe_buf = data.get(uptr..(uptr + ulen));
    let Some(image_bytes) = &maybe_buf else {
        state.log_error(HostError::OomPointer);
        return;
    };
    if image_bytes.len() < HEADER as usize {
        state.log_error("canvas is too small");
        return;
    }
    if image_bytes[0] != 0x22 {
        state.log_error("invalid magic number");
        return;
    }

    let width = u16::from_le_bytes([image_bytes[1], image_bytes[2]]) as u32;
    let image_bytes = &image_bytes[HEADER as usize..];
    if !(image_bytes.len() * 2).is_multiple_of(width as usize) {
        state.log_error(HostError::InvalidWidth);
        return;
    }
    let canvas = Canvas::new(ptr + HEADER, len - HEADER, width);
    state.canvas = Some(canvas)
}

pub(crate) fn unset_canvas(mut caller: C) {
    let state = caller.data_mut();
    state.called = "graphics.drop_canvas";
    state.canvas = None;
}

const IMG_HEADER: usize = 4;

/// Tile the given screen area with the provided sub-image.
pub(crate) fn draw_sub_tile(
    mut caller: C,
    ptr: u32,
    len: u32,
    // Screen area to fill.
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    // Image sub-region to use.
    sub_x: i32,
    sub_y: i32,
    sub_width: u32,
    sub_height: u32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_nine_slice";

    // Retrieve the raw data from memory.
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(image_bytes) = load_image(state, data, ptr, len) else {
        return;
    };

    let width = u16::from_le_bytes([image_bytes[1], image_bytes[2]]) as u32;
    if width == 0 {
        state.log_error("image has zero width");
        return;
    }
    let transp = u8::from_le_bytes([image_bytes[3]]);
    let image_bytes = &image_bytes[IMG_HEADER..];
    if !(image_bytes.len() * 2).is_multiple_of(width as usize) {
        state.log_error(HostError::InvalidWidth);
        return;
    }

    if !(w as u32).is_multiple_of(sub_width) {
        state.log_error("area width must be a multiple of sib-image width");
        return;
    }
    if !(h as u32).is_multiple_of(sub_height) {
        state.log_error("area width must be a multiple of sub-image height");
        return;
    }

    // TODO: support canvas
    if state.canvas.is_some() {
        state.log_error("images cannot be drawn on canvas yet");
        return;
    }

    let sub_point = Point::new(sub_x, sub_y);
    let sub_size = Size::new(sub_width, sub_height);
    let sub = Rectangle::new(sub_point, sub_size);

    let image = ParsedImage {
        bytes: image_bytes,
        width,
        transp,
        sub: Some(sub),
    };

    for px in (x..x + w).step_by(sub_width as _) {
        for py in (y..y + h).step_by(sub_height as _) {
            let point = Point::new(px, py);
            image.render(point, &mut state.frame);
        }
    }
}

/// Render the image using 9-slice scaling.
///
/// https://en.wikipedia.org/wiki/9-slice_scaling
pub(crate) fn draw_nine_slice(
    mut caller: C,
    ptr: u32,
    len: u32,
    // Screen area to fill.
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    // The area of the middle square on the image.
    mid_x: i32,
    mid_y: i32,
    mid_width: u32,
    mid_height: u32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_nine_slice";

    // Retrieve the raw data from memory.
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(image_bytes) = load_image(state, data, ptr, len) else {
        return;
    };

    let width = u16::from_le_bytes([image_bytes[1], image_bytes[2]]) as u32;
    if width == 0 {
        state.log_error("image has zero width");
        return;
    }
    let transp = u8::from_le_bytes([image_bytes[3]]);
    let image_bytes = &image_bytes[IMG_HEADER..];
    if !(image_bytes.len() * 2).is_multiple_of(width as usize) {
        state.log_error(HostError::InvalidWidth);
        return;
    }

    // TODO: support canvas
    if state.canvas.is_some() {
        state.log_error("images cannot be drawn on canvas yet");
        return;
    }

    let height = image_bytes.len() as u32 * 2 / width;
    let mut image = ParsedImage {
        bytes: image_bytes,
        width,
        transp,
        sub: None,
    };

    // Make sure that all segments of 9-slice fully fit into the area.
    // This limitation might be relaxed or removed in the future
    // when the implementation handles such cases correctly
    // (if this can be done without losing performance).
    if width > w as u32 {
        state.log_error("9-slice width is bigger than width of the target area");
        return;
    }
    if height > h as u32 {
        state.log_error("9-slice height is bigger than width of the target area");
        return;
    }
    if !(w as u32 - width).is_multiple_of(mid_width) {
        state.log_error("drawing area width cannot be tiled with 9-slice middle segment");
        return;
    }
    if !(h as u32 - height).is_multiple_of(mid_height) {
        state.log_error("drawing area height cannot be tiled with 9-slice middle segment");
        return;
    }

    // Segment corners.
    let x1 = 0i32;
    // TODO: handle mid_x being outside the image.
    let x2 = mid_x;
    let x3 = x2 + mid_width as i32;
    let x4 = width as i32;
    let y1 = 0i32;
    let y2 = mid_y;
    let y3 = y2 + mid_height as i32;
    let y4 = height as i32;

    // Segment sizes.
    let wl = (x2 - x1) as u32;
    let wm = (x3 - x2) as u32;
    let wr = (x4 - x3) as u32;
    let ht = (y2 - y1) as u32;
    let hm = (y3 - y2) as u32;
    let hb = (y4 - y3) as u32;

    // Top-left corner.
    {
        let size = Size::new(wl, ht);
        let point = Point::new(x1, y1);
        image.sub = Some(Rectangle::new(point, size));
        let point = Point::new(x, y);
        image.render(point, &mut state.frame);
    }
    // Top-right corner.
    {
        let size = Size::new(wr, ht);
        let point = Point::new(x3, y1);
        image.sub = Some(Rectangle::new(point, size));
        let point = Point::new(x + w - wr as i32, y);
        image.render(point, &mut state.frame);
    }
    // Bottom-left corner.
    {
        let size = Size::new(wl, hb);
        let point = Point::new(x1, y3);
        image.sub = Some(Rectangle::new(point, size));
        let point = Point::new(x, y + h - hb as i32);
        image.render(point, &mut state.frame);
    }
    // Bottom-right corner.
    {
        let size = Size::new(wr, hb);
        let point = Point::new(x3, y3);
        image.sub = Some(Rectangle::new(point, size));
        let point = Point::new(x + w - wr as i32, y + h - hb as i32);
        image.render(point, &mut state.frame);
    }

    // Top edge.
    let range_x = (x + mid_x..x + w - wr as i32).step_by(mid_width as usize);
    {
        let size = Size::new(wm, ht);
        let point = Point::new(x2, y1);
        image.sub = Some(Rectangle::new(point, size));
        for sx in range_x.clone() {
            image.render(Point::new(sx, y), &mut state.frame);
        }
    }
    // Bottom edge.
    {
        let size = Size::new(wm, hb);
        let point = Point::new(x2, y3);
        image.sub = Some(Rectangle::new(point, size));
        let sy = y + h - hb as i32;
        for sx in range_x.clone() {
            image.render(Point::new(sx, sy), &mut state.frame);
        }
    }
    // Left edge.
    let range_y = (y + mid_y..y + h - hb as i32).step_by(mid_height as usize);
    {
        let size = Size::new(wl, hm);
        let point = Point::new(x1, y2);
        image.sub = Some(Rectangle::new(point, size));
        for sy in range_y.clone() {
            image.render(Point::new(x, sy), &mut state.frame);
        }
    }
    // Right edge.
    {
        let size = Size::new(wr, hm);
        let point = Point::new(x3, y2);
        image.sub = Some(Rectangle::new(point, size));
        let sx = x + w - wr as i32;
        for sy in range_y.clone() {
            image.render(Point::new(sx, sy), &mut state.frame);
        }
    }

    // Middle.
    {
        let size = Size::new(wm, hm);
        let point = Point::new(x2, y2);
        image.sub = Some(Rectangle::new(point, size));
        for sy in range_y {
            for sx in range_x.clone() {
                image.render(Point::new(sx, sy), &mut state.frame);
            }
        }
    }
}

pub(crate) fn draw_sub_image(
    mut caller: C,
    ptr: u32,
    len: u32,
    x: i32,
    y: i32,
    sub_x: i32,
    sub_y: i32,
    sub_width: u32,
    sub_height: u32,
) {
    let state = caller.data_mut();
    state.called = "graphics.draw_sub_image";
    let sub_point = Point::new(sub_x, sub_y);
    let sub_size = Size::new(sub_width, sub_height);
    let sub = Rectangle::new(sub_point, sub_size);
    draw_image_inner(caller, ptr, len, x, y, Some(sub))
}

pub(crate) fn draw_image(mut caller: C, ptr: u32, len: u32, x: i32, y: i32) {
    let state = caller.data_mut();
    state.called = "graphics.draw_image";
    draw_image_inner(caller, ptr, len, x, y, None)
}

fn draw_image_inner(mut caller: C, ptr: u32, len: u32, x: i32, y: i32, sub: Option<Rectangle>) {
    // Retrieve the raw data from memory.
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(image_bytes) = load_image(state, data, ptr, len) else {
        return;
    };

    let width = u16::from_le_bytes([image_bytes[1], image_bytes[2]]) as u32;
    if width == 0 {
        state.log_error("image has zero width");
        return;
    }
    let transp = u8::from_le_bytes([image_bytes[3]]);
    let image_bytes = &image_bytes[IMG_HEADER..];
    if !(image_bytes.len() * 2).is_multiple_of(width as usize) {
        state.log_error(HostError::InvalidWidth);
        return;
    }

    // TODO: support canvas
    if state.canvas.is_some() {
        state.log_error("images cannot be drawn on canvas yet");
        return;
    }

    let point = Point::new(x, y);
    let image = ParsedImage {
        bytes: image_bytes,
        width,
        transp,
        sub,
    };
    image.render(point, &mut state.frame);
}

fn load_image<'a>(state: &mut State, data: &'a [u8], ptr: u32, len: u32) -> Option<&'a [u8]> {
    let ptr = ptr as usize;
    let len = len as usize;
    let maybe_buf = data.get(ptr..(ptr + len));
    let Some(image_bytes) = &maybe_buf else {
        state.log_error(HostError::OomPointer);
        return None;
    };
    if image_bytes.len() < IMG_HEADER {
        let msg = if ptr == 0 {
            "image is a nil pointer: make sure you've loaded it"
        } else if image_bytes.is_empty() {
            "image file is empty: make sure you load it with the correct name"
        } else {
            "image file is too small"
        };
        state.log_error(msg);
        return None;
    }
    if image_bytes[0] != 0x22 {
        state.log_error("invalid magic number");
        return None;
    }
    Some(image_bytes)
}

fn get_shape_style(fill_color: u32, stroke_color: u32, stroke_width: u32) -> PrimitiveStyle<Gray4> {
    let mut style = PrimitiveStyle::new();
    if fill_color != 0 {
        let fill_color = Gray4::new(fill_color as u8 - 1);
        style.fill_color = Some(fill_color);
    }
    if stroke_color != 0 && stroke_width != 0 {
        let stroke_color = Gray4::new(stroke_color as u8 - 1);
        style.stroke_color = Some(stroke_color);
        style.stroke_width = stroke_width;
    }
    style
}

/// Load mono font from the firefly format.
fn parse_font(bytes: &'_ [u8]) -> Result<MonoFont<'_>, &'static str> {
    if bytes.len() < 10 {
        let msg = if bytes.is_empty() {
            "font is empty: make sure you load it with a correct name"
        } else {
            "font is too short"
        };
        return Err(msg);
    }

    // read the header
    let encoding_index = read_u8(bytes, 1);
    let char_width = u32::from(read_u8(bytes, 2));
    let char_height = u32::from(read_u8(bytes, 3));
    let baseline = u32::from(read_u8(bytes, 4));
    let image_width = u32::from(read_u16(bytes, 5));
    let image = ImageRaw::new(&bytes[7..], image_width);

    let glyph_mapping: &dyn mapping::GlyphMapping = match encoding_index {
        0x0 => &mapping::ASCII,       // ASCII
        0x1 => &mapping::ISO_8859_1,  // Latin-1, Western European.
        0x2 => &mapping::ISO_8859_2,  // Latin-2, Central European.
        0x3 => &mapping::ISO_8859_3,  // Latin-3, South European.
        0x4 => &mapping::ISO_8859_4,  // Latin-4, North European.
        0x5 => &mapping::ISO_8859_9,  // Latin-5, Turkish.
        0x6 => &mapping::ISO_8859_10, // Latin-6, Nordic.
        0x7 => &mapping::ISO_8859_13, // Latin-7, Baltic Rim.
        0x8 => &mapping::ISO_8859_14, // Latin-8, Celtic.
        0x9 => &mapping::ISO_8859_15, // Latin-9 (revised Latin-1).
        0xa => &mapping::ISO_8859_16, // Latin-10: South-East European.
        0xb => &mapping::ISO_8859_5,  // Latin/Cyrillic.
        0xc => &mapping::ISO_8859_7,  // Latin/Greek.
        0xd => &mapping::JIS_X0201,   // Japanese katakana (halfwidth).
        _ => return Err("unknown mapping"),
    };
    let font = MonoFont {
        image,
        character_size: Size::new(char_width, char_height),
        character_spacing: 0,
        baseline,
        strikethrough: DecorationDimensions::new(char_height / 2, 1),
        underline: DecorationDimensions::new(baseline + 2, 1),
        glyph_mapping,
    };
    Ok(font)
}

/// Read little-endian u8 from the slice at the given index.
fn read_u8(bytes: &[u8], i: usize) -> u8 {
    bytes[i]
}

/// Read little-endian u16 from the slice at the given index.
fn read_u16(bytes: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([bytes[i], bytes[i + 1]])
}

fn parse_color(c: i32) -> Option<Gray4> {
    if c == 0 {
        None
    } else {
        Some(Gray4::new(c as u8 - 1))
    }
}

/// Statically ensure that the given Result cannot have an error.
fn never_fails<T>(_: Result<T, Infallible>) {}
