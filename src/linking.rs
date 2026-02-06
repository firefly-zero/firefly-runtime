use crate::host::*;
use crate::state::State;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

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
            _ => {
                return Err(LinkingError::UnknownHostFunction(
                    module_name.to_string(),
                    fn_name.to_string(),
                ));
            }
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
        "clear_screen" => host_func(ctx, graphics::clear_screen),
        "set_color" => host_func(ctx, graphics::set_color),
        "draw_point" => host_func(ctx, graphics::draw_point),
        "draw_line" => host_func(ctx, graphics::draw_line),
        "draw_rect" => host_func(ctx, graphics::draw_rect),
        "draw_rounded_rect" => host_func(ctx, graphics::draw_rounded_rect),
        "draw_circle" => host_func(ctx, graphics::draw_circle),
        "draw_ellipse" => host_func(ctx, graphics::draw_ellipse),
        "draw_triangle" => host_func(ctx, graphics::draw_triangle),
        "draw_arc" => host_func(ctx, graphics::draw_arc),
        "draw_sector" => host_func(ctx, graphics::draw_sector),
        "draw_qr" => host_func(ctx, graphics::draw_qr),
        "draw_text" => host_func(ctx, graphics::draw_text),
        "draw_image" => host_func(ctx, graphics::draw_image),
        "draw_sub_image" => host_func(ctx, graphics::draw_sub_image),
        "set_canvas" => host_func(ctx, graphics::set_canvas),
        "unset_canvas" => host_func(ctx, graphics::unset_canvas),
        _ => return None,
    };
    Some(func)
}

fn select_audio_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "reset" => host_func(ctx, audio::reset),
        "reset_all" => host_func(ctx, audio::reset_all),
        "clear" => host_func(ctx, audio::clear),
        "add_empty" => host_func(ctx, audio::add_empty),
        "add_file" => host_func(ctx, audio::add_file),
        "add_mix" => host_func(ctx, audio::add_mix),
        "add_all_for_one" => host_func(ctx, audio::add_all_for_one),
        "add_gain" => host_func(ctx, audio::add_gain),
        "add_loop" => host_func(ctx, audio::add_loop),
        "add_concat" => host_func(ctx, audio::add_concat),
        "add_pan" => host_func(ctx, audio::add_pan),
        "add_mute" => host_func(ctx, audio::add_mute),
        "add_pause" => host_func(ctx, audio::add_pause),
        "add_track_position" => host_func(ctx, audio::add_track_position),
        "add_low_pass" => host_func(ctx, audio::add_low_pass),
        "add_high_pass" => host_func(ctx, audio::add_high_pass),
        "add_take_left" => host_func(ctx, audio::add_take_left),
        "add_take_right" => host_func(ctx, audio::add_take_right),
        "add_swap" => host_func(ctx, audio::add_swap),
        "add_clip" => host_func(ctx, audio::add_clip),
        "add_noise" => host_func(ctx, audio::add_noise),
        "add_sine" => host_func(ctx, audio::add_sine),
        "add_square" => host_func(ctx, audio::add_square),
        "add_sawtooth" => host_func(ctx, audio::add_sawtooth),
        "add_triangle" => host_func(ctx, audio::add_triangle),
        "add_zero" => host_func(ctx, audio::add_zero),
        "mod_linear" => host_func(ctx, audio::mod_linear),
        "mod_hold" => host_func(ctx, audio::mod_hold),
        "mod_sine" => host_func(ctx, audio::mod_sine),
        _ => return None,
    };
    Some(func)
}

fn select_input_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "read_pad" => host_func(ctx, input::read_pad),
        "read_buttons" => host_func(ctx, input::read_buttons),
        _ => return None,
    };
    Some(func)
}

fn select_menu_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "add_menu_item" => host_func(ctx, menu::add_menu_item),
        "remove_menu_item" => host_func(ctx, menu::remove_menu_item),
        "open_menu" => host_func(ctx, menu::open_menu),
        _ => return None,
    };
    Some(func)
}

fn select_fs_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "get_rom_file_size" => host_func(ctx, fs::get_rom_file_size),
        "load_rom_file" => host_func(ctx, fs::load_rom_file),
        "get_file_size" => host_func(ctx, fs::get_file_size),
        "load_file" => host_func(ctx, fs::load_file),
        "dump_file" => host_func(ctx, fs::dump_file),
        "remove_file" => host_func(ctx, fs::remove_file),
        _ => return None,
    };
    Some(func)
}

fn select_net_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "get_me" => host_func(ctx, net::get_me),
        "get_peers" => host_func(ctx, net::get_peers),
        "save_stash" => host_func(ctx, net::save_stash),
        "load_stash" => host_func(ctx, net::load_stash),
        _ => return None,
    };
    Some(func)
}

fn select_stats_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "add_progress" => host_func(ctx, stats::add_progress),
        "add_score" => host_func(ctx, stats::add_score),
        _ => return None,
    };
    Some(func)
}

fn select_misc_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "log_debug" => host_func(ctx, misc::log_debug),
        "log_error" => host_func(ctx, misc::log_error),
        "set_seed" => host_func(ctx, misc::set_seed),
        "get_random" => host_func(ctx, misc::get_random),
        "get_name" => host_func(ctx, misc::get_name),
        "restart" => host_func(ctx, misc::restart),
        "set_conn_status" => host_func(ctx, misc::set_conn_status),
        "quit" => host_func(ctx, misc::quit),
        _ => return None,
    };
    Some(func)
}

fn select_sudo_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "list_dirs" => host_func(ctx, sudo::list_dirs),
        "list_dirs_buf_size" => host_func(ctx, sudo::list_dirs_buf_size),
        "list_files" => host_func(ctx, sudo::list_files),
        "list_files_buf_size" => host_func(ctx, sudo::list_files_buf_size),
        "get_file_size" => host_func(ctx, sudo::get_file_size),
        "load_file" => host_func(ctx, sudo::load_file),
        "run_app" => host_func(ctx, sudo::run_app),
        _ => return None,
    };
    Some(func)
}

fn select_wasip1_external<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "environ_get" => host_func(ctx, wasip1::environ_get),
        "environ_sizes_get" => host_func(ctx, wasip1::environ_sizes_get),
        "clock_time_get" => host_func(ctx, wasip1::clock_time_get),
        "fd_close" => host_func(ctx, wasip1::fd_close),
        "fd_read" => host_func(ctx, wasip1::fd_read),
        "fd_seek" => host_func(ctx, wasip1::fd_seek),
        "fd_write" => host_func(ctx, wasip1::fd_write),
        "proc_exit" => host_func(ctx, wasip1::proc_exit),
        _ => return None,
    };
    Some(func)
}

fn select_graphics_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "a" => host_func(ctx, graphics::draw_arc),
        "c" => host_func(ctx, graphics::draw_circle),
        "ca" => host_func(ctx, graphics::set_canvas),
        "cr" => host_func(ctx, graphics::unset_canvas),
        "cs" => host_func(ctx, graphics::clear_screen),
        "e" => host_func(ctx, graphics::draw_ellipse),
        "i" => host_func(ctx, graphics::draw_image),
        "l" => host_func(ctx, graphics::draw_line),
        "p" => host_func(ctx, graphics::draw_point),
        "r" => host_func(ctx, graphics::draw_rect),
        "rr" => host_func(ctx, graphics::draw_rounded_rect),
        "s" => host_func(ctx, graphics::draw_sector),
        "sc" => host_func(ctx, graphics::set_color),
        "si" => host_func(ctx, graphics::draw_sub_image),
        "t" => host_func(ctx, graphics::draw_triangle),
        "x" => host_func(ctx, graphics::draw_text),
        "q" => host_func(ctx, graphics::draw_qr),
        _ => return None,
    };
    Some(func)
}

fn select_input_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "p" => host_func(ctx, input::read_pad),
        "b" => host_func(ctx, input::read_buttons),
        _ => return None,
    };
    Some(func)
}

fn select_net_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "l" => host_func(ctx, net::load_stash),
        "m" => host_func(ctx, net::get_me),
        "p" => host_func(ctx, net::get_peers),
        "s" => host_func(ctx, net::save_stash),
        _ => return None,
    };
    Some(func)
}

fn select_stats_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "p" => host_func(ctx, stats::add_progress),
        "s" => host_func(ctx, stats::add_score),
        _ => return None,
    };
    Some(func)
}

fn select_misc_external_alias<'a>(
    ctx: impl wasmi::AsContextMut<Data = Box<State<'a>>>,
    fn_name: &str,
) -> Option<wasmi::Func> {
    let func = match fn_name {
        "d" => host_func(ctx, misc::log_debug),
        "e" => host_func(ctx, misc::log_error),
        "n" => host_func(ctx, misc::get_name),
        "q" => host_func(ctx, misc::quit),
        "r" => host_func(ctx, misc::get_random),
        "s" => host_func(ctx, misc::set_seed),
        _ => return None,
    };
    Some(func)
}

/// Utility function to wrap host functions without Wasmi imports.
#[inline]
fn host_func<T, P, R>(
    ctx: impl wasmi::AsContextMut<Data = T>,
    func: impl wasmi::IntoFunc<T, P, R>,
) -> wasmi::Func {
    wasmi::Func::wrap(ctx, func)
}
