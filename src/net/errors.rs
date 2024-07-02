use core::fmt;

pub(crate) enum NetcodeError {
    Serialize(postcard::Error),
    Deserialize(postcard::Error),
    Network(firefly_device::NetworkError),
    PeerListFull,
}

impl From<firefly_device::NetworkError> for NetcodeError {
    fn from(v: firefly_device::NetworkError) -> Self {
        Self::Network(v)
    }
}

impl fmt::Display for NetcodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetcodeError::Serialize(err) => write!(f, "serialization error: {err}"),
            NetcodeError::Deserialize(err) => write!(f, "deserialization error: {err}"),
            NetcodeError::Network(err) => write!(f, "network error: {err}"),
            NetcodeError::PeerListFull => write!(f, "cannot connect more devices"),
        }
    }
}
