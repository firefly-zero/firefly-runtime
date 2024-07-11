use crate::state::{NetHandler, State};

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
