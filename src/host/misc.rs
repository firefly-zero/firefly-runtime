use crate::error::HostError;
use crate::state::State;
use firefly_device::Device;

type C<'a> = wasmi::Caller<'a, State>;

/// Write a debug log message into console.
pub(crate) fn log_debug(mut caller: C, ptr: u32, len: u32) {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state
            .device
            .log_error("misc.log_debug", HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let ptr = ptr as usize;
    let len = len as usize;
    let Some(bytes) = &data.get(ptr..(ptr + len)) else {
        state
            .device
            .log_error("misc.log_debug", HostError::OomPointer);
        return;
    };
    let Ok(text) = core::str::from_utf8(bytes) else {
        state
            .device
            .log_error("misc.log_debug", HostError::TextUtf8);
        return;
    };
    state.device.log_debug("app", text);
}

/// Write a error log message into console.
pub(crate) fn log_error(mut caller: C, ptr: u32, len: u32) {
    let state = caller.data_mut();
    let Some(memory) = state.memory else {
        state
            .device
            .log_error("misc.log_error", HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let ptr = ptr as usize;
    let len = len as usize;
    let Some(bytes) = &data.get(ptr..(ptr + len)) else {
        state
            .device
            .log_error("misc.log_error", HostError::OomPointer);
        return;
    };
    let Ok(text) = core::str::from_utf8(bytes) else {
        state
            .device
            .log_error("misc.log_error", HostError::TextUtf8);
        return;
    };
    state.device.log_error("app", text);
}

/// Set random numbers generator seed.
pub(crate) fn set_seed(mut caller: C, seed: u32) {
    let state = caller.data_mut();
    state.seed = seed;
}

/// Get a pseudo-random integer.
///
/// Uses [xorshift] algorithm. It's very fast, easy to implement,
/// and has a very long period. Wikipedia claims that it fails some
/// statistical tests, but that still should be good enough for games.
///
/// [xorshift]: https://en.wikipedia.org/wiki/Xorshift
pub(crate) fn get_random(mut caller: C) -> u32 {
    let state = caller.data_mut();
    let mut x = state.seed;
    if x == 0 {
        x = 1;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    state.seed = x;
    x
}

pub(crate) fn quit(mut caller: C) {
    let state = caller.data_mut();
    state.exit = true;
}
