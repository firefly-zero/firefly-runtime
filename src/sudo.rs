use crate::state::State;
use firefly_device::Device;
use firefly_meta::{validate_id, validate_path_part};
use heapless::Vec;

type C<'a> = wasmi::Caller<'a, State>;

/// How many parts at most a file path can have
///
/// The current value is 4, assuming the flat and consistent structure:
///
/// `{data,roms,sys}/AUTHOR_ID/APP_ID/FILE_NAME`
const MAX_DEPTH: usize = 4;

pub(crate) fn iter_dirs_buf_size(caller: C, path_ptr: u32, path_len: u32) -> u32 {
    let state = caller.data();
    let Some(memory) = state.memory else {
        state.device.log_error("sudo", "memory not found");
        return 0;
    };
    let data = memory.data(&caller);
    let path_ptr = path_ptr as usize;
    let path_len = path_len as usize;
    let Some(path_bytes) = data.get(path_ptr..(path_ptr + path_len)) else {
        let msg = "file path points out of memory";
        state.device.log_error("sudo", msg);
        return 0;
    };
    let Ok(path) = core::str::from_utf8(path_bytes) else {
        let msg = "file path is not valid UTF-8";
        state.device.log_error("sudo", msg);
        return 0;
    };
    let path: Vec<&str, MAX_DEPTH> = path.split('/').collect();
    for part in &path {
        if validate_path_part(part).is_err() {
            let msg = "file path is not allowed";
            state.device.log_error("sudo", msg);
            return 0;
        }
    }

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
        state.device.log_error("sudo", "memory not found");
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some((path_bytes, buf)) = get_safe_subsclices(data, path_ptr, path_len, buf_ptr, buf_len)
    else {
        let msg = "invalid pointer for path or buffer";
        state.device.log_error("sudo.iter_dirs", msg);
        return 0;
    };

    // parse and validate the dir path.
    let Ok(path) = core::str::from_utf8(path_bytes) else {
        let msg = "file path is not valid UTF-8";
        state.device.log_error("sudo", msg);
        return 0;
    };
    let path: Vec<&str, MAX_DEPTH> = path.split('/').collect();
    for part in &path {
        if validate_path_part(part).is_err() {
            let msg = "file path is not allowed";
            state.device.log_error("sudo", msg);
            return 0;
        }
    }

    let mut pos = 0;
    state.device.iter_dir(&path, |_kind, entry_name| {
        buf[pos] = entry_name.len() as u8;
        buf[(pos + 1)..].copy_from_slice(entry_name);
        pos += entry_name.len() + 1;
    });
    pos as u32
}

/// Stop the current app and run the given one instead.
pub(crate) fn run_app(mut caller: C, author_ptr: u32, author_len: u32, app_ptr: u32, app_len: u32) {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.device.log_error("sudo", "memory not found");
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);

    let author_ptr = author_ptr as usize;
    let author_len = author_len as usize;
    let Some(author_bytes) = data.get(author_ptr..(author_ptr + author_len)) else {
        let msg = "invalid pointer for author_id";
        state.device.log_error("sudo.run_app", msg);
        return;
    };
    let Ok(author_id) = core::str::from_utf8(author_bytes) else {
        let msg = "author_id is not valid UTF-8";
        state.device.log_error("sudo.run_app", msg);
        return;
    };
    if validate_id(author_id).is_err() {
        let msg = "author_id is not allowed";
        state.device.log_error("sudo.run_app", msg);
        return;
    }

    let app_ptr = app_ptr as usize;
    let app_len = app_len as usize;
    let Some(app_bytes) = data.get(app_ptr..(app_ptr + app_len)) else {
        let msg = "invalid pointer for app_id";
        state.device.log_error("sudo.run_app", msg);
        return;
    };
    let Ok(app_id) = core::str::from_utf8(app_bytes) else {
        let msg = "app_id is not valid UTF-8";
        state.device.log_error("sudo.run_app", msg);
        return;
    };
    if validate_id(app_id).is_err() {
        let msg = "app_id is not allowed";
        state.device.log_error("sudo.run_app", msg);
        return;
    }

    todo!()
}

/// Get 2 subslices (one of which is mutable) from a slice.
///
/// The rust borrow checker requires that there is only one mutable reference
/// to a slice or any number of immitable references. If you need to mutate
/// a slice, you cannot have other references to it.
///
/// This function returns a mutable and immutable reference to separate regions
/// of the given slice. It is safe because the function ensures that the regions
/// don't intersect.
fn get_safe_subsclices(
    data: &mut [u8],
    path_ptr: u32,
    path_len: u32,
    buf_ptr: u32,
    buf_len: u32,
) -> Option<(&[u8], &mut [u8])> {
    let path_ptr = path_ptr as usize;
    let path_len = path_len as usize;
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    if buf_ptr >= data.len() || path_ptr >= data.len() {
        return None;
    }
    if path_ptr < buf_ptr {
        let (left, right) = data.split_at_mut(buf_ptr);
        let path = left.get(path_ptr..(path_ptr + path_len))?;
        let buf = right.get_mut(0..buf_len)?;
        Some((path, buf))
    } else {
        let (left, right) = data.split_at_mut(path_ptr);
        let buf = left.get_mut(buf_ptr..(buf_ptr + buf_len))?;
        let path = right.get(0..path_len)?;
        Some((path, buf))
    }
}
