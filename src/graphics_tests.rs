#[cfg(test)]
mod tests {
    use crate::graphics::*;
    use crate::{frame_buffer::FrameBuffer, state::State};
    use embedded_graphics::draw_target::DrawTargetExt;
    use embedded_graphics::geometry::{Point, Size};
    use embedded_graphics::mock_display::MockDisplay;
    use embedded_graphics::pixelcolor::Rgb888;
    use embedded_graphics::primitives::Rectangle;
    use firefly_device::DeviceImpl;
    use std::path::PathBuf;

    // const N: i32 = 0;
    // const W: i32 = 1;
    const G: i32 = 2;
    const R: i32 = 3;
    const B: i32 = 4;

    #[test]
    fn test_clear() {
        let mut store = make_store();
        let func = wasmi::Func::wrap(&mut store, clear);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in &state.frame.data {
            assert_eq!(byte, &0b_0000_0000);
        }

        let inputs = wrap_input(&[2]);
        let mut outputs = Vec::new();
        func.call(&mut store, &inputs, &mut outputs).unwrap();
        assert_eq!(outputs.len(), 0);

        // check that all pixel in the frame buffer are set to 1.
        let state = store.data();
        for byte in &state.frame.data {
            assert_eq!(byte, &0b_0101_0101);
        }
    }

    #[test]
    fn test_draw_line() {
        let mut store = make_store();
        let func = wasmi::Func::wrap(&mut store, draw_line);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in &state.frame.data {
            assert_eq!(byte, &0b_0000_0000);
        }

        store.data_mut().frame.mark_clean();
        let inputs = wrap_input(&[2, 1, 4, 3, R, 1]);
        func.call(&mut store, &inputs, &mut []).unwrap();

        let state = store.data_mut();
        check_display(
            &mut state.frame,
            &[
                "      ", // y=0
                "WWRWWW", // y=1
                "WWWRWW", // y=2
                "WWWWRW", // y=3
            ],
        );
    }

    /// Drawing a line out of screen bounds
    /// should simply cut the line at the boundary.
    #[test]
    fn test_draw_line_out_of_bounds() {
        let mut store = make_store();
        let func = wasmi::Func::wrap(&mut store, draw_line);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in &state.frame.data {
            assert_eq!(byte, &0b_0000_0000);
        }

        store.data_mut().frame.mark_clean();
        let inputs = wrap_input(&[-1, -2, 4, 3, G, 1]);
        func.call(&mut store, &inputs, &mut []).unwrap();

        let state = store.data_mut();
        check_display(
            &mut state.frame,
            &[
                "WGWWWW", // y=0
                "WWGWWW", // y=1
                "WWWGWW", // y=2
            ],
        );
    }

    #[test]
    fn test_draw_rect() {
        let mut store = make_store();
        let func = wasmi::Func::wrap(&mut store, draw_rect);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in &state.frame.data {
            assert_eq!(byte, &0b_0000_0000);
        }

        store.data_mut().frame.mark_clean();
        let inputs = wrap_input(&[1, 2, 4, 3, G, B, 1]);
        func.call(&mut store, &inputs, &mut []).unwrap();

        let state = store.data_mut();
        check_display(
            &mut state.frame,
            &[
                "      ", // y=0
                "      ", // y=1
                "WBBBBW", // y=2
                "WBGGBW", // y=3
                "WBBBBW", // y=4
            ],
        );
    }

    #[test]
    fn test_draw_rounded_rect() {
        let mut store = make_store();
        let func = wasmi::Func::wrap(&mut store, draw_rounded_rect);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in &state.frame.data {
            assert_eq!(byte, &0b_0000_0000);
        }

        store.data_mut().frame.mark_clean();
        let inputs = wrap_input(&[1, 2, 4, 4, 2, 2, G, B, 1]);
        func.call(&mut store, &inputs, &mut []).unwrap();

        let state = store.data_mut();
        check_display(
            &mut state.frame,
            &[
                "      ", // y=0
                "      ", // y=1
                "WWBBWW", // y=2
                "WBGGBW", // y=3
                "WBGGBW", // y=3
                "WWBBWW", // y=4
            ],
        );
    }

    #[test]
    fn test_draw_circle() {
        let mut store = make_store();
        let func = wasmi::Func::wrap(&mut store, draw_circle);

        // ensure that the frame buffer is empty
        let state = store.data();
        for byte in &state.frame.data {
            assert_eq!(byte, &0b_0000_0000);
        }

        store.data_mut().frame.mark_clean();
        let inputs = wrap_input(&[1, 2, 4, G, R, 1]);
        func.call(&mut store, &inputs, &mut []).unwrap();

        let state = store.data_mut();
        check_display(
            &mut state.frame,
            &[
                "      ", // y=0
                "      ", // y=1
                "WWRRWW", // y=2
                "WRGGRW", // y=3
                "WRGGRW", // y=3
                "WWRRWW", // y=4
            ],
        );
    }

    fn wrap_input(a: &[i32]) -> Vec<wasmi::Value> {
        let mut res = Vec::new();
        for el in a {
            res.push(wasmi::Value::I32(*el))
        }
        res
    }

    fn check_display(frame: &mut FrameBuffer, pattern: &[&str]) {
        let mut display = MockDisplay::<Rgb888>::new();
        let area = Rectangle::new(Point::zero(), Size::new(6, 64));
        let mut sub_display = display.clipped(&area);
        frame.palette = [
            Rgb888::new(255, 255, 255),
            Rgb888::new(0, 255, 0),
            Rgb888::new(255, 0, 0),
            Rgb888::new(0, 0, 255),
        ];
        frame.draw(&mut sub_display).unwrap();
        display.assert_pattern(pattern);
    }

    fn make_store() -> wasmi::Store<State> {
        let engine = wasmi::Engine::default();
        let root = PathBuf::from("/tmp");
        let device = DeviceImpl::new(root);
        let state = State::new("au", "ap", device);
        wasmi::Store::new(&engine, state)
    }
}
