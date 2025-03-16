use crate::host::*;
use crate::state::State;

/// Register all host-defined functions in the linker.
pub(crate) fn link(linker: &mut wasmi::Linker<State>, sudo: bool) -> Result<(), wasmi::Error> {
    linker.func_wrap("graphics", "clear_screen", graphics::clear_screen)?;
    linker.func_wrap("graphics", "set_color", graphics::set_color)?;
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
    linker.func_wrap("graphics", "set_canvas", graphics::set_canvas)?;
    linker.func_wrap("graphics", "unset_canvas", graphics::unset_canvas)?;

    linker.func_wrap("audio", "reset", audio::reset)?;
    linker.func_wrap("audio", "reset_all", audio::reset_all)?;
    linker.func_wrap("audio", "clear", audio::clear)?;
    linker.func_wrap("audio", "add_empty", audio::add_empty)?;
    linker.func_wrap("audio", "add_file", audio::add_file)?;
    linker.func_wrap("audio", "add_mix", audio::add_mix)?;
    linker.func_wrap("audio", "add_all_for_one", audio::add_all_for_one)?;
    linker.func_wrap("audio", "add_gain", audio::add_gain)?;
    linker.func_wrap("audio", "add_loop", audio::add_loop)?;
    linker.func_wrap("audio", "add_concat", audio::add_concat)?;
    linker.func_wrap("audio", "add_pan", audio::add_pan)?;
    linker.func_wrap("audio", "add_mute", audio::add_mute)?;
    linker.func_wrap("audio", "add_pause", audio::add_pause)?;
    linker.func_wrap("audio", "add_track_position", audio::add_track_position)?;
    linker.func_wrap("audio", "add_low_pass", audio::add_low_pass)?;
    linker.func_wrap("audio", "add_high_pass", audio::add_high_pass)?;
    linker.func_wrap("audio", "add_take_left", audio::add_take_left)?;
    linker.func_wrap("audio", "add_take_right", audio::add_take_right)?;
    linker.func_wrap("audio", "add_swap", audio::add_swap)?;
    linker.func_wrap("audio", "add_clip", audio::add_clip)?;
    linker.func_wrap("audio", "add_noise", audio::add_noise)?;
    linker.func_wrap("audio", "add_sine", audio::add_sine)?;
    linker.func_wrap("audio", "add_square", audio::add_square)?;
    linker.func_wrap("audio", "add_sawtooth", audio::add_sawtooth)?;
    linker.func_wrap("audio", "add_triangle", audio::add_triangle)?;
    linker.func_wrap("audio", "add_zero", audio::add_zero)?;
    linker.func_wrap("audio", "mod_linear", audio::mod_linear)?;
    linker.func_wrap("audio", "mod_hold", audio::mod_hold)?;
    linker.func_wrap("audio", "mod_sine", audio::mod_sine)?;

    linker.func_wrap("input", "read_pad", input::read_pad)?;
    linker.func_wrap("input", "read_buttons", input::read_buttons)?;

    linker.func_wrap("menu", "add_menu_item", menu::add_menu_item)?;
    linker.func_wrap("menu", "remove_menu_item", menu::remove_menu_item)?;
    linker.func_wrap("menu", "open_menu", menu::open_menu)?;

    linker.func_wrap("fs", "get_rom_file_size", fs::get_rom_file_size)?; // deprecated
    linker.func_wrap("fs", "load_rom_file", fs::load_rom_file)?; // deprecated
    linker.func_wrap("fs", "get_file_size", fs::get_file_size)?;
    linker.func_wrap("fs", "load_file", fs::load_file)?;
    linker.func_wrap("fs", "dump_file", fs::dump_file)?;
    linker.func_wrap("fs", "remove_file", fs::remove_file)?;

    linker.func_wrap("net", "get_me", net::get_me)?;
    linker.func_wrap("net", "get_peers", net::get_peers)?;
    linker.func_wrap("net", "save_stash", net::save_stash)?;
    linker.func_wrap("net", "load_stash", net::load_stash)?;

    linker.func_wrap("stats", "add_progress", stats::add_progress)?;
    linker.func_wrap("stats", "add_score", stats::add_score)?;

    linker.func_wrap("misc", "log_debug", misc::log_debug)?;
    linker.func_wrap("misc", "log_error", misc::log_error)?;
    linker.func_wrap("misc", "set_seed", misc::set_seed)?;
    linker.func_wrap("misc", "get_random", misc::get_random)?;
    linker.func_wrap("misc", "get_name", misc::get_name)?;
    linker.func_wrap("misc", "restart", misc::restart)?;
    linker.func_wrap("misc", "set_conn_status", misc::set_conn_status)?;
    linker.func_wrap("misc", "quit", misc::quit)?;

    if sudo {
        linker.func_wrap("sudo", "list_dirs", sudo::list_dirs)?;
        linker.func_wrap("sudo", "list_dirs_buf_size", sudo::list_dirs_buf_size)?;
        linker.func_wrap("sudo", "get_file_size", sudo::get_file_size)?;
        linker.func_wrap("sudo", "load_file", sudo::load_file)?;
        linker.func_wrap("sudo", "run_app", sudo::run_app)?;
    }

    // WASI preview 1
    const M: &str = "wasi_snapshot_preview1";
    linker.func_wrap(M, "environ_get", wasip1::environ_get)?;
    linker.func_wrap(M, "environ_sizes_get", wasip1::environ_sizes_get)?;
    linker.func_wrap(M, "clock_time_get", wasip1::clock_time_get)?;
    linker.func_wrap(M, "fd_close", wasip1::fd_close)?;
    linker.func_wrap(M, "fd_read", wasip1::fd_read)?;
    linker.func_wrap(M, "fd_seek", wasip1::fd_seek)?;
    linker.func_wrap(M, "fd_write", wasip1::fd_write)?;
    linker.func_wrap(M, "proc_exit", wasip1::proc_exit)?;

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
        link(&mut linker, true).unwrap();
    }
}
