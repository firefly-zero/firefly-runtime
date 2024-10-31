use crate::{error::HostError, state::State};

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn get_progress(mut caller: C, id: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.get_progress";
    let Some(stats) = &mut state.app_stats else {
        state.log_error(HostError::NoStats);
        return 0;
    };
    let Some(progress) = stats.badges.get(id as usize) else {
        let err = if stats.badges.is_empty() {
            HostError::NoBadges
        } else {
            HostError::NoBadge
        };
        state.log_error(err);
        return 0;
    };
    u32::from(progress.done) << 16 | u32::from(progress.goal)
}

pub(crate) fn add_progress(mut caller: C, id: u32, val: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.add_progress";
    todo!()
}

pub(crate) fn set_score(mut caller: C, peer_id: u32, board_id: u32, new_score: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.set_score";
    todo!()
}

pub(crate) fn get_top_score(mut caller: C, peer_id: u32, board_id: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.get_top_score";
    todo!()
}
