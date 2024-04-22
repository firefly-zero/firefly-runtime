use crate::state::State;
use crate::{fs, graphics, input, misc, sudo};

/// Register all host-defined functions in the linker.
pub(crate) fn link(linker: &mut wasmi::Linker<State>) -> Result<(), wasmi::Error> {
    linker.func_wrap("graphics", "clear_screen", graphics::clear_screen)?;
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

    linker.func_wrap("input", "read_pad", input::read_pad)?;
    linker.func_wrap("input", "read_buttons", input::read_buttons)?;

    linker.func_wrap("fs", "get_rom_file_size", fs::get_rom_file_size)?;
    linker.func_wrap("fs", "load_rom_file", fs::load_rom_file)?;
    linker.func_wrap("fs", "get_file_size", fs::get_file_size)?;
    linker.func_wrap("fs", "load_file", fs::load_file)?;
    linker.func_wrap("fs", "dump_file", fs::dump_file)?;
    linker.func_wrap("fs", "remove_file", fs::remove_file)?;

    linker.func_wrap("misc", "log_debug", misc::log_debug)?;
    linker.func_wrap("misc", "log_error", misc::log_error)?;
    linker.func_wrap("misc", "set_seed", misc::set_seed)?;
    linker.func_wrap("misc", "get_random", misc::get_random)?;
    linker.func_wrap("misc", "quit", misc::quit)?;

    linker.func_wrap("sudo", "list_dirs", sudo::list_dirs)?;
    linker.func_wrap("sudo", "list_dirs_buf_size", sudo::list_dirs_buf_size)?;
    linker.func_wrap("sudo", "load_file", sudo::load_file)?;
    linker.func_wrap("sudo", "run_app", sudo::run_app)?;

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
