use core::fmt;

pub enum Error {
    Wasmi(wasmi::Error),
    FuncCall(&'static str, wasmi::Error, Stats),
    FileNotFound(alloc::string::String),
    NoLauncher,
    InvalidAuthorID(firefly_types::ValidationError),
    InvalidAppID(firefly_types::ValidationError),
    InvalidWidth,
    CannotDisplay,
    ReadMeta,
    ReadStats,
    AuthorIDMismatch,
    AppIDMismatch,

    DecodeMeta(postcard::Error),
    DecodeStats(postcard::Error),
    SerialEncode(postcard::Error),
    SerialDecode(postcard::Error),

    SerialStart(firefly_device::NetworkError),
    SerialSend(firefly_device::NetworkError),
    SerialRecv(firefly_device::NetworkError),

    CheatUndefined,
    CheatInNet,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Wasmi(err) => write!(f, "wasm error: {err}"),
            Error::FileNotFound(s) => write!(f, "file not found: {s}"),
            Error::NoLauncher => write!(f, "no launcher installed"),
            Error::FuncCall(func, err, stats) => write!(f, "error calling {func}: {err}.\n{stats}"),
            Error::InvalidAuthorID(err) => write!(f, "invalid author ID: {err}"),
            Error::InvalidAppID(err) => write!(f, "invalid app ID: {err}"),
            Error::InvalidWidth => write!(f, "the image has invalid width"),
            Error::CannotDisplay => write!(f, "failed to draw on the display"),
            Error::ReadMeta => write!(f, "cannot read _meta"),
            Error::ReadStats => write!(f, "cannot read _meta"),
            Error::AuthorIDMismatch => write!(f, "author ID in meta and in path don't match"),
            Error::AppIDMismatch => write!(f, "app ID in meta and in path don't match"),
            Error::DecodeMeta(err) => write!(f, "cannot decode _meta: {err}"),
            Error::DecodeStats(err) => write!(f, "cannot decode stats: {err}"),
            Error::SerialEncode(err) => write!(f, "cannot encode response for serial: {err}"),
            Error::SerialDecode(err) => write!(f, "cannot decode request from serial: {err}"),
            Error::SerialStart(err) => write!(f, "cannot connect to serial port: {err}"),
            Error::SerialSend(err) => write!(f, "cannot send into serial port: {err}"),
            Error::SerialRecv(err) => write!(f, "cannot read from serial port: {err}"),
            Error::CheatUndefined => write!(f, "the app doesn't have cheat callback"),
            Error::CheatInNet => write!(f, "cheats are disabled in multiplayer"),
        }
    }
}

impl From<wasmi::Error> for Error {
    fn from(value: wasmi::Error) -> Self {
        Self::Wasmi(value)
    }
}

/// Runtime stats provided on guest failure that should help to debug the failure cause.
pub struct Stats {
    pub(crate) last_called: &'static str,
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.last_called.is_empty() {
            writeln!(f, "No host functions were called.")?;
        } else {
            writeln!(f, "The last called host function is {}.", self.last_called)?;
        }
        Ok(())
    }
}

/// Errors logged into console by host-defined functions.
pub(crate) enum HostError {
    MemoryNotFound,
    OomPointer,
    BufferSize,
    FileNotFound,
    FileReadOnly,
    FileRead,
    FileCreate,
    FileRemove,
    FileFlush,
    FileNameUtf8,
    FileName(firefly_types::ValidationError),
    MenuItemUtf8,
    IdUtf8,
    Id(firefly_types::ValidationError),
    TextUtf8,
    NoneColor,
    UnknownPeer(u32),
    AudioNode(firefly_audio::NodeError),
    NoStats,
    NoBadges,
    NoBadge,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::MemoryNotFound => write!(f, "memory not found"),
            HostError::FileNotFound => write!(f, "file not found"),
            HostError::FileReadOnly => write!(f, "files in the app ROM cannot be modified"),
            HostError::OomPointer => write!(f, "buffer points out of memory"),
            HostError::FileRead => write!(f, "cannot read file"),
            HostError::FileCreate => write!(f, "cannot create file"),
            HostError::FileRemove => write!(f, "cannot remove file"),
            HostError::FileFlush => write!(f, "cannot flush file"),
            HostError::BufferSize => write!(f, "buffer size for file does not match file size"),
            HostError::FileNameUtf8 => write!(f, "file name is not valid UTF-8"),
            HostError::FileName(err) => write!(f, "bad file name: {err}"),
            HostError::MenuItemUtf8 => write!(f, "menu item name is not valid UTF-8"),
            HostError::IdUtf8 => write!(f, "ID is not valid UTF-8"),
            HostError::Id(err) => write!(f, "bad ID: {err}"),
            HostError::TextUtf8 => write!(f, "text is not valid UTF-8"),
            HostError::NoneColor => write!(f, "color is None (0)"),
            HostError::UnknownPeer(p) => write!(f, "peer {p} is not connected"),
            HostError::AudioNode(err) => write!(f, "audio node error: {err}"),
            HostError::NoStats => write!(f, "the app doesn't have stats file"),
            HostError::NoBadges => write!(f, "the app doesn't have any badges"),
            HostError::NoBadge => write!(f, "the app doesn't have a badge with the given ID"),
        }
    }
}
