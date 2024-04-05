use crate::state::State;
use crate::{fs, graphics, input};

/// Register all host-defined functions in the linker.
pub(crate) fn link(linker: &mut wasmi::Linker<State>) -> Result<(), wasmi::Error> {
    linker.func_wrap("graphics", "clear", graphics::clear)?;
    linker.func_wrap("graphics", "get_screen_size", graphics::get_screen_size)?;
    linker.func_wrap("graphics", "set_color", graphics::set_color)?;
    linker.func_wrap("graphics", "set_colors", graphics::set_colors)?;
    linker.func_wrap("graphics", "draw_point", graphics::draw_point)?;
    linker.func_wrap("graphics", "draw_line", graphics::draw_line)?;
    linker.func_wrap("graphics", "draw_rect", graphics::draw_rect)?;
    linker.func_wrap("graphics", "draw_rounded_rect", graphics::draw_rounded_rect)?;
    linker.func_wrap("graphics", "draw_circle", graphics::draw_circle)?;
    linker.func_wrap("graphics", "draw_ellipse", graphics::draw_ellipse)?;
    linker.func_wrap("graphics", "draw_triangle", graphics::draw_triangle)?;
    linker.func_wrap("graphics", "draw_arc", graphics::draw_arc)?;
    linker.func_wrap("graphics", "draw_sector", graphics::draw_sector)?;
    linker.func_wrap("graphics", "draw_text", graphics::draw_text)?;
    linker.func_wrap("graphics", "draw_image", graphics::draw_image)?;
    linker.func_wrap("graphics", "draw_sub_image", graphics::draw_sub_image)?;

    linker.func_wrap("input", "read_left", input::read_left)?;
    linker.func_wrap("input", "read_right", input::read_right)?;
    linker.func_wrap("input", "read_buttons", input::read_buttons)?;

    linker.func_wrap("fs", "load_rom_file", fs::load_rom_file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::link;
    use crate::state::State;

    #[test]
    fn smoke_test_linking() {
        let engine = wasmi::Engine::default();
        let mut linker = <wasmi::Linker<State>>::new(&engine);
        link(&mut linker).unwrap();
    }
}
