use crate::error::HostError;
use crate::net::FSPeer;
use crate::state::{NetHandler, State};
use alloc::boxed::Box;
use firefly_types::FriendScore;

type C<'a, 'b> = wasmi::Caller<'a, Box<State<'b>>>;

/// The peer_id representing the "combined"
const COMBINED: u32 = 0xff;

pub(crate) fn add_progress(mut caller: C, peer_id: u32, badge_id: u32, val: i32) -> u32 {
    let state = caller.data_mut();
    state.called = "stats.add_progress";
    let mut handler = state.net_handler.replace(NetHandler::None);
    let peer = get_friend(&mut handler, peer_id);
    let result = if let Some(peer) = peer {
        add_progress_friend(state, peer, badge_id, val)
    } else if peer_id == COMBINED {
        add_progress_everyone(state, &mut handler, badge_id, val)
    } else {
        add_progress_me(state, badge_id, val)
    };
    state.net_handler.replace(handler);
    result
}

fn add_progress_everyone(
    state: &mut State,
    handler: &mut NetHandler,
    badge_id: u32,
    val: i32,
) -> u32 {
    let NetHandler::FrameSyncer(syncer) = handler else {
        return add_progress_me(state, badge_id, val);
    };
    let mut worst: u32 = u32::MAX;
    for peer in syncer.peers.iter_mut() {
        let peer_progress = if peer.addr.is_none() {
            add_progress_me(state, badge_id, val)
        } else {
            add_progress_friend(state, peer, badge_id, val)
        };
        if peer_progress < worst {
            worst = peer_progress;
        }
    }
    worst
}

fn add_progress_me(state: &mut State, badge_id: u32, val: i32) -> u32 {
    let Some(stats) = &mut state.app_stats else {
        state.log_error(HostError::NoStats);
        return 0;
    };
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
            if progress.done >= progress.goal {
                progress.new = true;
            }
        }
    }
    u32::from(progress.done) << 16 | u32::from(progress.goal)
}

fn add_progress_friend(state: &mut State, peer: &mut FSPeer, badge_id: u32, val: i32) -> u32 {
    let Some(stats) = &mut state.app_stats else {
        state.log_error(HostError::NoStats);
        return 0;
    };
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
    let idx = (badge_id - 1) as usize;
    let Some(done) = peer.badges.get_mut(idx) else {
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
        if *done < progress.goal {
            let new = (*done as i16).saturating_add(val);
            let new = new.max(0) as u16;
            *done = u16::min(new, progress.goal);
            if *done >= progress.goal {
                progress.new = true;
            }
        }
    }
    u32::from(*done) << 16 | u32::from(progress.goal)
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
    let peer = get_friend(handler, peer_id);
    let Ok(new_score) = i16::try_from(new_score) else {
        state.log_error(HostError::ValueTooBig);
        return 0;
    };
    if let Some(peer) = peer {
        let friend_id = peer.friend_id.unwrap();
        insert_friend_score(&mut scores.friends, friend_id, new_score);
        let Some(max_score) = peer.scores.get_mut(board_idx) else {
            let err = if peer.scores.is_empty() {
                HostError::NoBoards
            } else {
                HostError::NoBoard(board_id)
            };
            state.log_error(err);
            return 0;
        };
        if new_score > *max_score {
            *max_score = new_score;
        };
        i32::from(*max_score)
    } else if peer_id == COMBINED {
        let NetHandler::FrameSyncer(syncer) = handler else {
            insert_my_score(&mut scores.me, new_score);
            let personal_best = scores.me[0];
            return i32::from(personal_best);
        };
        let mut lowest: i32 = i32::MAX;
        for peer in syncer.peers.iter_mut() {
            let personal_best = if let Some(friend_id) = peer.friend_id {
                insert_friend_score(&mut scores.friends, friend_id, new_score);
                let Some(max_score) = peer.scores.get_mut(board_idx) else {
                    let err = if peer.scores.is_empty() {
                        HostError::NoBoards
                    } else {
                        HostError::NoBoard(board_id)
                    };
                    state.log_error(err);
                    return 0;
                };
                if new_score > *max_score {
                    *max_score = new_score;
                };
                i32::from(*max_score)
            } else {
                insert_my_score(&mut scores.me, new_score);
                let personal_best = scores.me[0];
                i32::from(personal_best)
            };
            if personal_best < lowest {
                lowest = personal_best;
            }
        }
        lowest
    } else {
        insert_my_score(&mut scores.me, new_score);
        let personal_best = scores.me[0];
        i32::from(personal_best)
    }
}

/// Get the peer with the given ID but only if it's not the current device.
///
/// Returns None if the given peer is this device.
pub(crate) fn get_friend<'b>(
    handler: &'b mut NetHandler<'_>,
    peer_id: u32,
) -> Option<&'b mut FSPeer> {
    if peer_id == COMBINED {
        return None;
    }
    let NetHandler::FrameSyncer(syncer) = handler else {
        return None;
    };
    let peer = syncer.peers.get_mut(peer_id as usize)?;
    peer.addr?;
    peer.friend_id?;
    Some(peer)
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

/// Insert a friend's score into the friends' scoreboard.
///
/// It tries to keep the board diverse: if possible, it will remove the lowest score
/// of the same person rather than of someone else.
fn insert_friend_score(scores: &mut [FriendScore; 8], friend_id: u16, new_score: i16) {
    let full = scores.last().unwrap().score != 0;

    // Skip all scores higher than the new one.
    let mut start = 0;
    for friend in scores.iter() {
        if new_score >= friend.score {
            break;
        }
        // If we already filled the scoreboard and the given friend
        // already has a higher score, just keep that higher score
        // instead of inserting a new one.
        if full && friend.index == friend_id {
            return;
        }
        start += 1;
    }
    let scores = &mut scores[start..];

    // In the upcoming shift of scores, insert first the new score.
    let mut prev = FriendScore {
        index: friend_id,
        score: new_score,
    };
    for friend in scores {
        let same_person = friend.index == friend_id;
        // Shift all scores to the right.
        core::mem::swap(friend, &mut prev);
        // Stop shifting when hitting the same person.
        // It's better to keep only the highest score from the same person
        // rather than remove highest (but lower than the new one) scores
        // of other people. We want to keep the scoreboards diverse.
        if full && same_person {
            break;
        }
    }
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

    fn new_score(i: u16, s: i16) -> FriendScore {
        FriendScore { index: i, score: s }
    }

    #[test]
    fn test_insert_friend_score() {
        let mut scores = [
            new_score(1, 43),
            new_score(1, 30),
            new_score(3, 28),
            new_score(4, 28),
            new_score(3, 20),
            new_score(5, 15),
            new_score(0, 0),
            new_score(0, 0),
        ];
        insert_friend_score(&mut scores, 6, 55);
        insert_friend_score(&mut scores, 6, 16);
        insert_friend_score(&mut scores, 6, 2);
        insert_friend_score(&mut scores, 3, 4);
        insert_friend_score(&mut scores, 3, 26);
        insert_friend_score(&mut scores, 1, 45);
        let expected = [
            new_score(6, 55),
            new_score(1, 45),
            new_score(1, 30),
            new_score(3, 28),
            new_score(4, 28),
            new_score(3, 20),
            new_score(6, 16),
            new_score(5, 15),
        ];
        assert_eq!(scores, expected);
    }
}
