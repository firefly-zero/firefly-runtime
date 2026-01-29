use crate::color::FromRGB;
use crate::error::Error;
use crate::state::NetHandler;
use core::fmt;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::RgbColor;
use firefly_hal::*;
use firefly_types::validate_id;
use heapless::String;
use serde::{Deserialize, Serialize};

/// Contains the basic information and resources needed to run an app.
pub struct RuntimeConfig<'a, D, C>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
{
    pub id: Option<FullID>,
    pub device: DeviceImpl<'a>,
    pub display: D,
    pub net_handler: NetHandler<'a>,
}

pub enum FullIDError {
    NoDot,
    LongAuthor,
    LongApp,
}

impl FullIDError {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NoDot => "the full app ID must contain a dot",
            Self::LongAuthor => "author ID is too long",
            Self::LongApp => "app ID is too long",
        }
    }
}

impl fmt::Display for FullIDError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The author and app ID combo. Must be unique. Cannot be changed.
#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct FullID {
    author: String<16>,
    app: String<16>,
}

impl FullID {
    pub fn new(author: String<16>, app: String<16>) -> Self {
        Self { author, app }
    }

    pub fn from_str(author: &str, app: &str) -> Option<Self> {
        let Ok(author) = String::try_from(author) else {
            return None;
        };
        let Ok(app) = String::try_from(app) else {
            return None;
        };
        Some(Self { author, app })
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn app(&self) -> &str {
        &self.app
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if let Err(err) = validate_id(&self.author) {
            return Err(Error::InvalidAuthorID(err));
        }
        if let Err(err) = validate_id(&self.app) {
            return Err(Error::InvalidAppID(err));
        }
        Ok(())
    }
}

impl TryFrom<&str> for FullID {
    type Error = FullIDError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let Some(dot) = value.find('.') else {
            return Err(FullIDError::NoDot);
        };
        let (author_id, app_id) = value.split_at(dot);
        let Ok(author_id) = heapless::String::try_from(author_id) else {
            return Err(FullIDError::LongAuthor);
        };
        let Ok(app_id) = heapless::String::try_from(&app_id[1..]) else {
            return Err(FullIDError::LongApp);
        };
        Ok(Self::new(author_id, app_id))
    }
}
