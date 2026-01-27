use crate::color::Rgb16;
use crate::config::FullID;
use crate::frame_buffer::FrameBuffer;
use crate::host::graphics::*;
use crate::state::{NetHandler, State};
use embedded_graphics::draw_target::DrawTargetExt;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mock_display::MockDisplay;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::RgbColor;
use embedded_graphics::primitives::Rectangle;
use firefly_hal::{Device, DeviceConfig, DeviceImpl};
use std::path::PathBuf;

const N: i32 = 0;
// const W: i32 = 1;
const G: i32 = 2;
const R: i32 = 3;
const B: i32 = 4;

/// A 4x4 16 BPP image with all 16 colors.
static IMG16: &[u8] = &[
    // header
    0x21, // magic number (marker that signals that this is an image)
    0x04, // bits per pixel (BPP, either 0x01, 0x02, or 0x04)
    0x04, // ┬ image width, 16 bit little-endian
    0x00, // ┘
    0xff, // transparency color
    0x01, // ┬ 8 bytes color palette (16 colors)
    0x23, // ┤
    0x45, // ┤
    0x67, // ┤
    0x89, // ┤
    0xAB, // ┤
    0xCD, // ┤
    0xEF, // ┘
    // pixels
    0x01, 0x23, // row 1
    0x45, 0x67, // row 2
    0x89, 0xAB, // row 3
    0xCD, 0xEF, // row 4
];

#[test]
fn test_clear_screen() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, clear_screen);

    let inputs = wrap_input(&[2]);
    let mut outputs = Vec::new();
    func.call(&mut store, &inputs, &mut outputs).unwrap();
    assert_eq!(outputs.len(), 0);

    // check that all pixel in the frame buffer are set to 1.
    let state = store.data();
    for byte in &*state.frame.data {
        assert_eq!(byte, &0b_0001_0001);
    }
}

#[test]
fn test_draw_line() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_line);

    let inputs = wrap_input(&[2, 1, 4, 3, R, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWRWWW", // y=1
            "WWWRWW", // y=2
            "WWWWRW", // y=3
            "WWWWWW", // y=4
            "WWWWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

/// Drawing a line out of screen bounds
/// should simply cut the line at the boundary.
#[test]
fn test_draw_line_out_of_bounds() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_line);

    let inputs = wrap_input(&[-1, -2, 4, 3, G, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WGWWWW", // y=0
            "WWGWWW", // y=1
            "WWWGWW", // y=2
            "WWWWGW", // y=3
            "WWWWWW", // y=4
            "WWWWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_rect_filled() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_rect);

    let inputs = wrap_input(&[1, 2, 4, 3, G, B, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "WBBBBW", // y=2
            "WBGGBW", // y=3
            "WBBBBW", // y=4
            "WWWWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_rect_solid_w4() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_rect);

    let inputs = wrap_input(&[1, 2, 4, 3, G, N, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "WGGGGW", // y=2
            "WGGGGW", // y=3
            "WGGGGW", // y=4
            "WWWWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_rect_solid_w5() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_rect);

    let inputs = wrap_input(&[1, 2, 5, 3, G, N, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWWW", // y=0
            "WWWWWWW", // y=1
            "WGGGGGW", // y=2
            "WGGGGGW", // y=3
            "WGGGGGW", // y=4
            "WWWWWWW", // y=5
            "WWWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_rounded_rect() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_rounded_rect);

    let inputs = wrap_input(&[1, 2, 4, 4, 2, 2, G, B, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "WWBBWW", // y=2
            "WBGGBW", // y=3
            "WBGGBW", // y=4
            "WWBBWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_circle() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_circle);

    let inputs = wrap_input(&[1, 2, 4, G, R, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "WWRRWW", // y=2
            "WRGGRW", // y=3
            "WRGGRW", // y=4
            "WWRRWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

/// Draw circle paritally out-of-bounds on the left.
#[test]
fn test_draw_circle_part_oob_left() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_circle);

    let inputs = wrap_input(&[-2, 2, 4, G, R, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWW", // y=0
            "WWWWW", // y=1
            "RWWWW", // y=2
            "GRWWW", // y=3
            "GRWWW", // y=4
            "RWWWW", // y=5
            "WWWWW", // y=6
        ],
    );
}

/// Draw circle paritally out-of-bounds on the left.
#[test]
fn test_draw_circle_part_oob_top() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_circle);

    let inputs = wrap_input(&[1, -1, 4, G, R, 1]);
    func.call(&mut store, &inputs, &mut []).unwrap();

    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WRGGRW", // y=0
            "WRGGRW", // y=1
            "WWRRWW", // y=2
            "WWWWWW", // y=3
            "WWWWWW", // y=4
            "WWWWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_image() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_image);
    write_mem(&mut store, 5, IMG16);
    let inputs = wrap_input(&[5, IMG16.len() as _, 1, 2]);
    func.call(&mut store, &inputs, &mut []).unwrap();
    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "WWGRBW", // y=2
            "WYMCKW", // y=3
            "WKKKKW", // y=4
            "WKKKKW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_image_oob_left1() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_image);
    write_mem(&mut store, 5, IMG16);
    let inputs = wrap_input(&[5, IMG16.len() as _, -1, 2]);
    func.call(&mut store, &inputs, &mut []).unwrap();
    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "GRBWWW", // y=2
            "MCKWWW", // y=3
            "KKKWWW", // y=4
            "KKKWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_image_oob_left2() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_image);
    write_mem(&mut store, 5, IMG16);
    let inputs = wrap_input(&[5, IMG16.len() as _, -2, 2]);
    func.call(&mut store, &inputs, &mut []).unwrap();
    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "RBWWWW", // y=2
            "CKWWWW", // y=3
            "KKWWWW", // y=4
            "KKWWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

#[test]
fn test_draw_image_oob_top() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_image);
    write_mem(&mut store, 5, IMG16);
    let inputs = wrap_input(&[5, IMG16.len() as _, 1, -1]);
    func.call(&mut store, &inputs, &mut []).unwrap();
    let state = store.data_mut();
    check_display(
        &mut state.frame,
        &[
            "WYMCKW", // y=0
            "WKKKKW", // y=1
            "WKKKKW", // y=2
            "WWWWWW", // y=3
            "WWWWWW", // y=4
            "WWWWWW", // y=5
            "WWWWWW", // y=6
        ],
    );
}

/// Place the given buffer into the linear wasm app memory.
fn write_mem(store: &mut wasmi::Store<Box<State<'_>>>, addr: usize, buf: &[u8]) {
    let mem_type = wasmi::MemoryType::new(1, Some(1));
    let memory = wasmi::Memory::new(&mut *store, mem_type).unwrap();
    let state = store.data_mut();
    state.memory = Some(memory);

    let data = memory.data_mut(store);
    data[addr..addr + buf.len()].copy_from_slice(buf);
}

fn wrap_input(a: &[i32]) -> Vec<wasmi::Val> {
    let mut res = Vec::new();
    for el in a {
        res.push(wasmi::Val::I32(*el))
    }
    res
}

fn check_display(frame: &mut FrameBuffer, pattern: &[&str]) {
    let mut display = MockDisplay::<Rgb888>::new();
    let w = pattern[0].len() as u32;
    let area = Rectangle::new(Point::zero(), Size::new(w, 7));
    let mut sub_display = display.clipped(&area);
    frame.palette = [
        // 0-4
        Rgb16::WHITE,
        Rgb16::GREEN,
        Rgb16::RED,
        Rgb16::BLUE,
        // 4-8
        Rgb16::YELLOW,
        Rgb16::MAGENTA,
        Rgb16::CYAN,
        Rgb16::BLACK,
        // 8-12
        Rgb16::BLACK,
        Rgb16::BLACK,
        Rgb16::BLACK,
        Rgb16::BLACK,
        // 12-16
        Rgb16::BLACK,
        Rgb16::BLACK,
        Rgb16::BLACK,
        Rgb16::BLACK,
    ];
    frame.draw(&mut sub_display).unwrap();
    display.assert_pattern(pattern);
}

fn make_store<'a>() -> wasmi::Store<Box<State<'a>>> {
    let engine = wasmi::Engine::default();
    let root = get_vfs();
    let config = DeviceConfig {
        root,
        ..Default::default()
    };
    let mut device = DeviceImpl::new(config);
    let rom_dir = device.open_dir(&["sys"]).ok().unwrap();
    let id = FullID::from_str("test-author", "test-app").unwrap();
    let state = State::new(id, device, rom_dir, NetHandler::None, false);
    let store = wasmi::Store::new(&engine, state);
    assert_fb_empty(&store);
    store
}

/// Ensure that the frame buffer is empty.
fn assert_fb_empty(store: &wasmi::Store<Box<State<'_>>>) {
    let state = store.data();
    for byte in &*state.frame.data {
        assert_eq!(byte, &0b_0000_0000);
    }
}
fn get_vfs() -> PathBuf {
    let root = std::env::temp_dir();
    _ = std::fs::create_dir(root.join("sys"));
    root
}
