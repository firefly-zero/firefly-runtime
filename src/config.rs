use crate::color::FromRGB;
use crate::error::Error;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::RgbColor;
use firefly_device::*;
use firefly_meta::validate_id;
use heapless::String;
use serde::{Deserialize, Serialize};

/// Contains the basic information and resources needed to run an app.
pub struct RuntimeConfig<D, C>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
{
    pub id: Option<FullID>,
    pub device: DeviceImpl,
    pub display: D,
}

/// The author and app ID combo. Must be unique. Cannot be changed.
#[derive(Clone, Serialize, Deserialize)]
pub struct FullID {
    author: String<16>,
    app: String<16>,
}

impl FullID {
    pub fn new(author: String<16>, app: String<16>) -> Self {
        Self { author, app }
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
