use crate::host::*;
use crate::state::State;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use wasmi::Func;

/// A linking error that may be returned by [`populate_externals`].
pub enum LinkingError {
    SudoDisabled,
    UnknownHostFunction(String, String),
}

impl fmt::Display for LinkingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SudoDisabled => {
                write!(f, "sudo is disabled")
            }
            Self::UnknownHostFunction(module, name) => {
                write!(f, "unknown host function: {module}.{name}")
            }
        }
    }
}

/// Populate all host-defined functions used by `module` in the `extern` vector.
///
/// If `sudo` is enabled, some more host-defined functions are allowed to be used.
pub(crate) fn populate_externals<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    module: &wasmi::Module,
    sudo: bool,
    externs: &mut Vec<wasmi::Extern>,
) -> Result<(), LinkingError> {
    let mut ctx = ctx;
    let imports = module.imports();
    externs.reserve(imports.len());
    for import in imports {
        let ctx = ctx.as_context_mut();
        let module_name = import.module();
        let fn_name = import.name();
        let maybe_func = match module_name {
            "graphics" => select_graphics_external(ctx, fn_name),
            "audio" => select_audio_external(ctx, fn_name),
            "input" => select_input_external(ctx, fn_name),
            "menu" => select_menu_external(ctx, fn_name),
            "fs" => select_fs_external(ctx, fn_name),
            "net" => select_net_external(ctx, fn_name),
            "stats" => select_stats_external(ctx, fn_name),
            "misc" => select_misc_external(ctx, fn_name),
            "sudo" => {
                if !sudo {
                    return Err(LinkingError::SudoDisabled);
                }
                select_sudo_external(ctx, fn_name)
            }
            "wasi_snapshot_preview1" => select_wasip1_external(ctx, fn_name),
            "g" => select_graphics_external_alias(ctx, fn_name),
            "i" => select_input_external_alias(ctx, fn_name),
            "n" => select_net_external_alias(ctx, fn_name),
            "s" => select_stats_external_alias(ctx, fn_name),
            "m" => select_misc_external_alias(ctx, fn_name),
            _ => None,
        };
        let Some(func) = maybe_func else {
            return Err(LinkingError::UnknownHostFunction(
                module_name.to_string(),
                fn_name.to_string(),
            ));
        };
        externs.push(wasmi::Extern::Func(func));
    }
    Ok(())
}

fn select_graphics_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "clear_screen" => Func::wrap(ctx, graphics::clear_screen),
        "set_color" => Func::wrap(ctx, graphics::set_color),
        "draw_point" => Func::wrap(ctx, graphics::draw_point),
        "draw_line" => Func::wrap(ctx, graphics::draw_line),
        "draw_rect" => Func::wrap(ctx, graphics::draw_rect),
        "draw_rounded_rect" => Func::wrap(ctx, graphics::draw_rounded_rect),
        "draw_circle" => Func::wrap(ctx, graphics::draw_circle),
        "draw_ellipse" => Func::wrap(ctx, graphics::draw_ellipse),
        "draw_triangle" => Func::wrap(ctx, graphics::draw_triangle),
        "draw_arc" => Func::wrap(ctx, graphics::draw_arc),
        "draw_sector" => Func::wrap(ctx, graphics::draw_sector),
        "draw_qr" => Func::wrap(ctx, graphics::draw_qr),
        "draw_text" => Func::wrap(ctx, graphics::draw_text),
        "draw_image" => Func::wrap(ctx, graphics::draw_image),
        "draw_sub_image" => Func::wrap(ctx, graphics::draw_sub_image),
        "set_canvas" => Func::wrap(ctx, graphics::set_canvas),
        "unset_canvas" => Func::wrap(ctx, graphics::unset_canvas),
        _ => return None,
    };
    Some(func)
}

fn select_audio_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "reset" => Func::wrap(ctx, audio::reset),
        "reset_all" => Func::wrap(ctx, audio::reset_all),
        "clear" => Func::wrap(ctx, audio::clear),
        "add_empty" => Func::wrap(ctx, audio::add_empty),
        "add_file" => Func::wrap(ctx, audio::add_file),
        "add_mix" => Func::wrap(ctx, audio::add_mix),
        "add_all_for_one" => Func::wrap(ctx, audio::add_all_for_one),
        "add_gain" => Func::wrap(ctx, audio::add_gain),
        "add_loop" => Func::wrap(ctx, audio::add_loop),
        "add_concat" => Func::wrap(ctx, audio::add_concat),
        "add_pan" => Func::wrap(ctx, audio::add_pan),
        "add_mute" => Func::wrap(ctx, audio::add_mute),
        "add_pause" => Func::wrap(ctx, audio::add_pause),
        "add_track_position" => Func::wrap(ctx, audio::add_track_position),
        "add_low_pass" => Func::wrap(ctx, audio::add_low_pass),
        "add_high_pass" => Func::wrap(ctx, audio::add_high_pass),
        "add_take_left" => Func::wrap(ctx, audio::add_take_left),
        "add_take_right" => Func::wrap(ctx, audio::add_take_right),
        "add_swap" => Func::wrap(ctx, audio::add_swap),
        "add_clip" => Func::wrap(ctx, audio::add_clip),
        "add_noise" => Func::wrap(ctx, audio::add_noise),
        "add_sine" => Func::wrap(ctx, audio::add_sine),
        "add_square" => Func::wrap(ctx, audio::add_square),
        "add_sawtooth" => Func::wrap(ctx, audio::add_sawtooth),
        "add_triangle" => Func::wrap(ctx, audio::add_triangle),
        "add_zero" => Func::wrap(ctx, audio::add_zero),
        "mod_linear" => Func::wrap(ctx, audio::mod_linear),
        "mod_hold" => Func::wrap(ctx, audio::mod_hold),
        "mod_sine" => Func::wrap(ctx, audio::mod_sine),
        _ => return None,
    };
    Some(func)
}

fn select_input_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "read_pad" => Func::wrap(ctx, input::read_pad),
        "read_buttons" => Func::wrap(ctx, input::read_buttons),
        _ => return None,
    };
    Some(func)
}

fn select_menu_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "add_menu_item" => Func::wrap(ctx, menu::add_menu_item),
        "remove_menu_item" => Func::wrap(ctx, menu::remove_menu_item),
        "open_menu" => Func::wrap(ctx, menu::open_menu),
        _ => return None,
    };
    Some(func)
}

fn select_fs_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "get_rom_file_size" => Func::wrap(ctx, fs::get_rom_file_size),
        "load_rom_file" => Func::wrap(ctx, fs::load_rom_file),
        "get_file_size" => Func::wrap(ctx, fs::get_file_size),
        "load_file" => Func::wrap(ctx, fs::load_file),
        "dump_file" => Func::wrap(ctx, fs::dump_file),
        "remove_file" => Func::wrap(ctx, fs::remove_file),
        _ => return None,
    };
    Some(func)
}

fn select_net_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "get_me" => Func::wrap(ctx, net::get_me),
        "get_peers" => Func::wrap(ctx, net::get_peers),
        "save_stash" => Func::wrap(ctx, net::save_stash),
        "load_stash" => Func::wrap(ctx, net::load_stash),
        _ => return None,
    };
    Some(func)
}

fn select_stats_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "add_progress" => Func::wrap(ctx, stats::add_progress),
        "add_score" => Func::wrap(ctx, stats::add_score),
        _ => return None,
    };
    Some(func)
}

fn select_misc_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "log_debug" => Func::wrap(ctx, misc::log_debug),
        "log_error" => Func::wrap(ctx, misc::log_error),
        "set_seed" => Func::wrap(ctx, misc::set_seed),
        "get_random" => Func::wrap(ctx, misc::get_random),
        "get_name" => Func::wrap(ctx, misc::get_name),
        "restart" => Func::wrap(ctx, misc::restart),
        "set_conn_status" => Func::wrap(ctx, misc::set_conn_status),
        "quit" => Func::wrap(ctx, misc::quit),
        _ => return None,
    };
    Some(func)
}

fn select_sudo_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "list_dirs" => Func::wrap(ctx, sudo::list_dirs),
        "list_dirs_buf_size" => Func::wrap(ctx, sudo::list_dirs_buf_size),
        "list_files" => Func::wrap(ctx, sudo::list_files),
        "list_files_buf_size" => Func::wrap(ctx, sudo::list_files_buf_size),
        "get_file_size" => Func::wrap(ctx, sudo::get_file_size),
        "load_file" => Func::wrap(ctx, sudo::load_file),
        "run_app" => Func::wrap(ctx, sudo::run_app),
        _ => return None,
    };
    Some(func)
}

fn select_wasip1_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "environ_get" => Func::wrap(ctx, wasip1::environ_get),
        "environ_sizes_get" => Func::wrap(ctx, wasip1::environ_sizes_get),
        "clock_time_get" => Func::wrap(ctx, wasip1::clock_time_get),
        "fd_close" => Func::wrap(ctx, wasip1::fd_close),
        "fd_read" => Func::wrap(ctx, wasip1::fd_read),
        "fd_seek" => Func::wrap(ctx, wasip1::fd_seek),
        "fd_write" => Func::wrap(ctx, wasip1::fd_write),
        "proc_exit" => Func::wrap(ctx, wasip1::proc_exit),
        _ => return None,
    };
    Some(func)
}

fn select_graphics_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "a" => Func::wrap(ctx, graphics::draw_arc),
        "c" => Func::wrap(ctx, graphics::draw_circle),
        "ca" => Func::wrap(ctx, graphics::set_canvas),
        "cr" => Func::wrap(ctx, graphics::unset_canvas),
        "cs" => Func::wrap(ctx, graphics::clear_screen),
        "e" => Func::wrap(ctx, graphics::draw_ellipse),
        "i" => Func::wrap(ctx, graphics::draw_image),
        "l" => Func::wrap(ctx, graphics::draw_line),
        "p" => Func::wrap(ctx, graphics::draw_point),
        "r" => Func::wrap(ctx, graphics::draw_rect),
        "rr" => Func::wrap(ctx, graphics::draw_rounded_rect),
        "s" => Func::wrap(ctx, graphics::draw_sector),
        "sc" => Func::wrap(ctx, graphics::set_color),
        "si" => Func::wrap(ctx, graphics::draw_sub_image),
        "t" => Func::wrap(ctx, graphics::draw_triangle),
        "x" => Func::wrap(ctx, graphics::draw_text),
        "q" => Func::wrap(ctx, graphics::draw_qr),
        _ => return None,
    };
    Some(func)
}

fn select_input_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "p" => Func::wrap(ctx, input::read_pad),
        "b" => Func::wrap(ctx, input::read_buttons),
        _ => return None,
    };
    Some(func)
}

fn select_net_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "l" => Func::wrap(ctx, net::load_stash),
        "m" => Func::wrap(ctx, net::get_me),
        "p" => Func::wrap(ctx, net::get_peers),
        "s" => Func::wrap(ctx, net::save_stash),
        _ => return None,
    };
    Some(func)
}

fn select_stats_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "p" => Func::wrap(ctx, stats::add_progress),
        "s" => Func::wrap(ctx, stats::add_score),
        _ => return None,
    };
    Some(func)
}

fn select_misc_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "d" => Func::wrap(ctx, misc::log_debug),
        "e" => Func::wrap(ctx, misc::log_error),
        "n" => Func::wrap(ctx, misc::get_name),
        "q" => Func::wrap(ctx, misc::quit),
        "r" => Func::wrap(ctx, misc::get_random),
        "s" => Func::wrap(ctx, misc::set_seed),
        _ => return None,
    };
    Some(func)
}
