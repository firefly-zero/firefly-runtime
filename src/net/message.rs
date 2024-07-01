use alloc::string::{String, ToString};

pub(crate) trait Serialize<S> {
    fn serialize<W: embedded_io::Write>(&self, w: W) -> Result<(), W::Error>;
    fn deserialize(r: &[u8]) -> Result<S, ()>;
}

pub(crate) enum Message {
    Req(Req),
    Resp(Resp),
}

impl Serialize<Message> for Message {
    fn serialize<W: embedded_io::Write>(&self, mut w: W) -> Result<(), W::Error> {
        match self {
            Message::Req(req) => {
                w.write(&[1])?;
                req.serialize(w)
            }
            Message::Resp(resp) => {
                w.write(&[2])?;
                resp.serialize(w)
            }
        }
    }

    fn deserialize(r: &[u8]) -> Result<Message, ()> {
        match r[0] {
            1 => Ok(Self::Req(Req::deserialize(&r[1..])?)),
            _ => Ok(Self::Resp(Resp::deserialize(&r[1..])?)),
        }
    }
}

pub(crate) enum Req {
    Intro,
    // Start,
    // State,
    // Input,
}

impl Serialize<Req> for Req {
    fn serialize<W: embedded_io::Write>(&self, mut w: W) -> Result<(), W::Error> {
        w.write(&[0])?;
        Ok(())
    }

    fn deserialize(r: &[u8]) -> Result<Req, ()> {
        Ok(Self::Intro)
    }
}

pub(crate) enum Resp {
    Intro(Intro),
    // Start(Start),
    // State(State),
    // Input(Input),
}

impl Serialize<Resp> for Resp {
    fn serialize<W: embedded_io::Write>(&self, w: W) -> Result<(), W::Error> {
        match self {
            Resp::Intro(intro) => intro.serialize(w),
        }
    }

    fn deserialize(r: &[u8]) -> Result<Resp, ()> {
        Ok(Resp::Intro(Intro::deserialize(r)?))
    }
}

pub(crate) struct Intro {
    pub name:    String,
    pub version: u16,
}

impl Serialize<Intro> for Intro {
    fn serialize<W: embedded_io::Write>(&self, mut w: W) -> Result<(), W::Error> {
        w.write_all(self.name.as_bytes())
    }

    fn deserialize(r: &[u8]) -> Result<Self, ()> {
        let version = u16::from_le_bytes([r[0], r[1]]);
        // UTF-8 validation will be handled later by "validate"
        let name = unsafe { core::str::from_utf8_unchecked(&r[2..]) };
        Ok(Self {
            name: name.to_string(),
            version,
        })
    }
}
