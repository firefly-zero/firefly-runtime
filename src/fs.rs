use crate::state::State;
use embedded_io::{Read, Write};
use firefly_device::Device;
use firefly_meta::validate_path_part;

type C<'a> = wasmi::Caller<'a, State>;

/// Get file size in bytes for a file in the app ROM.
///
/// It is used by the apps to allocate the buffer for loading the file.
pub(crate) fn get_rom_file_size(caller: C, path_ptr: u32, path_len: u32) -> u32 {
    get_file_size_inner(caller, "roms", path_ptr, path_len)
}

pub(crate) fn get_file_size(caller: C, path_ptr: u32, path_len: u32) -> u32 {
    get_file_size_inner(caller, "data", path_ptr, path_len)
}

pub fn get_file_size_inner(mut caller: C, dir: &str, path_ptr: u32, path_len: u32) -> u32 {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.device.log_error("fs", "memory not found");
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return 0;
    };
    let path = &[dir, &state.author_id, &state.app_id, name];
    state.device.get_file_size(path).unwrap_or(0)
}

/// Read contents of the file from the app ROM and write them into the buffer.
pub(crate) fn load_rom_file(
    caller: C,
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    load_file_inner(caller, "roms", path_ptr, path_len, buf_ptr, buf_len)
}

/// Read contents of the file from the app data dir and write them into the buffer.
pub(crate) fn load_file(
    caller: C,
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    load_file_inner(caller, "data", path_ptr, path_len, buf_ptr, buf_len)
}

pub fn load_file_inner(
    mut caller: C,
    dir: &str,
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.device.log_error("fs", "memory not found");
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return 0;
    };

    let path = &[dir, &state.author_id, &state.app_id, name];
    let Some(mut file) = state.device.open_file(path) else {
        let msg = "cannot open file";
        state.device.log_error("fs", msg);
        return 0;
    };
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        let msg = "buffer for file points out of memory";
        state.device.log_error("fs", msg);
        return 0;
    };
    let Ok(file_size) = file.read(buf) else {
        let msg = "cannot read file";
        state.device.log_error("fs", msg);
        return 0;
    };
    if file_size != buf_len {
        let msg = "buffer size for file does not match the file size";
        state.device.log_error("fs", msg);
        return 0;
    }
    file_size as u32
}

/// Create file in data dir and write into it the contents of the buffer.
pub(crate) fn dump_file(
    mut caller: C,
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.device.log_error("fs", "memory not found");
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return 0;
    };

    let path = &["data", &state.author_id, &state.app_id, name];
    let Some(mut file) = state.device.create_file(path) else {
        let msg = "cannot create file";
        state.device.log_error("fs.dump_file", msg);
        return 0;
    };
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        let msg = "buffer for file points out of memory";
        state.device.log_error("fs.dump_file", msg);
        return 0;
    };
    let Ok(file_size) = file.write(buf) else {
        let msg = "cannot read file";
        state.device.log_error("fs.dump_file", msg);
        return 0;
    };
    if file.flush().is_err() {
        let msg = "cannot flush file";
        state.device.log_error("fs.dump_file", msg);
        return 0;
    }
    if file_size != buf_len {
        let msg = "not the whole buffer could be written into file";
        state.device.log_error("fs.dump_file", msg);
        return 0;
    }
    file_size as u32
}

pub(crate) fn remove_file(mut caller: C, path_ptr: u32, path_len: u32) {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.device.log_error("fs", "memory not found");
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return;
    };

    let path = &["data", &state.author_id, &state.app_id, name];
    let ok = state.device.remove_file(path);
    if !ok {
        let msg = "cannot remove file";
        state.device.log_error("fs.remove_file", msg);
    };
}

/// Load, parse, and validate the file name
fn get_file_name<'a>(
    state: &State,
    data: &'a [u8],
    path_ptr: u32,
    path_len: u32,
) -> Option<&'a str> {
    let path_ptr = path_ptr as usize;
    let path_len = path_len as usize;
    let Some(name_bytes) = &data.get(path_ptr..(path_ptr + path_len)) else {
        let msg = "file name points out of memory";
        state.device.log_error("fs", msg);
        return None;
    };
    let Ok(name) = core::str::from_utf8(name_bytes) else {
        let msg = "file name is not valid UTF-8";
        state.device.log_error("fs", msg);
        return None;
    };
    if validate_path_part(name).is_err() {
        let msg = "file name is not allowed";
        state.device.log_error("fs", msg);
        return None;
    }
    Some(name)
}
