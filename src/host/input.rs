use crate::error::HostError;
use crate::state::{NetHandler, State};
use firefly_device::*;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn read_pad(mut caller: C, player: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "input.read_pad";
    let Some(input) = get_input(state, player) else {
        return 0xffff;
    };
    let Some(pad) = input.pad else {
        return 0xffff;
    };
    let x = pad.x as u16 as u32;
    let y = pad.y as u16 as u32;
    x << 16 | y
}

pub(crate) fn read_buttons(mut caller: C, _player: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "input.read_buttons";
    let Some(input) = &state.input else {
        return 0;
    };
    let mut res: u32 = 0;
    for button in input.buttons.into_iter().rev() {
        res = (res << 1) | u32::from(button);
    }
    res
}

fn get_input(state: &mut State, player: u32) -> Option<InputState> {
    let NetHandler::FrameSyncer(syncer) = state.net_handler.get_mut() else {
        return state.input.clone();
    };

    let Some(peer) = syncer.peers.get(player as usize) else {
        state.log_error(HostError::UnknownPeer(player));
        return None;
    };
    let Some(frame_state) = peer.states.get_current() else {
        return None;
    };
    let input = frame_state.input;
    let input = InputState {
        pad: input.pad.map(|p| Pad { x: p.0, y: p.1 }),
        buttons: [false; 5],
    };
    Some(input)
}
