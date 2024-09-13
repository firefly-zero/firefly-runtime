use crate::error::HostError;
use crate::state::State;
use alloc::boxed::Box;
use firefly_audio::*;

type C<'a> = wasmi::Caller<'a, State>;

/// Add sine wave generator as a child for the given node.
pub(crate) fn add_sine(mut caller: C, parent_id: u32, freq: f32, phase: f32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_sine";
    let proc = Sine::new(freq, phase);
    add_node(state, parent_id, Box::new(proc))
}

/// Add square wave generator as a child for the given node.
pub(crate) fn add_square(mut caller: C, parent_id: u32, freq: f32, phase: f32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_square";
    let proc = Square::new(freq, phase);
    add_node(state, parent_id, Box::new(proc))
}

/// Add sawtooth wave generator as a child for the given node.
pub(crate) fn add_sawtooth(mut caller: C, parent_id: u32, freq: f32, phase: f32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_sawtooth";
    let proc = Sawtooth::new(freq, phase);
    add_node(state, parent_id, Box::new(proc))
}

/// Add white noise generator as a child for the given node.
pub(crate) fn add_noise(mut caller: C, parent_id: u32, seed: i32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_noise";
    let proc = Noise::new(seed);
    add_node(state, parent_id, Box::new(proc))
}

/// Add empty source as a child for the given node.
pub(crate) fn add_empty(mut caller: C, parent_id: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_empty";
    let proc = Empty::new();
    add_node(state, parent_id, Box::new(proc))
}

/// Add zero source as a child for the given node.
pub(crate) fn add_zero(mut caller: C, parent_id: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_zero";
    let proc = Zero::new();
    add_node(state, parent_id, Box::new(proc))
}

/// Add gain filter as a child for the given node.
pub(crate) fn add_gain(mut caller: C, parent_id: u32, lvl: f32) -> u32 {
    let state = caller.data_mut();
    state.called = "audio.add_gain";
    let proc = Gain::new(lvl);
    add_node(state, parent_id, Box::new(proc))
}

fn add_node(state: &mut State, parent_id: u32, proc: Box<dyn firefly_audio::Processor>) -> u32 {
    match state.audio.add_node(parent_id, proc) {
        Ok(id) => id,
        Err(err) => {
            state.log_error(HostError::AudioNode(err));
            0
        }
    }
}

/// Modulate a parameter of the given node using linear modulation.
pub(crate) fn mod_linear(
    mut caller: C,
    node_id: u32,
    param: u32,
    start: f32,
    end: f32,
    start_at: u32,
    end_at: u32,
) {
    let state = caller.data_mut();
    state.called = "audio.mod_linear";
    let lfo = modulators::Linear::new(start, end, start_at, end_at);
    modulate(state, node_id, param, Box::new(lfo));
}

pub(crate) fn mod_hold(mut caller: C, node_id: u32, param: u32, v1: f32, v2: f32, time: u32) {
    let state = caller.data_mut();
    state.called = "audio.mod_hold";
    let lfo = modulators::Hold::new(v1, v2, time);
    modulate(state, node_id, param, Box::new(lfo));
}

pub(crate) fn mod_sine(mut caller: C, node_id: u32, param: u32, freq: f32, low: f32, high: f32) {
    let state = caller.data_mut();
    state.called = "audio.mod_sine";
    let lfo = modulators::Sine::new(freq, low, high);
    modulate(state, node_id, param, Box::new(lfo));
}

fn modulate(state: &mut State, node_id: u32, param: u32, lfo: Box<dyn modulators::Modulator>) {
    let node = match state.audio.get_node(node_id) {
        Ok(node) => node,
        Err(err) => {
            state.log_error(HostError::AudioNode(err));
            return;
        }
    };
    if param > 8 {
        state.log_error("param value is too high");
        return;
    }
    node.modulate(param as u8, lfo);
}

/// Reset the given node.
pub(crate) fn reset(mut caller: C, node_id: u32) {
    let state = caller.data_mut();
    state.called = "audio.reset";
    match state.audio.get_node(node_id) {
        Ok(node) => node.reset(),
        Err(err) => state.log_error(HostError::AudioNode(err)),
    };
}

/// Reset the given node and all its child nodes.
pub(crate) fn reset_all(mut caller: C, node_id: u32) {
    let state = caller.data_mut();
    state.called = "audio.reset_all";
    match state.audio.get_node(node_id) {
        Ok(node) => node.reset_all(),
        Err(err) => state.log_error(HostError::AudioNode(err)),
    };
}

/// Remove all children from the node.
pub(crate) fn clear(mut caller: C, node_id: u32) {
    let state = caller.data_mut();
    state.called = "audio.clear";
    let res = state.audio.clear(node_id);
    if let Err(err) = res {
        state.log_error(HostError::AudioNode(err));
    }
}
