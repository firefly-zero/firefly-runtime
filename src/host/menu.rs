use crate::{error::HostError, state::State};
use alloc::string::ToString;
use firefly_device::Device;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn add_menu_item(mut caller: C, index: u32, name_ptr: u32, name_len: u32) {
    if index > 4 {
        return;
    }

    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state
            .device
            .log_error("menu.add", HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let name_ptr = name_ptr as usize;
    let name_len = name_len as usize;
    let Some(name_bytes) = &data.get(name_ptr..(name_ptr + name_len)) else {
        state.device.log_error("menu.add", HostError::OomPointer);
        return;
    };
    let Ok(name) = core::str::from_utf8(name_bytes) else {
        state.device.log_error("menu.add", HostError::MenuItemUtf8);
        return;
    };

    state.menu.add(index as u8, name.to_string())
}

pub(crate) fn remove_menu_item(mut caller: C, index: u32) {
    let state = caller.data_mut();
    if index > 4 {
        return;
    }
    state.menu.remove(index as u8);
}

pub(crate) fn open_menu(mut caller: C) {
    let state = caller.data_mut();
    state.menu.activate();
}
