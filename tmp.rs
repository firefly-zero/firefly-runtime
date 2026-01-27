#[test]
fn test_draw_image_oob_right2() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, draw_image);
    write_mem(&mut store, 5, IMG16);
    let inputs = wrap_input(&[5, IMG16.len() as _, 240 - 2, 2]);
    func.call(&mut store, &inputs, &mut []).unwrap();
    let state = store.data_mut();
    check_display_at(
        &mut state.frame,
        Point::new(240 - 6, 0),
        &[
            "WWWWWW", // y=0
            "WWWWWW", // y=1
            "WWWWWG", // y=2
            "WWWWYM", // y=3
            "WWWWKK", // y=4
            "WWWWKK", // y=5
            "WWWWWW", // y=6
        ],
    );
}
