use crate::error::HostError;
use crate::state::State;
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
