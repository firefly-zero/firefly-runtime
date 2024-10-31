use crate::state::State;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn get_progress(mut caller: C, id: u32) -> u32 {
    todo!()
}

pub(crate) fn add_progress(mut caller: C, id: u32, val: u32) -> u32 {
    todo!()
}

pub(crate) fn set_score(mut caller: C, peer_id: u32, board_id: u32, new_score: u32) -> u32 {
    todo!()
}

pub(crate) fn get_top_score(mut caller: C, peer_id: u32, board_id: u32) -> u32 {
    todo!()
}
