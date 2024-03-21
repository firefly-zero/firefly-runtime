use crate::graphics;
use crate::state::State;

pub(crate) fn link(linker: &mut wasmi::Linker<State>) -> Result<(), wasmi::errors::LinkerError> {
    linker.func_wrap("graphics", "clear", graphics::clear)?;
    linker.func_wrap("graphics", "set_color", graphics::set_color)?;
    linker.func_wrap("graphics", "set_colors", graphics::set_colors)?;
    linker.func_wrap("graphics", "draw_line", graphics::draw_line)?;
    Ok(())
}
