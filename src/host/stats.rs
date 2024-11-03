use crate::{error::HostError, state::State};

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn get_progress(mut caller: C, id: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.get_progress";
    let Some(stats) = &mut state.app_stats else {
        state.log_error(HostError::NoStats);
        return 0;
    };
    let idx = (id - 1) as usize;
    let Some(progress) = stats.badges.get(idx) else {
        let err = if stats.badges.is_empty() {
            HostError::NoBadges
        } else {
            HostError::NoBadge(id)
        };
        state.log_error(err);
        return 0;
    };
    u32::from(progress.done) << 16 | u32::from(progress.goal)
}

pub(crate) fn add_progress(mut caller: C, id: u32, val: i32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.add_progress";

    let Some(stats) = &mut state.app_stats else {
        state.log_error(HostError::NoStats);
        return 0;
    };
    let idx = (id - 1) as usize;
    let Some(progress) = stats.badges.get_mut(idx) else {
        let err = if stats.badges.is_empty() {
            HostError::NoBadges
        } else {
            HostError::NoBadge(id)
        };
        state.log_error(err);
        return 0;
    };
    let Ok(val) = i16::try_from(val) else {
        state.log_error("the value is too big");
        return 0;
    };
    if progress.done < progress.goal {
        let new = (progress.done as i16 + val) as u16;
        progress.done = u16::min(new, progress.goal);
        state.app_stats_dirty = true;
        if progress.done >= progress.goal {
            progress.new = true;
        }
    }
    u32::from(progress.done) << 16 | u32::from(progress.goal)
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
