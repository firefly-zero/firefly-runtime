use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum Message {
    Req(Req),
    Resp(Resp),
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
    // Start,
    // State,
    // Input,
}

#[derive(Serialize, Deserialize)]

pub(crate) enum Resp {
    Intro(Intro),
    // Start(Start),
    // State(State),
    // Input(Input),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Intro {
    pub name:    heapless::String<16>,
    pub version: u16,
}
