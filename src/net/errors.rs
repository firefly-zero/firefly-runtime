use core::fmt;

pub(crate) enum NetcodeError {
    Serialize(postcard::Error),
    Deserialize(postcard::Error),
    Network(firefly_device::NetworkError),
    EmptyBufferIn,
    EmptyBufferOut,
    PeerListFull,
    UnknownPeer,
    FrameTimeout,
}

impl From<firefly_device::NetworkError> for NetcodeError {
    fn from(v: firefly_device::NetworkError) -> Self {
        Self::Network(v)
    }
}

impl fmt::Display for NetcodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use NetcodeError::*;
        match self {
            Serialize(err) => write!(f, "serialization error: {err}"),
            Deserialize(err) => write!(f, "deserialization error: {err}"),
            Network(err) => write!(f, "network error: {err}"),
            PeerListFull => write!(f, "cannot connect more devices"),
            EmptyBufferIn => write!(f, "received empty message"),
            EmptyBufferOut => write!(f, "serializer produced empty message"),
            UnknownPeer => write!(f, "received message from unknown peer"),
            FrameTimeout => write!(f, "timed out waiting for frame state"),
        }
    }
}
