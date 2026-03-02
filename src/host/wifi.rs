use crate::{error::HostError, state::State};
use alloc::boxed::Box;
use firefly_hal::{Device, Wifi};

type C<'a, 'b> = wasmi::Caller<'a, Box<State<'b>>>;

pub(crate) fn scan(mut caller: C, ptr: u32, len: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "wifi.scan";

    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return 1;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let ptr = ptr as usize;
    let len = len as usize;
    let Some(buf) = data.get_mut(ptr..(ptr + len)) else {
        state.log_error(HostError::OomPointer);
        return 0;
    };

    let mut wifi = state.device.wifi();
    let points = match wifi.scan() {
        Ok(points) => points,
        Err(err) => {
            state.log_error(err);
            return 0;
        }
    };

    let mut pos = 0;
    for point in points {
        buf[pos] = point.len() as u8;
        match buf.get_mut((pos + 1)..(pos + 1 + point.len())) {
            Some(buf) => {
                buf.copy_from_slice(point.as_bytes());
                pos += point.len() + 1;
            }
            None => {
                state.log_error("buffer is not big enough to fit all entries");
                return 0;
            }
        };
    }
    pos as u32
}

pub(crate) fn connect(mut caller: C, ssid_ptr: u32, ssid_len: u32, pass_ptr: u32, pass_len: u32) {
    let state = caller.data_mut();
    state.called = "wifi.connect";

    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let Some(ssid) = load_string(state, data, ssid_ptr, ssid_len) else {
        return;
    };
    let Some(pass) = load_string(state, data, pass_ptr, pass_len) else {
        return;
    };

    let mut wifi = state.device.wifi();
    let res = wifi.connect(ssid, pass);
    if let Err(err) = res {
        state.log_error(err);
    }
}

pub(crate) fn status(mut caller: C) -> u32 {
    let state = caller.data_mut();
    state.called = "wifi.status";

    let mut wifi = state.device.wifi();
    let res = wifi.status();
    match res {
        Ok(status) => u32::from(status),
        Err(err) => {
            state.log_error(err);
            0
        }
    }
}

pub(crate) fn disconnect(mut caller: C) {
    let state = caller.data_mut();
    state.called = "wifi.disconnect";

    let wifi = state.device.wifi();
    let res = wifi.disconnect();
    if let Err(err) = res {
        state.log_error(err);
    }
}

pub(crate) fn tcp_connect(mut caller: C, ip: u32, port: u32) {
    let state = caller.data_mut();
    state.called = "wifi.tcp_connect";

    let mut wifi = state.device.wifi();
    let res = wifi.tcp_connect(ip, port as u16);
    if let Err(err) = res {
        state.log_error(err);
    }
}

pub(crate) fn tcp_status(mut caller: C) -> u32 {
    let state = caller.data_mut();
    state.called = "wifi.tcp_status";

    let mut wifi = state.device.wifi();
    let res = wifi.tcp_status();
    match res {
        Ok(status) => u32::from(status),
        Err(err) => {
            state.log_error(err);
            0
        }
    }
}

pub(crate) fn tcp_send(mut caller: C, ptr: u32, len: u32) {
    let state = caller.data_mut();
    state.called = "wifi.tcp_send";

    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let ptr = ptr as usize;
    let len = len as usize;
    let Some(data) = &data.get(ptr..(ptr + len)) else {
        state.log_error(HostError::OomPointer);
        return;
    };

    let mut wifi = state.device.wifi();
    let res = wifi.tcp_send(data);
    if let Err(err) = res {
        state.log_error(err);
    }
}

pub(crate) fn tcp_close(mut caller: C) {
    let state = caller.data_mut();
    state.called = "wifi.tcp_close";

    let mut wifi = state.device.wifi();
    let res = wifi.tcp_close();
    if let Err(err) = res {
        state.log_error(err);
    }
}

pub(super) fn load_string<'a>(
    state: &State,
    data: &'a [u8],
    ptr: u32,
    len: u32,
) -> Option<&'a str> {
    let ptr = ptr as usize;
    let len = len as usize;
    let Some(text_bytes) = &data.get(ptr..(ptr + len)) else {
        state.log_error(HostError::OomPointer);
        return None;
    };
    let Ok(text) = core::str::from_utf8(text_bytes) else {
        state.log_error(HostError::TextUtf8);
        return None;
    };
    Some(text)
}
