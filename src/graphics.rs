use crate::color::{BPPAdapter, ColorReplaceAdapter};
use crate::state::State;
use core::convert::Infallible;
use embedded_graphics::image::{Image, ImageRaw, ImageRawLE};
use embedded_graphics::mono_font::{mapping, DecorationDimensions, MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::{BinaryColor, Gray2, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;
use firefly_device::Device;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn get_screen_size(caller: C) -> u32 {
    let state = caller.data();
    let size = state.frame.size();
    (size.width << 16) | size.height
}

/// Set every pixel of the frame buffer to the given color.
pub(crate) fn clear_screen(mut caller: C, color: u32) {
    if color == 0 {
        return;
    }
    let state = caller.data_mut();
    let color = Gray2::new(color as u8 - 1);
    never_fails(state.frame.clear(color));
}

/// Set the given palette color.
pub(crate) fn set_color(mut caller: C, index: u32, r: u32, g: u32, b: u32) {
    let state = caller.data_mut();
    state.frame.palette[index as usize - 1] = Rgb888::new(r as u8, g as u8, b as u8);
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
    state.frame.palette[0] = Rgb888::new(c1_r as u8, c1_g as u8, c1_b as u8);
    state.frame.palette[1] = Rgb888::new(c2_r as u8, c2_g as u8, c2_b as u8);
    state.frame.palette[2] = Rgb888::new(c3_r as u8, c3_g as u8, c3_b as u8);
    state.frame.palette[3] = Rgb888::new(c4_r as u8, c4_g as u8, c4_b as u8);
}

/// Draw a single point.
///
/// Without scailing, sets a single pixel.
pub(crate) fn draw_point(mut caller: C, x: i32, y: i32, color: u32) {
    if color == 0 {
        return;
    }
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
    let state = caller.data();
    let Some(memory) = state.memory else {
        state.device.log_error("graphics", "memory not found");
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);

    let text_ptr = text_ptr as usize;
    let text_len = text_len as usize;
    let font_ptr = font_ptr as usize;
    let font_len = font_len as usize;

    // The slices must not intersect and must be within memory.
    if text_ptr == font_ptr {
        let msg = "text and font point to the same slice";
        state.device.log_error("graphics.draw_text", msg);
        return;
    }
    if text_ptr < font_ptr && text_ptr + text_len >= font_ptr {
        let msg = "text and font slices intersect";
        state.device.log_error("graphics.draw_text", msg);
        return;
    }
    if font_ptr < text_ptr && font_ptr + font_len >= text_ptr {
        let msg = "text and font slices intersect";
        state.device.log_error("graphics.draw_text", msg);
        return;
    }

    let Some(text_bytes) = &data.get(text_ptr..(text_ptr + text_len)) else {
        let msg = "text points outside of memory";
        state.device.log_error("graphics.draw_text", msg);
        return;
    };
    let Some(font_bytes) = &data.get(font_ptr..(font_ptr + font_len)) else {
        let msg = "font points outside of memory";
        state.device.log_error("graphics.draw_text", msg);
        return;
    };
    let font = match parse_font(font_bytes) {
        Ok(font) => font,
        Err(err) => {
            state.device.log_error("graphics.draw_text", err);
            return;
        }
    };
    let color = Gray2::new(color as u8 - 1);
    let style = MonoTextStyle::new(&font, color);
    let point = Point::new(x, y);
    let Ok(text) = core::str::from_utf8(text_bytes) else {
        let msg = "the given text is not valid UTF-8";
        state.device.log_error("graphics.draw_text", msg);
        return;
    };
    let text = Text::new(text, point, style);
    never_fails(text.draw(&mut state.frame));
}

pub(crate) fn draw_sub_image(
    caller: C,
    ptr: u32,
    len: u32,
    x: i32,
    y: i32,
    sub_x: i32,
    sub_y: i32,
    sub_width: u32,
    sub_height: u32,
    c1: i32,
    c2: i32,
    c3: i32,
    c4: i32,
) {
    let sub_point = Point::new(sub_x, sub_y);
    let sub_size = Size::new(sub_width, sub_height);
    let sub = Rectangle::new(sub_point, sub_size);
    let colors = parse_4_colors(c1, c2, c3, c4);
    draw_image_inner(caller, ptr, len, x, y, colors, Some(sub))
}

pub(crate) fn draw_image(
    caller: C,
    ptr: u32,
    len: u32,
    x: i32,
    y: i32,
    c1: i32,
    c2: i32,
    c3: i32,
    c4: i32,
) {
    let colors = parse_4_colors(c1, c2, c3, c4);
    draw_image_inner(caller, ptr, len, x, y, colors, None)
}

fn draw_image_inner(
    mut caller: C,
    ptr: u32,
    len: u32,
    x: i32,
    y: i32,
    colors: [Option<Gray2>; 4],
    sub: Option<Rectangle>,
) {
    // retrieve the raw data from memory
    let Some((state, image_bytes)) = get_bytes(&mut caller, ptr, len) else {
        return;
    };
    if image_bytes.len() < 4 {
        return;
    }

    // read image header
    let bpp = u8::from_le_bytes([image_bytes[1]]);
    let width = u16::from_le_bytes([image_bytes[2], image_bytes[3]]) as u32;
    let image_bytes = &image_bytes[4..];

    let point = Point::new(x, y);
    let mut adapter = ColorReplaceAdapter {
        target: &mut state.frame,
        colors,
    };
    if bpp == 1 {
        draw_1bpp(image_bytes, width, point, sub, &mut adapter);
    } else {
        draw_2bpp(image_bytes, width, point, sub, &mut adapter);
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

/// Get State from Store and a slice of bytes from Memory in the given range.
fn get_bytes<'b>(
    caller: &'b mut wasmi::Caller<'_, State>,
    ptr: u32,
    len: u32,
) -> Option<(&'b mut State, &'b [u8])> {
    let state = caller.data();
    let Some(memory) = state.memory else {
        state.device.log_error("graphics", "memory not found");
        return None;
    };
    let (data, state) = memory.data_and_store_mut(caller);
    let ptr = ptr as usize;
    let len = len as usize;
    let Some(bytes) = &data.get(ptr..(ptr + len)) else {
        state.device.log_error("graphics", "slice out of range");
        return None;
    };
    Some((state, bytes))
}

/// Load mono font from the firefly format.
fn parse_font(bytes: &[u8]) -> Result<MonoFont, &str> {
    if bytes.len() < 7 {
        return Err("file too short");
    }

    // read the header
    let encoding_index = read_u8(bytes, 1) as u32;
    let char_width = read_u8(bytes, 2) as u32;
    let char_height = read_u8(bytes, 3) as u32;
    let baseline = read_u8(bytes, 4) as u32;
    let image_width = read_u16(bytes, 5) as u32;
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
        _ => return Err("uknown mapping"),
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

/// Read little-endian u32 from the slice at the given index.
fn read_u8(bytes: &[u8], s: usize) -> u8 {
    u8::from_le_bytes([bytes[s]])
}

/// Read little-endian u32 from the slice at the given index.
fn read_u16(bytes: &[u8], s: usize) -> u16 {
    u16::from_le_bytes([bytes[s], bytes[s + 1]])
}

fn parse_4_colors(c1: i32, c2: i32, c3: i32, c4: i32) -> [Option<Gray2>; 4] {
    [
        parse_color(c1),
        parse_color(c2),
        parse_color(c3),
        parse_color(c4),
    ]
}

fn parse_color(c: i32) -> Option<Gray2> {
    if c == 0 {
        None
    } else {
        Some(Gray2::new(c as u8 - 1))
    }
}

/// Statically ensure that the given Result cannot have an error.
fn never_fails<T>(_: Result<T, Infallible>) {}
