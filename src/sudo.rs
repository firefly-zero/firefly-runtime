use crate::fs::get_file_name;
use crate::state::State;
use alloc::string::ToString;
use firefly_device::Device;
use heapless::Vec;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn iter_dirs_buf_size(caller: C, path_ptr: u32, path_len: u32) -> u32 {
    let state = caller.data();
    let Some(memory) = state.memory else {
        state.device.log_error("fs", "memory not found");
        return 0;
    };
    let data = memory.data(&caller);
    let Some(name) = get_file_name(state, data, path_ptr, path_len) else {
        return 0;
    };
    let path: Vec<&str, 4> = name.split('/').collect();
    let mut size = 0;
    state.device.iter_dir(&path, |_kind, entry_name| {
        size += entry_name.len() + 1;
    });
    size as u32
}

pub(crate) fn iter_dirs(
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
    // reallocate the file path to remove reference to data
    let name = name.to_string();
    let path: Vec<&str, 4> = name.split('/').collect();

    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        let msg = "buffer points out of memory";
        state.device.log_error("fs", msg);
        return 0;
    };
    let mut pos = 0;
    state.device.iter_dir(&path, |_kind, entry_name| {
        buf[pos] = entry_name.len() as u8;
        buf[(pos + 1)..].copy_from_slice(entry_name);
        pos += entry_name.len() + 1;
    });
    pos as u32
}
