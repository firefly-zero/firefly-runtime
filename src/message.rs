use alloc::string::{String, ToString};

pub(crate) trait Serialize<S> {
    fn serialize<W: embedded_io::Write>(&self, w: W) -> Result<(), W::Error>;
    fn deserialize<R: embedded_io::Read>(r: R) -> Result<S, R::Error>;
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

    fn deserialize<R: embedded_io::Read>(mut r: R) -> Result<Message, R::Error> {
        let mut buf = [0u8];
        r.read(&mut buf)?;
        match buf[0] {
            1 => Ok(Self::Req(Req::deserialize(r)?)),
            _ => Ok(Self::Resp(Resp::deserialize(r)?)),
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

    fn deserialize<R: embedded_io::Read>(r: R) -> Result<Req, R::Error> {
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

    fn deserialize<R: embedded_io::Read>(r: R) -> Result<Resp, R::Error> {
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

    fn deserialize<R: embedded_io::Read>(mut r: R) -> Result<Self, R::Error> {
        let mut buf = [0u8; 16];
        let res = r.read_exact(&mut buf[..2]);
        if let Err(err) = res {
            match err {
                embedded_io::ReadExactError::UnexpectedEof => {}
                embedded_io::ReadExactError::Other(err) => return Err(err),
            }
        }
        let version = u16::from_le_bytes([buf[0], buf[1]]);
        let size = r.read(&mut buf)?;
        // UTF-8 validation will be handled later by "validate"
        let name = unsafe { core::str::from_utf8_unchecked(&buf[..size]) };
        Ok(Self {
            name: name.to_string(),
            version,
        })
    }
}
