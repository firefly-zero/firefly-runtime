use crate::error::HostError;
use crate::state::State;
use crate::utils::{read_into, write_all};
use crate::NetHandler;
use alloc::boxed::Box;
use embedded_io::Write;
use firefly_hal::{Device, Dir};
use firefly_types::validate_path_part;

type C<'a, 'b> = wasmi::Caller<'a, Box<State<'b>>>;

/// DEPRECATED
pub(crate) fn get_rom_file_size(caller: C, path_ptr: u32, path_len: u32) -> u32 {
    // let state = caller.data_mut();
    // state.log_error("get_rom_file_size is deprecated");
    get_file_size(caller, path_ptr, path_len)
}

/// Get file size in bytes for a file.
///
/// It is used by the apps to allocate the buffer for loading the file.
///
/// It will first lookup file in the app's ROM directory and then check
/// the app writable data directory.
pub(crate) fn get_file_size(mut caller: C, path_ptr: u32, path_len: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "fs.get_file_size";
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return 0;
    };
    if let Ok(size) = state.rom_dir.get_file_size(name) {
        return size;
    }

    let dir_path = &["data", state.id.author(), state.id.app(), "etc"];
    let mut dir = match state.device.open_dir(dir_path) {
        Ok(dir) => dir,
        Err(err) => {
            state.log_error(err);
            return 0;
        }
    };
    match dir.get_file_size(name) {
        Ok(size) => size,
        Err(err) => {
            state.log_error(err);
            0
        }
    }
}

/// DEPRECATED
pub(crate) fn load_rom_file(
    caller: C,
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    // let state = caller.data_mut();
    // state.log_error("get_rom_file_size is deprecated");
    load_file(caller, path_ptr, path_len, buf_ptr, buf_len)
}

/// Read contents of the file and write them into the buffer.
///
/// It will first lookup file in the app's ROM directory and then check
/// the app writable data directory.
pub(crate) fn load_file(
    mut caller: C,
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    let state = caller.data_mut();
    state.called = "fs.load_file";
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return 0;
    };

    let file = match state.rom_dir.open_file(name) {
        Ok(file) => file,
        Err(err) => {
            let dir_path = &["data", state.id.author(), state.id.app(), "etc"];
            let mut dir = match state.device.open_dir(dir_path) {
                Ok(dir) => dir,
                Err(err) => {
                    state.log_error(err);
                    return 0;
                }
            };
            let Ok(file) = dir.open_file(name) else {
                state.log_error(err);
                return 0;
            };
            let handler = state.net_handler.get_mut();
            if matches!(handler, NetHandler::FrameSyncer(_)) {
                state.log_error(HostError::DataFileInNet);
                return 0;
            }
            file
        }
    };
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        state.log_error(HostError::OomPointer);
        return 0;
    };
    let file_size = match read_into(file, buf) {
        Ok(file_size) => file_size,
        Err(err) => {
            state.log_error(HostError::FileRead(err.into()));
            return 0;
        }
    };
    if file_size != buf_len {
        state.log_error(HostError::BufferSize);
        return 0;
    }
    file_size as u32
}

/// Create file in data dir and write into it the contents of the buffer.
///
/// Return how many bytes were written.
pub(crate) fn dump_file(
    mut caller: C,
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> u32 {
    let state = caller.data_mut();
    state.called = "fs.dump_file";
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return 0;
    };

    // reject writing into files that are already present in ROM to avoid shadowing
    if state.rom_dir.get_file_size(name).is_ok() {
        state.log_error(HostError::FileReadOnly);
        return 0;
    }

    let dir_path = &["data", state.id.author(), state.id.app(), "etc"];
    let mut dir = match state.device.open_dir(dir_path) {
        Ok(dir) => dir,
        Err(err) => {
            state.log_error(err);
            return 0;
        }
    };
    let mut file = match dir.create_file(name) {
        Ok(file) => file,
        Err(err) => {
            state.log_error(err);
            return 0;
        }
    };
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        state.log_error(HostError::OomPointer);
        return 0;
    };
    let Ok(file_size) = write_all(&mut file, buf) else {
        state.log_error(HostError::FileWrite);
        return 0;
    };
    if file.flush().is_err() {
        state.log_error(HostError::FileFlush);
        return 0;
    }
    if file_size != buf_len {
        state.log_error(HostError::BufferSize);
        return 0;
    }
    file_size as u32
}

pub(crate) fn remove_file(mut caller: C, path_ptr: u32, path_len: u32) {
    let state = caller.data_mut();
    state.called = "fs.remove_file";
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return;
    };

    // reject removing files that are already present in ROM to avoid shadowing
    if state.rom_dir.get_file_size(name).is_ok() {
        state.log_error(HostError::FileReadOnly);
        return;
    }

    let dir_path = &["data", state.id.author(), state.id.app(), "etc"];
    let mut dir = match state.device.open_dir(dir_path) {
        Ok(dir) => dir,
        Err(err) => {
            state.log_error(err);
            return;
        }
    };
    if let Err(err) = dir.remove_file(name) {
        state.log_error(err);
    };
}

/// Load, parse, and validate the file name
pub(super) fn get_file_name<'a>(
    state: &State,
    data: &'a [u8],
    path_ptr: u32,
    path_len: u32,
) -> Option<&'a str> {
    let path_ptr = path_ptr as usize;
    let path_len = path_len as usize;
    let Some(name_bytes) = &data.get(path_ptr..(path_ptr + path_len)) else {
        state.log_error(HostError::OomPointer);
        return None;
    };
    let Ok(name) = core::str::from_utf8(name_bytes) else {
        state.log_error(HostError::FileNameUtf8);
        return None;
    };
    if let Err(err) = validate_path_part(name) {
        state.log_error(HostError::FileName(err));
        return None;
    }
    Some(name)
}
