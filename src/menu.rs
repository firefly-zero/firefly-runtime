use crate::color::FromRGB;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point, Size};
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle, StyledDrawable,
};
use embedded_graphics::text::Text;
use firefly_device::InputState;

const LINE_HEIGHT: i32 = 12;

enum MenuItem {
    Connect,
    Quit,
    ScreenShot,
}

impl MenuItem {
    fn as_str(&self) -> &str {
        match self {
            MenuItem::Connect => "start multiplayer",
            MenuItem::ScreenShot => "take screenshot",
            MenuItem::Quit => "exit app",
        }
    }
}

pub(crate) struct Menu {
    /// System menu items.
    items: heapless::Vec<MenuItem, 3>,

    selected: i32,

    /// True if the menu should be currently shown.
    active: bool,

    /// True if the menu is currently rendered on the screen.
    rendered: bool,

    /// True if the menu button is currently pressed.
    is_pressed: bool,
}

impl Menu {
    pub fn new() -> Self {
        let mut items = heapless::Vec::new();
        _ = items.push(MenuItem::Connect);
        _ = items.push(MenuItem::ScreenShot);
        _ = items.push(MenuItem::Quit);
        Self {
            items,
            selected: 0,
            active: false,
            rendered: false,
            is_pressed: false,
        }
    }

    pub fn handle_input(&mut self, input: &Option<InputState>) {
        let is_pressed = matches!(
            input,
            Some(InputState {
                pad:     _,
                buttons: [_, _, _, _, true],
            })
        );
        // the menu button wasn't pressed but is pressed now.
        if !self.is_pressed && is_pressed {
            self.active = !self.active;
            if self.active {
                self.rendered = false;
            }
        }
        self.is_pressed = is_pressed;
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn render<D, C, E>(&self, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        if self.rendered {
            return Ok(());
        }
        let corners = CornerRadii::new(Size::new_equal(4));
        let white = C::from_rgb(0xf4, 0xf4, 0xf4);
        let black = C::from_rgb(0x1a, 0x1c, 0x2c);
        let mut box_style = PrimitiveStyle::new();
        box_style.stroke_color = Some(black);
        box_style.stroke_width = 1;
        let text_style = MonoTextStyle::new(&FONT_6X9, black);

        display.clear(white)?;
        for (item, i) in self.items.iter().zip(0..) {
            let point = Point::new(6, 9 + i * LINE_HEIGHT);
            let text = Text::new(item.as_str(), point, text_style);
            text.draw(display)?;

            if i == self.selected {
                let point = Point::new(3, 2 + i * LINE_HEIGHT);
                let rect = Rectangle::new(point, Size::new(232, LINE_HEIGHT as u32));
                let rect = RoundedRectangle::new(rect, corners);
                rect.draw_styled(&box_style, display)?;
            }
        }
        Ok(())
    }
}
