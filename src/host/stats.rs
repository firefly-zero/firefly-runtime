use crate::{error::HostError, state::NetHandler, state::State};
use firefly_types::FriendScore;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn add_progress(mut caller: C, peer_id: u32, badge_id: u32, val: i32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.add_progress";

    let Some(stats) = &mut state.app_stats else {
        state.log_error(HostError::NoStats);
        return 0;
    };

    // We currently add progress only for the local player.
    // Each device stores its own progress.
    let handler = state.net_handler.get_mut();
    let friend_id = get_friend_id(handler, peer_id);
    if friend_id.is_some() {
        return 0;
    }

    let idx = (badge_id - 1) as usize;
    let Some(progress) = stats.badges.get_mut(idx) else {
        let err = if stats.badges.is_empty() {
            HostError::NoBadges
        } else {
            HostError::NoBadge(badge_id)
        };
        state.log_error(err);
        return 0;
    };
    if val != 0 {
        let Ok(val) = i16::try_from(val) else {
            state.log_error(HostError::ValueTooBig);
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

pub(crate) fn add_score(mut caller: C, peer_id: u32, board_id: u32, new_score: i32) -> i32 {
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
    let friend_id = get_friend_id(handler, peer_id);
    let Ok(new_score) = i16::try_from(new_score) else {
        state.log_error(HostError::ValueTooBig);
        return 0;
    };
    if let Some(friend_id) = friend_id {
        insert_friend_score(&mut scores.friends, friend_id, new_score);
        for friend in scores.friends.iter() {
            if friend.index == friend_id {
                return i32::from(friend.score);
            }
        }
    }
    insert_my_score(&mut scores.me, new_score);
    let personal_best = scores.me[0];
    i32::from(personal_best)
}

/// Get the friend's ID for the given peer to be used in scores.
///
/// Returns None if the given peer is this device.
fn get_friend_id(handler: &mut NetHandler, peer_id: u32) -> Option<u16> {
    if peer_id == 0xFF {
        return None;
    }
    let NetHandler::FrameSyncer(syncer) = handler else {
        return None;
    };
    let peer = syncer.peers.get(peer_id as usize)?;
    peer.addr?;
    Some(peer.friend_id)
}

fn insert_my_score(scores: &mut [i16; 8], new_score: i16) {
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

fn insert_friend_score(scores: &mut [FriendScore; 8], friend_id: u16, new_score: i16) {
    let mut idx = None;
    for (i, friend) in scores.iter().enumerate() {
        if new_score > friend.score {
            idx = Some(i);
            break;
        }
    }
    let Some(idx) = idx else { return };
    scores[idx..].rotate_right(1);
    scores[idx] = FriendScore {
        index: friend_id,
        score: new_score,
    };
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
