use crate::state::State;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn args_get(_caller: C, _argv: i32, _argv_buf: i32) -> i32 {
    0
}

pub(crate) fn args_sizes_get(_caller: C, _offset0: i32, _offset1: i32) -> i32 {
    0
}

pub(crate) fn environ_get(_caller: C, _environ: i32, _environ_buf: i32) -> i32 {
    0
}

pub(crate) fn environ_sizes_get(_caller: C, _offset0: i32, _offset1: i32) -> i32 {
    0
}

pub(crate) fn clock_res_get(_caller: C, _id: i32, _offset0: i32) -> i32 {
    0
}

pub(crate) fn clock_time_get(_caller: C, _id: i32, _precision: i64, _offset0: i32) -> i32 {
    0
}

pub(crate) fn fd_advise(_caller: C, _fd: i32, _offset: i64, _len: i64, _advice: i32) -> i32 {
    0
}

pub(crate) fn fd_allocate(_caller: C, _fd: i32, _offset: i64, _len: i64) -> i32 {
    0
}

pub(crate) fn fd_close(_caller: C, _fd: i32) -> i32 {
    0
}

pub(crate) fn fd_datasync(_caller: C, _fd: i32) -> i32 {
    0
}

pub(crate) fn fd_fdstat_get(_caller: C, _fd: i32, _offset0: i32) -> i32 {
    0
}

pub(crate) fn fd_fdstat_set_flags(_caller: C, _fd: i32, _flags: i32) -> i32 {
    0
}

pub(crate) fn fd_fdstat_set_rights(
    _caller: C,
    _fd: i32,
    _fs_rights_base: i64,
    _fs_rights_inheriting: i64,
) -> i32 {
    0
}

pub(crate) fn fd_filestat_get(_caller: C, _fd: i32, _offset0: i32) -> i32 {
    0
}

pub(crate) fn fd_filestat_set_size(_caller: C, _fd: i32, _size: i64) -> i32 {
    0
}

pub(crate) fn fd_filestat_set_times(
    _caller: C,
    _fd: i32,
    _atim: i64,
    _mtim: i64,
    _fst_flags: i32,
) -> i32 {
    0
}

pub(crate) fn fd_pread(
    _caller: C,
    _fd: i32,
    _iov_buf: i32,
    _iov_buf_len: i32,
    _offset: i64,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn fd_prestat_get(_caller: C, _fd: i32, _offset0: i32) -> i32 {
    0
}

pub(crate) fn fd_prestat_dir_name(_caller: C, _fd: i32, _path: i32, _path_len: i32) -> i32 {
    0
}

pub(crate) fn fd_pwrite(
    _caller: C,
    _fd: i32,
    _ciov_buf: i32,
    _ciov_buf_len: i32,
    _offset: i64,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn fd_read(
    _caller: C,
    _fd: i32,
    _iov_buf: i32,
    _iov_buf_len: i32,
    _offset1: i32,
) -> i32 {
    0
}

pub(crate) fn fd_readdir(
    _caller: C,
    _fd: i32,
    _buf: i32,
    _buf_len: i32,
    _cookie: i64,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn fd_renumber(_caller: C, _fd: i32, _to: i32) -> i32 {
    0
}

pub(crate) fn fd_seek(_caller: C, _fd: i32, _offset: i64, _whence: i32, _offset0: i32) -> i32 {
    0
}

pub(crate) fn fd_sync(_caller: C, _fd: i32) -> i32 {
    0
}

pub(crate) fn fd_tell(_caller: C, _fd: i32, _offset0: i32) -> i32 {
    0
}

pub(crate) fn fd_write(
    _caller: C,
    _fd: i32,
    _ciov_buf: i32,
    _ciov_buf_len: i32,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn path_create_directory(_caller: C, _fd: i32, _offset: i32, _length: i32) -> i32 {
    0
}

pub(crate) fn path_filestat_get(
    _caller: C,
    _fd: i32,
    _flags: i32,
    _offset: i32,
    _length: i32,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn path_filestat_set_times(
    _caller: C,
    _fd: i32,
    _flags: i32,
    _offset: i32,
    _length: i32,
    _atim: i64,
    _mtim: i64,
    _fst_flags: i32,
) -> i32 {
    0
}

pub(crate) fn path_link(
    _caller: C,
    _old_fd: i32,
    _old_flags: i32,
    _old_offset: i32,
    _old_length: i32,
    _new_fd: i32,
    _new_offset: i32,
    _new_length: i32,
) -> i32 {
    0
}

pub(crate) fn path_open(
    _caller: C,
    _fd: i32,
    _dirflags: i32,
    _offset: i32,
    _length: i32,
    _oflags: i32,
    _fs_rights_base: i64,
    _fdflags: i64,
    _fs_rights_inheriting: i32,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn path_readlink(
    _caller: C,
    _fd: i32,
    _offset: i32,
    _length: i32,
    _buf: i32,
    _buf_len: i32,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn path_remove_directory(_caller: C, _fd: i32, _offset: i32, _length: i32) -> i32 {
    0
}

pub(crate) fn path_rename(
    _caller: C,
    _fd: i32,
    _old_offset: i32,
    _old_length: i32,
    _new_fd: i32,
    _new_offset: i32,
    _new_length: i32,
) -> i32 {
    0
}

pub(crate) fn path_symlink(
    _caller: C,
    _old_offset: i32,
    _old_length: i32,
    _fd: i32,
    _new_offset: i32,
    _new_length: i32,
) -> i32 {
    0
}

pub(crate) fn path_unlink_file(_caller: C, _fd: i32, _offset: i32, _length: i32) -> i32 {
    0
}

pub(crate) fn poll_oneoff(
    _caller: C,
    _in_: i32,
    _out: i32,
    _nsubscriptions: i32,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn proc_exit(_caller: C, _rval: i32) {}

pub(crate) fn proc_raise(_caller: C, _sig: i32) -> i32 {
    0
}

pub(crate) fn sched_yield(_caller: C) -> i32 {
    0
}

pub(crate) fn random_get(_caller: C, _buf: i32, _buf_len: i32) -> i32 {
    0
}

pub(crate) fn sock_accept(_caller: C, _fd: i32, _flags: i32, _offset0: i32) -> i32 {
    0
}

pub(crate) fn sock_recv(
    _caller: C,
    _fd: i32,
    _iov_buf: i32,
    _iov_buf_len: i32,
    _ri_flags: i32,
    _offset0: i32,
    _offset1: i32,
) -> i32 {
    0
}

pub(crate) fn sock_send(
    _caller: C,
    _fd: i32,
    _ciov_buf: i32,
    _ciov_buf_len: i32,
    _si_flags: i32,
    _offset0: i32,
) -> i32 {
    0
}

pub(crate) fn sock_shutdown(_caller: C, _fd: i32, _how: i32) -> i32 {
    0
}
