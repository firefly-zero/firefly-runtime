use crate::error::HostError;
use crate::state::State;
use firefly_device::Device;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn environ_get(_caller: C, _environ: i32, _environ_buf: i32) -> i32 {
    0
}

pub(crate) fn environ_sizes_get(mut caller: C, offset0: i32, offset1: i32) -> i32 {
    let state = caller.data();
    let Some(memory) = state.memory else {
        state.device.log_error("fs", HostError::MemoryNotFound);
        return 1;
    };
    let data = memory.data_mut(&mut caller);
    let offset0 = offset0 as usize;
    let offset1 = offset1 as usize;
    data[offset0] = 0;
    data[offset0 + 1] = 0;
    data[offset0 + 2] = 0;
    data[offset0 + 3] = 0;
    data[offset1] = 0;
    data[offset1 + 1] = 0;
    data[offset1 + 2] = 0;
    data[offset1 + 3] = 0;
    0
}

pub(crate) fn clock_time_get(_caller: C, _id: i32, _precision: i64, _offset0: i32) -> i32 {
    0
}
pub(crate) fn fd_close(_caller: C, _fd: i32) -> i32 {
    todo!()
}

pub(crate) fn fd_read(
    _caller: C,
    _fd: i32,
    _iov_buf: i32,
    _iov_buf_len: i32,
    _offset1: i32,
) -> i32 {
    todo!()
}

pub(crate) fn fd_seek(_caller: C, _fd: i32, _offset: i64, _whence: i32, _offset0: i32) -> i32 {
    todo!()
}

pub(crate) fn fd_write(_fd: i32, _ciov_buf: i32, _ciov_buf_len: i32, _offset0: i32) -> i32 {
    todo!()
}

pub(crate) fn proc_exit(mut caller: C, _rval: i32) {
    let state = caller.data_mut();
    state.exit = true;
    // TODO: Apps expect that the guest code will stop execution after calling proc_exit.
    // Clang inserts "unreachable" right after that. Can we signal from here to wasmi
    // to stop execution?
}
