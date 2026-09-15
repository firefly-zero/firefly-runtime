//! Undocumented wasm module with functions for sys.connector.

use crate::state::{NetHandler, State};
use alloc::boxed::Box;
use alloc::vec::Vec;
use firefly_hal::{Device, Duration, Network};

type C<'a, 'b> = wasmi::Caller<'a, Box<State<'b>>>;

/// Undocumented function called from `sys.connector` on "confirm".
///
/// Tells other peers that we're ready to go into [`NetHandler::Connection`]
/// state and wait for them to go into the same state.
pub(crate) fn set_ready(mut caller: C, peer_map: u32, hash: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "conn.set_ready";
    let mut handler = state.net_handler.replace(NetHandler::None);
    let NetHandler::Connector(connector) = &mut handler else {
        state.net_handler.replace(handler);
        state.log_error("can mark connection as ready only from connector");
        return 1;
    };

    // We send Ready even if our list of peers doesn't match the list of peers
    // that other devices have. That will trigger the peer mismatch error
    // on all devices instead of just this device which is better
    // since we don't know which device in particular has the wrong list.
    //
    // Typically it's the one with fewer peers but it's also possible
    // that some of devices have a "rogue" device from another
    // group of players nearby.
    let mut peer_map = peer_map;
    for peer in &connector.peer_infos {
        if peer_map & 1 == 1 {
            let res = connector.send_ready(&mut state.device, peer.addr, hash);
            if let Err(err) = res {
                state.net_handler.replace(handler);
                state.log_error(err);
                return 3;
            }
        }
        peer_map >>= 1;
    }

    state.net_handler.replace(handler);
    0
}

pub(crate) fn get_ready_map(mut caller: C, peer_map: u32, hash: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "conn.get_ready_map";
    let mut handler = state.net_handler.replace(NetHandler::None);
    let NetHandler::Connector(connector) = &mut handler else {
        state.net_handler.replace(handler);
        state.log_error("can check connection readiness only from connector");
        return 0;
    };
    let mut ready_map: u32 = 0;
    let mut peer_map = peer_map;
    for peer in &connector.peer_infos {
        ready_map >>= 1;
        if peer_map & 1 == 1 && peer.ready != 0 {
            ready_map |= 1;
            // Ensure that all peers have the same peer list hash.
            if peer.ready != hash {
                state.net_handler.replace(handler);
                return u32::MAX;
            }
        }
        peer_map >>= 1;
    }
    state.net_handler.replace(handler);
    ready_map
}

/// Undocumented function called from `sys.connector` in `before_exit`.
///
/// Sets the mapping of the peers that the user accepted
/// and transitions net state from Connector into Connection.
/// If none of the peers are accepted, transitions into None (offline) instead.
///
/// It doesn't need to be sudo because it only works in the `Connector` state
/// which is only available when `sys.connector` is running.
/// Curiously enough, `sys.connector` is not a `sudo` app.
pub(crate) fn set_peers(mut caller: C, peer_map: u32) {
    let state = caller.data_mut();
    state.called = "conn.set_peers";
    let handler = state.net_handler.replace(NetHandler::None);
    let NetHandler::Connector(mut connector) = handler else {
        state.net_handler.replace(handler);
        state.log_error("can set connection peers only from connector");
        return;
    };

    // TODO: do this in a single loop and without Vec.
    let mut to_remove: Vec<usize> = Vec::new();
    let mut peer_map = peer_map;
    for i in 0..connector.peer_infos.len() {
        if peer_map & 1 == 0 {
            to_remove.push(i);
        }
        peer_map >>= 1;
    }
    if !to_remove.is_empty() {
        to_remove.reverse();
        for i in to_remove {
            let res = connector.send_disconnect_to(&mut state.device, i);
            if let Err(err) = res {
                state.log_error(err);
            }
            connector.peer_infos.remove(i);
        }
        // If we sent disconnect to some peers and there are no more peers left
        // (which will make us disable networking below), wait a bit to make sure
        // the disconnect messages are delivered before we turn off wifi.
        if connector.peer_infos.is_empty() {
            state.device.delay(Duration::from_ms(50));
        }
    }

    if connector.peer_infos.is_empty() {
        state.set_next(None);
        state.net_handler.replace(NetHandler::None);
        let res = state.device.net_stop();
        if let Err(err) = res {
            state.log_error(err);
        }
        return;
    }

    if let Err(err) = connector.validate() {
        state.log_error(err);
    }
    state.set_next(None);
    let connection = connector.into_connection(&mut state.device);
    state
        .net_handler
        .replace(NetHandler::Connection(connection));
}
