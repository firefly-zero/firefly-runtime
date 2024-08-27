use crate::error::HostError;
use crate::state::State;
use alloc::boxed::Box;
use firefly_audio::*;
// use firefly_device::*;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn get_sink(mut caller: C, index: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.get_sink";
    let sink = match index {
        1 => Sink::Adaptive,
        2 => Sink::Headphones,
        3 => Sink::Speakers,
        _ => {
            state.log_error(HostError::UnknownNode(index));
            return 0;
        }
    };
    sink.id()
}

pub(crate) fn add_sine(mut caller: C, parent_id: u32, freq: f32, phase: f32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_sine";
    let beh = Sine::new(freq, phase);
    let Some(parent) = state.audio.root.get_node(parent_id) else {
        state.log_error(HostError::UnknownNode(parent_id));
        return 0;
    };
    let Some(id) = parent.add(Box::new(beh)) else {
        state.log_error(HostError::TooManyNodes(parent_id));
        return 0;
    };
    id
}

pub(crate) fn reset(mut caller: C, node_id: u32) {
    let state = caller.data_mut();
    state.called = "audio.reset";
    let Some(node) = state.audio.root.get_node(node_id) else {
        state.log_error(HostError::UnknownNode(node_id));
        return;
    };
    node.reset();
}
