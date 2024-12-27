use crate::error::HostError;
use crate::state::State;
use alloc::string::ToString;

type C<'a, 'b> = wasmi::Caller<'a, State<'b>>;


pub(crate) fn add_menu_item(mut caller: C, index: u32, name_ptr: u32, name_len: u32) {
    let state = caller.data_mut();
    state.called = "menu.add_menu_item";
    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let name_ptr = name_ptr as usize;
    let name_len = name_len as usize;
    let Some(name_bytes) = &data.get(name_ptr..(name_ptr + name_len)) else {
        state.log_error(HostError::OomPointer);
        return;
    };
    let Ok(name) = core::str::from_utf8(name_bytes) else {
        state.log_error(HostError::MenuItemUtf8);
        return;
    };

    state.menu.add(index as u8, name.to_string())
}

pub(crate) fn remove_menu_item(mut caller: C, index: u32) {
    let state = caller.data_mut();
    state.called = "menu.remove_menu_item";
    state.menu.remove(index as u8);
}

pub(crate) fn open_menu(mut caller: C) {
    let state = caller.data_mut();
    state.called = "menu.open_menu";
    state.menu.activate();
}
