use super::NetcodeError;
use crate::config::FullID;
use alloc::boxed::Box;
use firefly_hal::InputState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum Message {
    Req(Req),
    Resp(Resp),
}

impl From<Resp> for Message {
    fn from(v: Resp) -> Self {
        Self::Resp(v)
    }
}

impl From<Req> for Message {
    fn from(v: Req) -> Self {
        Self::Req(v)
    }
}

impl Message {
    // TODO: return NetworkError
    pub fn decode(s: &[u8]) -> Result<Self, NetcodeError> {
        if s.is_empty() {
            return Err(NetcodeError::EmptyBufferIn);
        }
        if s == b"HELLO" {
            return Ok(Self::Req(Req::Hello));
        }
        let res = postcard::from_bytes(s);
        match res {
            Ok(raw) => Ok(raw),
            Err(err) => Err(NetcodeError::Deserialize(err)),
        }
    }

    // TODO: return NetworkError
    pub fn encode<'a>(&self, buf: &'a mut [u8]) -> Result<&'a mut [u8], NetcodeError> {
        let res = postcard::to_slice(self, buf);
        match res {
            Ok(raw) => {
                if raw.is_empty() {
                    Err(NetcodeError::EmptyBufferOut)
                } else {
                    Ok(raw)
                }
            }
            Err(err) => Err(NetcodeError::Serialize(err)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Req {
    Hello,
    Intro,
    Start,
    State(u32),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Resp {
    Intro(Intro),
    Start(Start),
    State(FrameState),
}

impl From<FrameState> for Resp {
    fn from(v: FrameState) -> Self {
        Self::State(v)
    }
}

impl From<Start> for Resp {
    fn from(v: Start) -> Self {
        Self::Start(v)
    }
}

impl From<Intro> for Resp {
    fn from(v: Intro) -> Self {
        Self::Intro(v)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Intro {
    pub name: heapless::String<16>,
    pub version: u16,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub(crate) struct FrameState {
    pub frame: u32,
    pub input: Input,
    // rand: Option<...>
    // rand_key: Option<...>
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Start {
    /// The full ID of the app to run.
    pub id: FullID,
    /// The peer's progress for each badge.
    pub badges: Box<[u16]>,
    /// The peer's top score for each board.
    pub scores: Box<[i16]>,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub(crate) struct Input {
    pub pad: Option<(i16, i16)>,
    pub buttons: u8,
}

impl From<Input> for InputState {
    fn from(value: Input) -> Self {
        InputState {
            pad: value.pad.map(Into::into),
            buttons: value.buttons,
        }
    }
}
