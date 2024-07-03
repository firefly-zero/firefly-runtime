use crate::color::FromRGB;
use crate::frame_buffer::WIDTH;
use crate::state::State;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point};
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use firefly_device::InputState;

use super::Connector;

const FONT_HEIGHT: i32 = 9;
const FONT_WIDTH: i32 = 6;
const X: i32 = 120 - 3 * 13;
const Y: i32 = 71;

pub(crate) struct ConnectScene {
    frame: usize,
    any_pressed: bool,
    stoped: bool,
}

impl ConnectScene {
    pub fn new() -> Self {
        Self {
            frame: 0,
            any_pressed: false,
            stoped: false,
        }
    }

    pub fn update(&mut self, input: &Option<InputState>) {
        self.frame += 1;
        if let Some(input) = input {
            let any_pressed = input.buttons.iter().any(|x| *x);
            if any_pressed {
                self.any_pressed = true
            } else {
                if self.any_pressed {
                    self.stoped = true
                }
                self.any_pressed = false
            }
        }
    }

    pub fn render<D, C, E>(&self, state: &State, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        let connector = state.connector.replace(None);
        let res = if let Some(connector) = &connector {
            self.render_inner(&connector, display)
        } else {
            Ok(())
        };
        state.connector.replace(connector);
        res
    }

    /// Show the connector state.
    fn render_inner<D, C, E>(&self, connector: &Connector, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        let black = C::from_rgb(0x1a, 0x1c, 0x2c);

        {
            let white = C::from_rgb(0xf4, 0xf4, 0xf4);
            display.clear(white)?;
        }

        // Render gray "Connecting..." message
        if !self.stoped {
            let gray = C::from_rgb(0x94, 0xb0, 0xc2);
            let text_style = MonoTextStyle::new(&FONT_6X9, gray);
            let point = Point::new(X, Y - FONT_HEIGHT);
            let text = "Connecting...";
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }

        // Render black "Connecting..." message on top of the gray one.
        // It is sliced over time to show that the device is not frozen.
        if !self.stoped {
            let quarter_second = self.frame / 15;
            let text_style = MonoTextStyle::new(&FONT_6X9, black);
            let text = "Connecting...";
            let (shift, text) = if quarter_second % 28 >= 14 {
                (quarter_second as i32 % 14, &text[quarter_second % 14..])
            } else {
                (0, &text[..(quarter_second % 14)])
            };
            let point = Point::new(X + shift * FONT_WIDTH, Y - FONT_HEIGHT);
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }

        // Show the current device name.
        {
            let point = Point::new(X, Y);
            let text_style = MonoTextStyle::new(&FONT_6X9, black);
            let text = Text::new("you:", point, text_style);
            text.draw(display)?;
        }
        {
            let blue = C::from_rgb(0x3b, 0x5d, 0xc9);
            let point = Point::new(X + FONT_WIDTH * 5, Y);
            let text_style = MonoTextStyle::new(&FONT_6X9, blue);
            let text = if connector.me.name.is_empty() {
                "<empty>"
            } else {
                &connector.me.name
            };
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }

        self.render_peers_list(connector, display)?;

        // Show gray "press any button to stop" at the bottom of the screen.
        {
            let gray = C::from_rgb(0x94, 0xb0, 0xc2);
            let text_style = MonoTextStyle::new(&FONT_6X9, gray);
            let text = if self.stoped {
                "press A to continue / B to cancel"
            } else {
                "(press any button to stop)"
            };
            let width = text.len() as i32 * FONT_WIDTH;
            let point = Point::new((WIDTH as i32 - width) / 2, 140);
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }

        Ok(())
    }

    /// Show the list of connected peers.
    fn render_peers_list<D, C, E>(&self, connector: &Connector, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        let blue = C::from_rgb(0x3b, 0x5d, 0xc9);
        let text_style = MonoTextStyle::new(&FONT_6X9, blue);
        let mut addrs = connector.peer_addrs().clone();
        let peers = connector.peer_infos();
        let peer_count = peers.len() as i32;
        for (peer, i) in connector.peer_infos().iter().zip(0..) {
            addrs.retain(|addr| *addr != peer.addr);
            let point = Point::new(X, Y + FONT_HEIGHT + FONT_HEIGHT * i);
            let text = if peer.name.is_empty() {
                "<empty>"
            } else {
                &peer.name
            };
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }
        // Show peers that are advertised but haven't sent intro yet
        // but only if connection phase is not stoped yet.
        // If it is stoped, all peers without intro will be discarded.
        if !self.stoped {
            for (_, i) in addrs.iter().zip(peer_count..) {
                let point = Point::new(X, Y + 9 * i);
                let text = Text::new("???", point, text_style);
                text.draw(display)?;
            }
        }
        Ok(())
    }
}
