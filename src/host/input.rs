use crate::error::HostError;
use crate::state::{NetHandler, State};
use firefly_hal::*;

type C<'a, 'b> = wasmi::Caller<'a, State<'b>>;


/// Get the finger position on the pad.
///
/// We pack both x and y in a single u32 value because most of the wasm
/// compilers still don't support multiple return values. Shame!
pub(crate) fn read_pad(mut caller: C, index: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "input.read_pad";
    let Some(input) = get_input(state, index) else {
        return 0xffff;
    };
    let Some(pad) = input.pad else {
        return 0xffff;
    };
    let x = pad.x as u16 as u32;
    let y = pad.y as u16 as u32;
    x << 16 | y
}

/// Get the map of the pressed buttons.
///
/// 0 bit represents `A`, 1 bit represents `B`, an so on in order:
/// `A`, `B`, `X`, `Y`, and `menu`.
pub(crate) fn read_buttons(mut caller: C, index: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "input.read_buttons";
    let Some(input) = get_input(state, index) else {
        return 0;
    };
    u32::from(input.buttons)
}

/// Get the input for the peer with the given ID.
///
/// Automatically picks between local input, peer input, or combined input.
fn get_input(state: &mut State, index: u32) -> Option<InputState> {
    // If running in offline mode, the input is the local device input.
    let NetHandler::FrameSyncer(syncer) = state.net_handler.get_mut() else {
        return state.input.clone();
    };

    // Since we use u32 to map all peers in `net.get_peers`, there cannot be more
    // than 32 peers connected. We can use the remaining invalid IDs as an indication
    // of requesting a combined input.
    //
    // The official SDKs use the index 255.
    if index > 32 {
        return Some(syncer.get_combined_input());
    }

    let Some(peer) = syncer.peers.get(index as usize) else {
        state.log_error(HostError::UnknownPeer(index));
        return None;
    };
    let Some(frame_state) = peer.states.get_current() else {
        // No input known for the frame. Currently, it happens for the first frame.
        //
        // TODO: Make it impossible. Otherwise, we risk that some peers will have input
        //       and some won't resulting in different handled input.
        return None;
    };
    let input = frame_state.input;
    let input = InputState {
        pad: input.pad.map(Into::into),
        buttons: frame_state.input.buttons,
    };
    Some(input)
}
