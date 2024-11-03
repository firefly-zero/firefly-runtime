use crate::{error::HostError, state::NetHandler, state::State};

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
    if val != 0 {
        let Ok(val) = i16::try_from(val) else {
            state.log_error("the value is too big");
            return 0;
        };
        if progress.done < progress.goal {
            let new = (progress.done as i16).saturating_add(val);
            let new = new.max(0) as u16;
            progress.done = u16::min(new, progress.goal);
            state.app_stats_dirty = true;
            if progress.done >= progress.goal {
                progress.new = true;
            }
        }
    }
    u32::from(progress.done) << 16 | u32::from(progress.goal)
}

pub(crate) fn set_score(mut caller: C, board_id: u32, peer_id: u32, new_score: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.set_score";
    let Some(stats) = &mut state.app_stats else {
        state.log_error(HostError::NoStats);
        return 0;
    };
    let board_idx = (board_id - 1) as usize;
    let Some(scores) = stats.scores.get_mut(board_idx) else {
        let err = if stats.scores.is_empty() {
            HostError::NoBoards
        } else {
            HostError::NoBoard(board_id)
        };
        state.log_error(err);
        return 0;
    };

    let handler = state.net_handler.get_mut();
    let is_me = match handler {
        NetHandler::FrameSyncer(syncer) => match syncer.peers.get(peer_id as usize) {
            Some(peer) => peer.addr.is_none(),
            None => true,
        },
        _ => true,
    };
    let Ok(new_score) = u16::try_from(new_score) else {
        state.log_error("the value is too big");
        return 0;
    };
    let personal_best = if is_me {
        insert_my_score(&mut scores.me, new_score);
        scores.me[0]
    } else {
        todo!()
        // insert_score(&mut scores.friends, new_score);
    };
    u32::from(personal_best)
}

pub(crate) fn get_top_score(mut caller: C, board_id: u32, peer_id: u32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.get_top_score";
    todo!()
}

fn insert_my_score(scores: &mut [u16; 8], new_score: u16) {
    let mut idx = None;
    for (i, old_score) in scores.iter().enumerate() {
        if new_score > *old_score {
            idx = Some(i);
            break;
        }
    }
    let Some(idx) = idx else { return };
    scores[idx..].rotate_right(1);
    scores[idx] = new_score;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_my_score() {
        let mut scores = [0, 0, 0, 0, 0, 0, 0, 0];
        insert_my_score(&mut scores, 13);
        assert_eq!(scores, [13, 0, 0, 0, 0, 0, 0, 0]);
        insert_my_score(&mut scores, 20);
        assert_eq!(scores, [20, 13, 0, 0, 0, 0, 0, 0]);
        insert_my_score(&mut scores, 13);
        assert_eq!(scores, [20, 13, 13, 0, 0, 0, 0, 0]);
        insert_my_score(&mut scores, 15);
        assert_eq!(scores, [20, 15, 13, 13, 0, 0, 0, 0]);
        insert_my_score(&mut scores, 7);
        assert_eq!(scores, [20, 15, 13, 13, 7, 0, 0, 0]);
        insert_my_score(&mut scores, 6);
        assert_eq!(scores, [20, 15, 13, 13, 7, 6, 0, 0]);
        insert_my_score(&mut scores, 7);
        assert_eq!(scores, [20, 15, 13, 13, 7, 7, 6, 0]);
        insert_my_score(&mut scores, 3);
        assert_eq!(scores, [20, 15, 13, 13, 7, 7, 6, 3]);
        insert_my_score(&mut scores, 2);
        assert_eq!(scores, [20, 15, 13, 13, 7, 7, 6, 3]);
        insert_my_score(&mut scores, 3);
        assert_eq!(scores, [20, 15, 13, 13, 7, 7, 6, 3]);
        insert_my_score(&mut scores, 1);
        assert_eq!(scores, [20, 15, 13, 13, 7, 7, 6, 3]);
        insert_my_score(&mut scores, 14);
        assert_eq!(scores, [20, 15, 14, 13, 13, 7, 7, 6]);
    }
}
