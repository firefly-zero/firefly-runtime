use crate::linking::LinkingError;
use core::fmt;

pub enum Error {
    Wasmi(wasmi::Error),
    FuncCall(&'static str, wasmi::Error, RuntimeStats),
    FileEmpty(&'static str),
    OpenDir(alloc::string::String, firefly_hal::FSError),
    OpenFile(&'static str, firefly_hal::FSError),
    ReadFile(&'static str, firefly_hal::FSError),
    NoLauncher,
    InvalidAuthorID(firefly_types::ValidationError),
    InvalidAppID(firefly_types::ValidationError),
    CannotDisplay,
    AuthorIDMismatch,
    AppIDMismatch,

    Linking(LinkingError),

    DecodeMeta(postcard::Error),
    DecodeStats(postcard::Error),
    SerialEncode(postcard::Error),
    SerialDecode(postcard::Error),

    SerialStart(firefly_hal::NetworkError),
    SerialSend(firefly_hal::NetworkError),
    SerialRecv(firefly_hal::NetworkError),

    CheatUndefined,
    CheatInNet,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wasmi(err) => {
                write!(f, "wasm error: ")?;
                use wasmi::errors::ErrorKind::*;
                match err.kind() {
                    TrapCode(_) => write!(f, "trap code: {err}"),
                    Message(_) => write!(f, "message: {err}"),
                    I32ExitStatus(_) => write!(f, "exit status: {err}"),
                    Host(_) => write!(f, "host: {err}"),
                    Global(_) => write!(f, "global: {err}"),
                    Memory(_) => write!(f, "memory: {err}"),
                    Table(_) => write!(f, "table: {err}"),
                    Linker(_) => write!(f, "linker: {err}"),
                    Instantiation(_) => write!(f, "instantiation: {err}"),
                    Fuel(_) => write!(f, "fuel: {err}"),
                    Func(_) => write!(f, "func: {err}"),
                    Read(_) => write!(f, "read: {err}"),
                    Wasm(_) => write!(f, "parse: {err}"),
                    Translation(_) => write!(f, "translation: {err}"),
                    Limits(_) => write!(f, "limits: {err}"),
                    Ir(_) => write!(f, "IR: {err}"),
                    _ => write!(f, "unknown: {err}"),
                }
            }
            Self::FileEmpty(s) => write!(f, "file is empty: {s}"),
            Self::OpenDir(s, e) => write!(f, "cannot open {s} dir: {e}"),
            Self::OpenFile(s, e) => write!(f, "cannot open {s} file: {e}"),
            Self::NoLauncher => write!(f, "no launcher installed"),
            Self::FuncCall(func, err, stats) => write!(f, "error calling {func}: {err}.\n{stats}"),
            Self::InvalidAuthorID(err) => write!(f, "invalid author ID: {err}"),
            Self::InvalidAppID(err) => write!(f, "invalid app ID: {err}"),
            Self::CannotDisplay => write!(f, "failed to draw on the display"),
            Self::ReadFile(name, err) => write!(f, "cannot read {name}: {err}"),
            Self::AuthorIDMismatch => write!(f, "author ID in meta and in path don't match"),
            Self::AppIDMismatch => write!(f, "app ID in meta and in path don't match"),
            Self::Linking(err) => write!(f, "linking: {err}"),
            Self::DecodeMeta(err) => write!(f, "cannot decode _meta: {err}"),
            Self::DecodeStats(err) => write!(f, "cannot decode stats: {err}"),
            Self::SerialEncode(err) => write!(f, "cannot encode response for serial: {err}"),
            Self::SerialDecode(err) => write!(f, "cannot decode request from serial: {err}"),
            Self::SerialStart(err) => write!(f, "cannot connect to serial port: {err}"),
            Self::SerialSend(err) => write!(f, "cannot send into serial port: {err}"),
            Self::SerialRecv(err) => write!(f, "cannot read from serial port: {err}"),
            Self::CheatUndefined => write!(f, "the app doesn't have cheat callback"),
            Self::CheatInNet => write!(f, "cheats are disabled in multiplayer"),
        }
    }
}

impl From<wasmi::Error> for Error {
    fn from(value: wasmi::Error) -> Self {
        Self::Wasmi(value)
    }
}

impl From<LinkingError> for Error {
    fn from(value: LinkingError) -> Self {
        Self::Linking(value)
    }
}

/// Runtime stats provided on guest failure that should help to debug the failure cause.
pub struct RuntimeStats {
    pub(crate) last_called: &'static str,
}

impl fmt::Display for RuntimeStats {
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
    DataFileInNet,
    FileReadOnly,
    FileNotFound,
    FileRead(firefly_hal::FSError),
    FileWrite,
    FileFlush,
    FileNameUtf8,
    FileName(firefly_types::ValidationError),
    MenuItemUtf8,
    IdUtf8,
    Id(firefly_types::ValidationError),
    TextUtf8,
    InvalidWidth,
    NoneColor,
    UnknownPeer(u32),
    AudioNode(firefly_audio::NodeError),
    NoStats,
    NoBadges,
    NoBadge(u32),
    NoBoards,
    NoBoard(u32),
    ValueTooBig,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemoryNotFound => write!(f, "memory not found"),
            Self::FileReadOnly => write!(f, "files in the app ROM cannot be modified"),
            Self::DataFileInNet => write!(f, "cannot read data files in multiplayer"),
            Self::OomPointer => write!(f, "buffer points out of memory"),
            Self::FileRead(e) => write!(f, "cannot read file: {e}"),
            Self::FileWrite => write!(f, "cannot write file"),
            Self::FileNotFound => write!(f, "file not found"),
            Self::FileFlush => write!(f, "cannot flush file"),
            Self::BufferSize => write!(f, "buffer size for file does not match file size"),
            Self::FileNameUtf8 => write!(f, "file name is not valid UTF-8"),
            Self::FileName(err) => write!(f, "bad file name: {err}"),
            Self::MenuItemUtf8 => write!(f, "menu item name is not valid UTF-8"),
            Self::IdUtf8 => write!(f, "ID is not valid UTF-8"),
            Self::Id(err) => write!(f, "bad ID: {err}"),
            Self::TextUtf8 => write!(f, "text is not valid UTF-8"),
            Self::InvalidWidth => write!(f, "the image has invalid width"),
            Self::NoneColor => write!(f, "color is None (0)"),
            Self::UnknownPeer(p) => write!(f, "peer {p} is not connected"),
            Self::AudioNode(err) => write!(f, "audio node error: {err}"),
            Self::NoStats => write!(f, "the app doesn't have stats file"),
            Self::NoBadges => write!(f, "the app doesn't have any badges"),
            Self::NoBadge(id) => write!(f, "the app doesn't have a badge with ID {id}"),
            Self::NoBoards => write!(f, "the app doesn't have any boards"),
            Self::NoBoard(id) => write!(f, "the app doesn't have a board with ID {id}"),
            Self::ValueTooBig => write!(f, "the value is too big"),
        }
    }
}
