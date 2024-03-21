use crate::graphics;
use crate::state::State;

pub fn link(linker: &mut wasmi::Linker<State>) -> Result<(), wasmi::errors::LinkerError> {
    linker.func_wrap("graphics", "draw_line", graphics::draw_line)?;
    Ok(())
}
