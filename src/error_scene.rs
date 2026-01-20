use crate::color::FromRGB;
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle, StyledDrawable,
};
use embedded_graphics::text::Text;
use firefly_hal::{Device, DeviceImpl, Duration, Instant};

const FONT_HEIGHT: i32 = 10;
const FONT_WIDTH: i32 = 6;
const CENTER: Point = Point::new(240 / 2, 160 / 2);
const BTN_DELAY: Duration = Duration::from_ms(500);

/// An alert popup window showing an error message.
pub(crate) struct ErrorScene {
    msg: alloc::string::String,
    start: Option<Instant>,
    showed_msg: bool,
    showed_btn: bool,
    enabled_btn: bool,
    buttons: u8,
}

impl ErrorScene {
    pub fn new(msg: alloc::string::String) -> Self {
        Self {
            msg,
            start: None,
            showed_msg: false,
            showed_btn: false,
            enabled_btn: false,
            buttons: 0,
        }
    }

    pub fn update(&mut self, device: &mut DeviceImpl) -> bool {
        // Check if the confirmation button is active.
        if !self.enabled_btn {
            let now = device.now();
            let start = match self.start {
                Some(start) => start,
                None => {
                    self.start = Some(now);
                    now
                }
            };
            if now - start > BTN_DELAY {
                self.enabled_btn = true;
                self.showed_btn = false;
            }
        }

        // If the button is active, check if the user pressed and released it.
        if self.enabled_btn {
            let buttons = match device.read_input() {
                Some(input) => input.buttons,
                None => 0u8,
            };
            let buttons = buttons & 0b11111;
            if self.buttons != 0 && buttons == 0 {
                return true;
            }
            self.buttons = buttons
        }
        false
    }

    pub fn render<D, C, E>(&mut self, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        if !self.showed_msg {
            display.clear(C::BG)?;
            let mut text_style = MonoTextStyle::new(&FONT_6X9, C::PRIMARY);
            text_style.background_color = Some(C::BG);
            wrap_text(&mut self.msg);
            let line_width = self
                .msg
                .lines()
                .map(|line| line.len())
                .max()
                .unwrap_or_default();
            let x_shift = FONT_WIDTH * line_width as i32 / 2;
            let point = Point::new(CENTER.x - x_shift, 71 - FONT_HEIGHT);
            let text = Text::new(&self.msg, point, text_style);
            text.draw(display)?;
            self.showed_msg = true;
        }

        if !self.showed_btn {
            let color = if self.enabled_btn {
                C::ACCENT
            } else {
                C::MUTED
            };

            let text = "oh no!";
            let x_shift = (FONT_WIDTH / 2) * text.len() as i32;
            let point = Point::new(CENTER.x - x_shift, 120 - FONT_HEIGHT);

            {
                let point = Point::new(point.x - 2, point.y - 8);
                let mut box_style = PrimitiveStyle::with_stroke(color, 1);
                box_style.fill_color = Some(C::BG);
                let corners = CornerRadii::new(Size::new_equal(4));
                let size = Size {
                    width: (text.len() as i32 * FONT_WIDTH) as u32 + 4,
                    height: FONT_HEIGHT as u32 + 4,
                };
                let rect = RoundedRectangle::new(Rectangle::new(point, size), corners);
                rect.draw_styled(&box_style, display)?;
            }

            let mut text_style = MonoTextStyle::new(&FONT_6X9, color);
            text_style.background_color = Some(C::BG);
            let text = Text::new(text, point, text_style);
            text.draw(display)?;
            self.showed_btn = true;
        }
        Ok(())
    }
}

/// Split long lines of text into several lines.
fn wrap_text(text: &mut str) {
    let bytes = unsafe { text.as_bytes_mut() };
    let mut i = 0;
    for char in bytes {
        i += 1;
        if i > 20 && *char == b' ' {
            *char = b'\n'
        }
        if *char == b'\n' {
            i = 0;
        }
    }
}
