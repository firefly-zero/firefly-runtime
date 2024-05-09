use core::fmt::Display;

pub enum Error {
    Wasmi(wasmi::Error),
    FuncCall(&'static str, wasmi::Error),
    FileNotFound,
    NoLauncher,
    InvalidAuthorID(firefly_meta::ValidationError),
    InvalidAppID(firefly_meta::ValidationError),
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Wasmi(err) => write!(f, "wasm error: {err}"),
            Error::FileNotFound => write!(f, "file not found"),
            Error::NoLauncher => write!(f, "no launcher installed"),
            Error::FuncCall(func, err) => write!(f, "error calling {func}: {err}"),
            Error::InvalidAuthorID(err) => write!(f, "invalid author ID: {err}"),
            Error::InvalidAppID(err) => write!(f, "invalid app ID: {err}"),
        }
    }
}

impl From<wasmi::Error> for Error {
    fn from(value: wasmi::Error) -> Self {
        Self::Wasmi(value)
    }
}

// pub(crate) enum HostError {
//     NoneColor,
//     InvalidPointer,
// }
