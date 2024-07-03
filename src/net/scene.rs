use crate::color::FromRGB;
use crate::state::State;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point};
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;

pub(crate) struct ConnectScene {
    frame: usize,
}

impl ConnectScene {
    pub fn new() -> Self {
        Self { frame: 0 }
    }

    pub fn update(&mut self) {
        self.frame += 1;
    }

    pub fn render<D, C, E>(&self, state: &State, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        let quarter_second = self.frame / 15;
        let white = C::from_rgb(0xf4, 0xf4, 0xf4);
        let gray = C::from_rgb(0x56, 0x6c, 0x86);
        let black = C::from_rgb(0x1a, 0x1c, 0x2c);
        let blue = C::from_rgb(0x3b, 0x5d, 0xc9);
        display.clear(white)?;
        let point = Point::new(120 - 3 * 13, 80 - 9);

        let text_style = MonoTextStyle::new(&FONT_6X9, gray);
        let text = "Connecting...";
        let text = Text::new(text, point, text_style);
        text.draw(display)?;

        let text_style = MonoTextStyle::new(&FONT_6X9, black);
        let text = "Connecting...";
        let text = &text[..(quarter_second % 13) + 1];
        let text = Text::new(text, point, text_style);
        text.draw(display)?;

        let connector = state.connector.replace(None);
        if let Some(connector) = &connector {
            let text_style = MonoTextStyle::new(&FONT_6X9, blue);
            let mut addrs = connector.peer_addrs().clone();
            let peers = connector.peer_infos();
            let peer_count = peers.len() as i32;
            for (peer, i) in connector.peer_infos().iter().zip(0..) {
                addrs.retain(|addr| *addr != peer.addr);
                let point = Point::new(120 - 3 * 13, 80 + 9 * i);
                let text = if peer.name.is_empty() {
                    "<empty>"
                } else {
                    &peer.name
                };
                let text = Text::new(text, point, text_style);
                text.draw(display)?;
            }
            for (_, i) in addrs.iter().zip(peer_count..) {
                let point = Point::new(120 - 3 * 13, 80 + 9 * i);
                let text = Text::new("???", point, text_style);
                text.draw(display)?;
            }
        }
        state.connector.replace(connector);

        Ok(())
    }
}
