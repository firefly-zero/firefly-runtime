use core::fmt::Display;

pub enum Error {
    Wasmi(wasmi::Error),
    FileNotFound,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Wasmi(err) => write!(f, "wasm error: {err}"),
            Error::FileNotFound => write!(f, "file not found"),
        }
    }
}

impl From<wasmi::Error> for Error {
    fn from(value: wasmi::Error) -> Self {
        Self::Wasmi(value)
    }
}
