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
use firefly_hal::InputState;

const LINE_HEIGHT: i32 = 12;

pub(crate) enum MenuItem {
    Custom(u8, alloc::string::String),
    Connect,
    Disconnect,
    ScreenShot,
    Restart,
    Quit,
}

impl MenuItem {
    fn as_str(&self) -> &str {
        match self {
            MenuItem::Custom(_, t) => t,
            MenuItem::Connect => "start multiplayer",
            MenuItem::Disconnect => "exit multiplayer",
            MenuItem::ScreenShot => "take screenshot",
            MenuItem::Restart => "restart app",
            MenuItem::Quit => "exit app",
        }
    }
}

#[derive(Default)]
pub(crate) struct Menu {
    /// Custom menu items.
    app_items: alloc::vec::Vec<MenuItem>,

    /// System menu items.
    sys_items: heapless::Vec<MenuItem, 4>,

    selected: i32,

    /// True if the menu should be currently shown.
    active: bool,

    /// True if the menu is currently rendered on the screen.
    rendered: bool,

    /// True if the menu button is currently pressed.
    menu_pressed: bool,

    /// True if the selection button (A) is currently pressed.
    select_pressed: bool,

    /// True if the menu button was released when the menu was open.
    was_released: bool,

    down_pressed: bool,
    up_pressed: bool,
}

impl Menu {
    pub fn new(offline: bool, launcher: bool) -> Self {
        let mut items = heapless::Vec::new();
        if launcher {
            if offline {
                _ = items.push(MenuItem::Connect);
            } else {
                _ = items.push(MenuItem::Disconnect);
            }
        }
        _ = items.push(MenuItem::ScreenShot);
        if !launcher {
            _ = items.push(MenuItem::Restart);
            _ = items.push(MenuItem::Quit);
        }
        Self {
            app_items: alloc::vec::Vec::new(),
            sys_items: items,
            ..Default::default()
        }
    }

    /// Add a custom menu item.
    pub(crate) fn add(&mut self, index: u8, name: alloc::string::String) {
        self.app_items.push(MenuItem::Custom(index, name));
    }

    /// Remove a custom menu item.
    pub(crate) fn remove(&mut self, index: u8) {
        self.app_items
            .retain(|item| !matches!(item, MenuItem::Custom(i, _) if *i == index));
    }

    pub fn handle_input(&mut self, input: &Option<InputState>) -> Option<&MenuItem> {
        let def = InputState::default();
        let input = match input {
            Some(input) => input,
            None => &def,
        };
        self.handle_menu_button(input.menu());
        if !self.active {
            return None;
        }
        self.handle_pad(input);
        self.handle_select(input.a())
    }

    fn handle_menu_button(&mut self, pressed: bool) {
        // Depending on if menu is open or not, handle the menu button in a way
        // that the button is always released when the app is running.
        if self.active {
            // When menu is open, close it on releasing the menu button.
            if self.was_released && self.menu_pressed && !pressed {
                self.active = false;
            }
            if !pressed {
                self.was_released = true;
            }
        } else {
            // When menu is closed, open it on pressing the menu button.
            #[allow(clippy::collapsible_else_if)]
            if !self.menu_pressed && pressed {
                self.active = true;
                self.rendered = false;
                self.was_released = false;
            }
        }
        self.menu_pressed = pressed;
    }

    fn handle_pad(&mut self, input: &InputState) {
        let Some(pad) = &input.pad else {
            self.down_pressed = false;
            self.up_pressed = false;
            return;
        };
        if pad.y < -50 {
            self.down_pressed = false;
            let n_items = self.app_items.len() + self.sys_items.len();
            if !self.up_pressed && self.selected < n_items as i32 - 1 {
                self.selected += 1;
                self.rendered = false;
            }
            self.up_pressed = true;
        }
        if pad.y > 50 {
            self.up_pressed = false;
            if !self.down_pressed && self.selected > 0 {
                self.selected -= 1;
                self.rendered = false;
            }
            self.down_pressed = true;
        }
    }

    fn handle_select(&mut self, pressed: bool) -> Option<&MenuItem> {
        if self.select_pressed {
            if !pressed {
                self.select_pressed = false;
                let selected = self.selected as usize;
                // Close menu and return control to the game
                self.active = false;
                if let Some(item) = self.app_items.get(selected) {
                    return Some(item);
                }
                let selected = selected - self.app_items.len();
                return self.sys_items.get(selected);
            }
        } else {
            self.select_pressed = pressed;
        }
        None
    }

    /// True if the menu should be currently shown.
    ///
    /// While it is true, the app is paused.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Open the menu (if closed).
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Close the menu (if open).
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn render<D, C, E>(&mut self, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E> + OriginDimensions,
        C: RgbColor + FromRGB,
    {
        if self.rendered {
            return Ok(());
        }
        self.rendered = true;
        let corners = CornerRadii::new(Size::new_equal(4));
        let box_style = PrimitiveStyle::with_stroke(C::PRIMARY, 1);
        let mut black_style = MonoTextStyle::new(&FONT_6X9, C::PRIMARY);
        black_style.background_color = Some(C::BG);
        let mut blue_style = MonoTextStyle::new(&FONT_6X9, C::ACCENT);
        blue_style.background_color = Some(C::BG);

        display.clear(C::BG)?;
        let items = self.app_items.iter().chain(self.sys_items.iter());
        for (item, i) in items.zip(0..) {
            let point = Point::new(6, 9 + i * LINE_HEIGHT);
            let text_style = match item {
                MenuItem::Custom(_, _) => blue_style,
                _ => black_style,
            };
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
