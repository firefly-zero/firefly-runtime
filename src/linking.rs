use crate::host::*;
use crate::state::State;

/// Register all host-defined functions in the linker.
pub(crate) fn link(linker: &mut wasmi::Linker<State>) -> Result<(), wasmi::Error> {
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

    linker.func_wrap("input", "read_pad", input::read_pad)?;
    linker.func_wrap("input", "read_buttons", input::read_buttons)?;

    linker.func_wrap("menu", "add_menu_item", menu::add_menu_item)?;
    linker.func_wrap("menu", "remove_menu_item", menu::remove_menu_item)?;
    linker.func_wrap("menu", "open_menu", menu::open_menu)?;

    linker.func_wrap("fs", "get_rom_file_size", fs::get_rom_file_size)?;
    linker.func_wrap("fs", "load_rom_file", fs::load_rom_file)?;
    linker.func_wrap("fs", "get_file_size", fs::get_file_size)?;
    linker.func_wrap("fs", "load_file", fs::load_file)?;
    linker.func_wrap("fs", "dump_file", fs::dump_file)?;
    linker.func_wrap("fs", "remove_file", fs::remove_file)?;

    linker.func_wrap("net", "is_online", net::is_online)?;
    linker.func_wrap("net", "get_player_id", net::get_player_id)?;

    linker.func_wrap("misc", "log_debug", misc::log_debug)?;
    linker.func_wrap("misc", "log_error", misc::log_error)?;
    linker.func_wrap("misc", "set_seed", misc::set_seed)?;
    linker.func_wrap("misc", "get_random", misc::get_random)?;
    linker.func_wrap("misc", "quit", misc::quit)?;

    linker.func_wrap("sudo", "list_dirs", sudo::list_dirs)?;
    linker.func_wrap("sudo", "list_dirs_buf_size", sudo::list_dirs_buf_size)?;
    linker.func_wrap("sudo", "get_file_size", sudo::get_file_size)?;
    linker.func_wrap("sudo", "load_file", sudo::load_file)?;
    linker.func_wrap("sudo", "run_app", sudo::run_app)?;

    // WASI preview 1
    const M: &str = "wasi_snapshot_preview1";
    linker.func_wrap(M, "args_get", wasip1::args_get)?;
    linker.func_wrap(M, "args_sizes_get", wasip1::args_sizes_get)?;
    linker.func_wrap(M, "environ_get", wasip1::environ_get)?;
    linker.func_wrap(M, "environ_sizes_get", wasip1::environ_sizes_get)?;
    linker.func_wrap(M, "clock_res_get", wasip1::clock_res_get)?;
    linker.func_wrap(M, "clock_time_get", wasip1::clock_time_get)?;
    linker.func_wrap(M, "fd_advise", wasip1::fd_advise)?;
    linker.func_wrap(M, "fd_allocate", wasip1::fd_allocate)?;
    linker.func_wrap(M, "fd_close", wasip1::fd_close)?;
    linker.func_wrap(M, "fd_datasync", wasip1::fd_datasync)?;
    linker.func_wrap(M, "fd_fdstat_get", wasip1::fd_fdstat_get)?;
    linker.func_wrap(M, "fd_fdstat_set_flags", wasip1::fd_fdstat_set_flags)?;
    linker.func_wrap(M, "fd_fdstat_set_rights", wasip1::fd_fdstat_set_rights)?;
    linker.func_wrap(M, "fd_filestat_get", wasip1::fd_filestat_get)?;
    linker.func_wrap(M, "fd_filestat_set_size", wasip1::fd_filestat_set_size)?;
    linker.func_wrap(M, "fd_filestat_set_times", wasip1::fd_filestat_set_times)?;
    linker.func_wrap(M, "fd_pread", wasip1::fd_pread)?;
    linker.func_wrap(M, "fd_prestat_get", wasip1::fd_prestat_get)?;
    linker.func_wrap(M, "fd_prestat_dir_name", wasip1::fd_prestat_dir_name)?;
    linker.func_wrap(M, "fd_pwrite", wasip1::fd_pwrite)?;
    linker.func_wrap(M, "fd_read", wasip1::fd_read)?;
    linker.func_wrap(M, "fd_readdir", wasip1::fd_readdir)?;
    linker.func_wrap(M, "fd_renumber", wasip1::fd_renumber)?;
    linker.func_wrap(M, "fd_seek", wasip1::fd_seek)?;
    linker.func_wrap(M, "fd_sync", wasip1::fd_sync)?;
    linker.func_wrap(M, "fd_tell", wasip1::fd_tell)?;
    linker.func_wrap(M, "fd_write", wasip1::fd_write)?;
    linker.func_wrap(M, "path_create_directory", wasip1::path_create_directory)?;
    linker.func_wrap(M, "path_filestat_get", wasip1::path_filestat_get)?;
    linker.func_wrap(
        M,
        "path_filestat_set_times",
        wasip1::path_filestat_set_times,
    )?;
    linker.func_wrap(M, "path_link", wasip1::path_link)?;
    linker.func_wrap(M, "path_open", wasip1::path_open)?;
    linker.func_wrap(M, "path_readlink", wasip1::path_readlink)?;
    linker.func_wrap(M, "path_remove_directory", wasip1::path_remove_directory)?;
    linker.func_wrap(M, "path_rename", wasip1::path_rename)?;
    linker.func_wrap(M, "path_symlink", wasip1::path_symlink)?;
    linker.func_wrap(M, "path_unlink_file", wasip1::path_unlink_file)?;
    linker.func_wrap(M, "poll_oneoff", wasip1::poll_oneoff)?;
    linker.func_wrap(M, "proc_exit", wasip1::proc_exit)?;
    linker.func_wrap(M, "proc_raise", wasip1::proc_raise)?;
    linker.func_wrap(M, "sched_yield", wasip1::sched_yield)?;
    linker.func_wrap(M, "random_get", wasip1::random_get)?;
    linker.func_wrap(M, "sock_accept", wasip1::sock_accept)?;
    linker.func_wrap(M, "sock_recv", wasip1::sock_recv)?;
    linker.func_wrap(M, "sock_send", wasip1::sock_send)?;
    linker.func_wrap(M, "sock_shutdown", wasip1::sock_shutdown)?;

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
