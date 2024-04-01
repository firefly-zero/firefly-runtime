use crate::state::State;

type C<'a> = wasmi::Caller<'a, State>;

pub(crate) fn read_left(mut _caller: C) -> u64 {
    todo!()
}

pub(crate) fn read_right(mut _caller: C) -> u64 {
    todo!()
}

pub(crate) fn read_buttons(mut _caller: C) -> u64 {
    todo!()
}
