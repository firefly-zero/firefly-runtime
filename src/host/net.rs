use crate::{
    error::HostError,
    net::FSPeer,
    state::{NetHandler, State},
};

use super::stats::get_friend;

type C<'a> = wasmi::Caller<'a, State>;

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
        Some(peer) => save_stash_friend(peer, buf),
        None => {
            let buf = alloc::vec::Vec::from(buf);
            state.stash = Some(buf.into_boxed_slice());
            state.stash_dirty = true;
        }
    }
    state.net_handler.replace(handler);
}

/// Store the stash of the given peer (that isn't the current device).
///
/// We keep the stash updates just in case the game want to load them back
/// using `net.load_stash`. However, we don't preserve them anywhere in the FS
/// after the game is closed. It's responsibility of each device to keep their own
/// stash and then share it when starting the game.
fn save_stash_friend(peer: &mut FSPeer, buf: &[u8]) {
    let stash_len = peer.stash.len();
    let buf_len = buf.len();
    if stash_len < buf_len {
        peer.stash.copy_from_slice(&buf[..stash_len]);
        peer.stash.extend_from_slice(&buf[stash_len..]);
    } else {
        peer.stash.truncate(buf_len);
        peer.stash.shrink_to_fit();
        peer.stash.copy_from_slice(buf);
    }
}
