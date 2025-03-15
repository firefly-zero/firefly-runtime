use super::Connector;
use crate::color::FromRGB;
use crate::frame_buffer::WIDTH;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point};
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use firefly_hal::InputState;

const FONT_HEIGHT: i32 = 10;
const FONT_WIDTH: i32 = 6;
const X: i32 = 120 - 3 * 13;
const Y: i32 = 71;

#[derive(PartialEq)]
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
                a: input.a(),
                b: input.b(),
                any: (input.buttons & 0b1111) != 0,
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
    /// The hash of the scene state. If hash changes, scene needs to be re-rendered.
    hash: usize,
    /// True if the scene was rendered at least once.
    ///
    /// We also set it to false when the scan for device is stopped
    /// to force clearing the screen. It's needed because the scene
    /// in these states is slightly different.
    rendered: bool,
}

impl ConnectScene {
    pub fn new() -> Self {
        Self {
            frame: 0,
            buttons: Buttons::new(&None),
            stopped: false,
            hash: 0,
            rendered: false,
        }
    }

    /// Force the scene to redraw the UI.
    pub fn redraw(&mut self) {
        self.rendered = false;
        self.hash = 0;
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
            self.rendered = false;
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

    pub fn render<D, C, E>(&mut self, connector: &Connector, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        let hash = self.calculate_hash(connector);
        if self.hash == hash {
            return Ok(());
        }
        self.hash = hash;
        let res = self.render_inner(connector, display);
        self.rendered = true;
        res
    }

    /// Show the connector state.
    fn render_inner<D, C, E>(&self, connector: &Connector, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        if !self.rendered {
            display.clear(C::BG)?;
        }

        // Render gray "Connecting..." message
        if !self.stopped {
            let mut text_style = MonoTextStyle::new(&FONT_6X9, C::MUTED);
            text_style.background_color = Some(C::BG);
            let point = Point::new(X, Y - FONT_HEIGHT);
            let text = "Connecting...";
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }

        // Render black "Connecting..." message on top of the gray one.
        // It is sliced over time to show that the device is not frozen.
        if !self.stopped {
            let quarter_second = self.frame / 15;
            let mut text_style = MonoTextStyle::new(&FONT_6X9, C::PRIMARY);
            text_style.background_color = Some(C::BG);
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
            let mut text_style = MonoTextStyle::new(&FONT_6X9, C::PRIMARY);
            text_style.background_color = Some(C::BG);
            let text = Text::new("you:", point, text_style);
            text.draw(display)?;
        }
        {
            let point = Point::new(X + FONT_WIDTH * 5, Y);
            let mut text_style = MonoTextStyle::new(&FONT_6X9, C::ACCENT);
            text_style.background_color = Some(C::BG);
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
            let mut text_style = MonoTextStyle::new(&FONT_6X9, C::MUTED);
            text_style.background_color = Some(C::BG);
            let text = if self.stopped {
                if connector.peer_infos().is_empty() {
                    "press E to cancel"
                } else {
                    "press S to continue / E to cancel"
                }
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
        let mut text_style = MonoTextStyle::new(&FONT_6X9, C::ACCENT);
        text_style.background_color = Some(C::BG);
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

        // Draw empty rectangle at the end to hide devices
        // that were connected but now are disconnected.
        {
            let line = connector.peer_addrs().len() as i32 + 1;
            let point = Point::new(X, Y + FONT_HEIGHT * line);
            let text = "                ";
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
        }

        Ok(())
    }

    /// Calculate the state hash.
    ///
    /// If the hash has changed, we need to re-render the UI.
    fn calculate_hash(&self, connector: &Connector) -> usize {
        let mut hash = 0;
        hash += connector.peer_addrs().len();
        hash += connector.peer_infos().len();
        hash += self.frame / 15;
        hash += usize::from(self.buttons.any) * 32;
        hash += usize::from(self.buttons.a) * 64;
        hash += usize::from(self.buttons.a) * 128;
        hash
    }
}
