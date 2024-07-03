use super::Connector;
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

const FONT_HEIGHT: i32 = 10;
const FONT_WIDTH: i32 = 6;
const X: i32 = 120 - 3 * 13;
const Y: i32 = 71;

pub(crate) enum ConnectStatus {
    /// Stopped listening, [Connector] should do nothing.
    Stopped,
    /// Cancelled connecting, destroy [Connector].
    Cancelled,
    /// Finished connecting, proceed to multiplayer.
    Finished,
}

struct Buttons {
    a: bool,
    b: bool,
    any: bool,
}

impl Buttons {
    fn new(input: &Option<InputState>) -> Self {
        match input {
            Some(input) => Self {
                a: input.buttons[0],
                b: input.buttons[1],
                any: input.buttons.iter().any(|x| *x),
            },
            None => Self {
                a: false,
                b: false,
                any: false,
            },
        }
    }
}

pub(crate) struct ConnectScene {
    frame: usize,
    buttons: Buttons,
    stopped: bool,
}

impl ConnectScene {
    pub fn new() -> Self {
        Self {
            frame: 0,
            buttons: Buttons::new(&None),
            stopped: false,
        }
    }

    pub fn update(&mut self, input: &Option<InputState>) -> Option<ConnectStatus> {
        let buttons = Buttons::new(input);
        let res = self.update_inner(&buttons);
        self.buttons = buttons;
        res
    }

    fn update_inner(&mut self, buttons: &Buttons) -> Option<ConnectStatus> {
        self.frame += 1;
        let new_buttons = buttons;
        let old_buttons = &self.buttons;
        // If a button is pressed, just track it and return.
        // All actions the module does happen on button release, not press.
        if new_buttons.any {
            return None;
        }

        // Connecting is not stopped, a button was pressed
        // but is released now. Stop connecting.
        if !self.stopped && old_buttons.any {
            self.stopped = true;
            return Some(ConnectStatus::Stopped);
        }

        // Connecting is stopped. The user either confirms that all
        // connected devices are good or cancels.
        if self.stopped {
            if !new_buttons.a && old_buttons.a {
                return Some(ConnectStatus::Finished);
            }
            if !new_buttons.b && old_buttons.b {
                return Some(ConnectStatus::Cancelled);
            }
        }
        None
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
        if !self.stopped {
            let gray = C::from_rgb(0x94, 0xb0, 0xc2);
            let text_style = MonoTextStyle::new(&FONT_6X9, gray);
            let point = Point::new(X, Y - FONT_HEIGHT);
            let text = "Connecting...";
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }

        // Render black "Connecting..." message on top of the gray one.
        // It is sliced over time to show that the device is not frozen.
        if !self.stopped {
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
            let text = if self.stopped {
                "press A to continue / B to cancel"
            } else {
                "(press any button to stop)"
            };
            let width = text.len() as i32 * FONT_WIDTH;
            let point = Point::new((WIDTH as i32 - width) / 2, 150);
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
        // but only if connection phase is not stopped yet.
        // If it is stopped, all peers without intro will be discarded.
        if !self.stopped {
            for (_, i) in addrs.iter().zip(peer_count..) {
                let point = Point::new(X, Y + FONT_HEIGHT * (i + 1));
                let text = Text::new("???", point, text_style);
                text.draw(display)?;
            }
        }
        Ok(())
    }
}
