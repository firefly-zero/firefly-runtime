use core::fmt;

pub enum Error {
    Wasmi(wasmi::Error),
    FuncCall(&'static str, wasmi::Error),
    FileNotFound,
    NoLauncher,
    InvalidAuthorID(firefly_meta::ValidationError),
    InvalidAppID(firefly_meta::ValidationError),
    CannotDisplay,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Wasmi(err) => write!(f, "wasm error: {err}"),
            Error::FileNotFound => write!(f, "file not found"),
            Error::NoLauncher => write!(f, "no launcher installed"),
            Error::FuncCall(func, err) => write!(f, "error calling {func}: {err}"),
            Error::InvalidAuthorID(err) => write!(f, "invalid author ID: {err}"),
            Error::InvalidAppID(err) => write!(f, "invalid app ID: {err}"),
            Error::CannotDisplay => write!(f, "failed to draw on the display"),
        }
    }
}

impl From<wasmi::Error> for Error {
    fn from(value: wasmi::Error) -> Self {
        Self::Wasmi(value)
    }
}

/// Errors logged into console by host-defined functions.
pub(crate) enum HostError {
    MemoryNotFound,
    OomPointer,
    BufferSize,
    FileNotFound,
    FileRead,
    FileCreate,
    FileRemove,
    FileFlush,
    FileNameUtf8,
    FileName(firefly_meta::ValidationError),
    IdUtf8,
    Id(firefly_meta::ValidationError),
    TextUtf8,
    NoneColor,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::MemoryNotFound => write!(f, "memory not found"),
            HostError::FileNotFound => write!(f, "file not found"),
            HostError::OomPointer => write!(f, "buffer points out of memory"),
            HostError::FileRead => write!(f, "cannot read file"),
            HostError::FileCreate => write!(f, "cannot create file"),
            HostError::FileRemove => write!(f, "cannot remove file"),
            HostError::FileFlush => write!(f, "cannot flush file"),
            HostError::BufferSize => write!(f, "buffer size for file does not match file size"),
            HostError::FileNameUtf8 => write!(f, "file name is not valid UTF-8"),
            HostError::FileName(err) => write!(f, "bad file name: {err}"),
            HostError::IdUtf8 => write!(f, "ID is not valid UTF-8"),
            HostError::Id(err) => write!(f, "bad ID: {err}"),
            HostError::TextUtf8 => write!(f, "text is not valid UTF-8"),
            HostError::NoneColor => write!(f, "color is None (0)"),
        }
    }
}
