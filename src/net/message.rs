use crate::config::FullID;
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
    pub fn decode(s: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(s)
    }

    pub fn encode<'a>(&self, buf: &'a mut [u8]) -> Result<&'a mut [u8], postcard::Error> {
        postcard::to_slice(self, buf)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Req {
    Intro,
    Start,
    // State,
    // Input,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Resp {
    Intro(Intro),
    Start(FullID),
    // State(State),
    // Input(Input),
}

impl From<FullID> for Resp {
    fn from(v: FullID) -> Self {
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
