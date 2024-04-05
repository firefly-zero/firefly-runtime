use crate::state::State;
use firefly_device::Device;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn log_debug(mut caller: C, ptr: u32, len: u32) {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state.device.log_error("misc.log_debug", "memory not found");
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let ptr = ptr as usize;
    let len = len as usize;
    let Some(bytes) = &data.get(ptr..(ptr + len)) else {
        let msg = "text points outside of memory";
        state.device.log_error("misc.log_debug", msg);
        return;
    };
    let Ok(text) = core::str::from_utf8(bytes) else {
        let msg = "the given text is not valid UTF-8";
        state.device.log_error("misc.log_debug", msg);
        return;
    };
    state.device.log_debug("app", text);
}
