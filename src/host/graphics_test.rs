use crate::config::FullID;
use crate::frame_buffer::FrameBuffer;
use crate::host::graphics::*;
use crate::state::{NetHandler, State};
use embedded_graphics::geometry::Point;
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
    0xab, // ┤
    0xcd, // ┤
    0xef, // ┘
    // pixels
    0x01, 0x23, // row 1
    0x45, 0x67, // row 2
    0x89, 0xab, // row 3
    0xcd, 0xef, // row 4
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
        &state.frame,
        &[
            "......", // y=0
            "..R...", // y=1
            "...R..", // y=2
            "....R.", // y=3
            "......", // y=4
            "......", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            ".P....", // y=0
            "..P...", // y=1
            "...P..", // y=2
            "....P.", // y=3
            "......", // y=4
            "......", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            "......", // y=0
            "......", // y=1
            ".OOOO.", // y=2
            ".OPPO.", // y=3
            ".OOOO.", // y=4
            "......", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            "......", // y=0
            "......", // y=1
            ".PPPP.", // y=2
            ".PPPP.", // y=3
            ".PPPP.", // y=4
            "......", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            ".......", // y=0
            ".......", // y=1
            ".PPPPP.", // y=2
            ".PPPPP.", // y=3
            ".PPPPP.", // y=4
            ".......", // y=5
            ".......", // y=6
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
        &state.frame,
        &[
            "......", // y=0
            "......", // y=1
            "..OO..", // y=2
            ".OPPO.", // y=3
            ".OPPO.", // y=4
            "..OO..", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            "......", // y=0
            "......", // y=1
            "..RR..", // y=2
            ".RPPR.", // y=3
            ".RPPR.", // y=4
            "..RR..", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            ".....", // y=0
            ".....", // y=1
            "R....", // y=2
            "PR...", // y=3
            "PR...", // y=4
            "R....", // y=5
            ".....", // y=6
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
        &state.frame,
        &[
            ".RPPR.", // y=0
            ".RPPR.", // y=1
            "..RR..", // y=2
            "......", // y=3
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
        &state.frame,
        &[
            "......", // y=0
            "......", // y=1
            "..PRO.", // y=2
            ".YgGD.", // y=3
            ".dBbC.", // y=4
            ".W◔◑◕.", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            "......", // y=0
            "......", // y=1
            "PRO...", // y=2
            "gGD...", // y=3
            "BbC...", // y=4
            "◔◑◕...", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            "......", // y=0
            "......", // y=1
            "RO....", // y=2
            "GD....", // y=3
            "bC....", // y=4
            "◑◕....", // y=5
            "......", // y=6
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
        &state.frame,
        &[
            ".YgGD.", // y=0
            ".dBbC.", // y=1
            ".W◔◑◕.", // y=2
            "......", // y=3
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

fn check_display(frame: &FrameBuffer, pattern: &[&str]) {
    for (line, y) in pattern.iter().zip(0..) {
        for (expected, x) in line.chars().zip(0..) {
            let point = Point::new(x, y);
            let actual = get_fb_char(frame, point);
            assert_eq!(actual, expected, "invalid color at x={x}, y={y}")
        }
    }
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

fn get_fb_char(frame: &FrameBuffer, point: Point) -> char {
    const WIDTH: usize = 240;
    const PPB: usize = 2;
    const CHARS: &str = ".PROYgGDdBbCW◔◑◕";
    let x = point.x as usize;
    let y = point.y as usize;
    let pixel_index = y * WIDTH + x;
    let byte_index = pixel_index / PPB;
    let byte = frame.data[byte_index];
    let luma = if x.is_multiple_of(2) { byte } else { byte >> 4 };
    let luma = (luma & 0xf) as usize;
    CHARS.chars().nth(luma).unwrap()
}

fn get_vfs() -> PathBuf {
    let root = std::env::temp_dir();
    _ = std::fs::create_dir(root.join("sys"));
    root
}
