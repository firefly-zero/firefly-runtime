use crate::{
    error::HostError,
    state::{NetHandler, State},
};

use super::stats::get_friend;

type C<'a, 'b> = wasmi::Caller<'a, State<'b>>;


/// Get the index of the local peer.
pub(crate) fn get_me(mut caller: C) -> u32 {
    let state = caller.data_mut();
    state.called = "net.get_me";
    let handler = state.net_handler.get_mut();
    let NetHandler::FrameSyncer(syncer) = handler else {
        return 0;
    };
    for (peer, i) in syncer.peers.iter().zip(0u32..) {
        if peer.addr.is_none() {
            return i;
        }
    }
    unreachable!("list of peers has no local device")
}

/// Get the map of peers that are currently online.
pub(crate) fn get_peers(mut caller: C) -> u32 {
    let state = caller.data_mut();
    state.called = "net.get_peers";
    let handler = state.net_handler.get_mut();
    let NetHandler::FrameSyncer(syncer) = handler else {
        return 1;
    };
    let mut res = 0u32;
    for _peer in &syncer.peers {
        res = res << 1 | 1;
    }
    res
}

pub(crate) fn save_stash(mut caller: C, peer_id: u32, buf_ptr: u32, buf_len: u32) {
    let state = caller.data_mut();
    state.called = "net.save_stash";

    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        state.log_error(HostError::OomPointer);
        return;
    };
    if buf.len() > 80 {
        state.log_error("stash size cannot exceed 80 bytes");
        return;
    }

    let mut handler = state.net_handler.replace(NetHandler::None);
    let peer = get_friend(&mut handler, peer_id);
    match peer {
        Some(peer) => {
            // Store the stash of the given peer (that isn't the current device).
            //
            // We keep the stash updates just in case the game want to load them back
            // using `net.load_stash`. However, we don't preserve them anywhere in the FS
            // after the game is closed. It's responsibility of each device to keep their own
            // stash and then share it when starting the game.
            rewrite_vec(&mut peer.stash, buf);
        }
        None => {
            // Store the stash of the current device.
            rewrite_vec(&mut state.stash, buf);
            state.stash_dirty = true;
        }
    }
    state.net_handler.replace(handler);
}

pub(crate) fn load_stash(mut caller: C, peer_id: u32, buf_ptr: u32, buf_len: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "net.load_stash";

    let Some(memory) = state.memory else {
        state.log_error(HostError::MemoryNotFound);
        return 0;
    };
    let (data, state) = memory.data_and_store_mut(&mut caller);
    let buf_ptr = buf_ptr as usize;
    let buf_len = buf_len as usize;
    let Some(buf) = data.get_mut(buf_ptr..(buf_ptr + buf_len)) else {
        state.log_error(HostError::OomPointer);
        return 0;
    };

    let mut handler = state.net_handler.replace(NetHandler::None);
    let peer = get_friend(&mut handler, peer_id);
    let stash = match peer {
        Some(peer) => &peer.stash,
        None => &state.stash,
    };
    let stash_len = stash.len();
    if stash_len > buf.len() {
        state.log_error("the buffer is not big enough to fit stash");
        buf.copy_from_slice(&stash[..buf.len()]);
    } else {
        buf[..stash_len].copy_from_slice(&stash[..]);
    };
    state.net_handler.replace(handler);
    stash_len as u32
}

/// Repalce the content of the vector with the buffer.
fn rewrite_vec(stash: &mut alloc::vec::Vec<u8>, buf: &[u8]) {
    let stash_len = stash.len();
    let buf_len = buf.len();
    if stash_len < buf_len {
        stash.copy_from_slice(&buf[..stash_len]);
        stash.extend_from_slice(&buf[stash_len..]);
    } else {
        stash.truncate(buf_len);
        stash.shrink_to_fit();
        stash.copy_from_slice(buf);
    }
}
