use crate::state::State;
use embedded_io::Read;
use firefly_device::Device;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn load_rom_file(
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

    let path_ptr = path_ptr as usize;
    let path_len = path_len as usize;
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;

    let Some(name_bytes) = &data.get(path_ptr..(path_ptr + path_len)) else {
        return 0;
    };
    let Ok(name) = core::str::from_utf8(name_bytes) else {
        return 0;
    };
    let path = &["roms", "a", "b", name];
    let Some(mut file) = state.device.open_file(path) else {
        return 0;
    };
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        return 0;
    };
    let Ok(file_size) = file.read(buf) else {
        return 0;
    };
    file_size as u32
}
