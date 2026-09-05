use crate::battery::Battery;
use crate::color::FromRGB;
use crate::net::FrameSyncer;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle, StyledDrawable, Triangle,
};
use embedded_graphics::text::Text;
use firefly_hal::{InputState, Pad};

const LINE_HEIGHT: i32 = 12;
const OFFSET: i32 = 20;

pub(crate) enum MenuItem {
    Custom(u8, alloc::string::String),
    ScreenShot,
    Restart,
    Quit,
}

impl MenuItem {
    fn as_str(&self) -> &str {
        match self {
            Self::Custom(_, t) => t,
            Self::ScreenShot => "take screenshot",
            Self::Restart => "restart app",
            Self::Quit => "exit app",
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum DPad4 {
    None,
    Left,
    Right,
    Up,
    Down,
}

impl DPad4 {
    fn from_pad(pad: Option<&Pad>) -> Self {
        const DPAD4_THRESHOLD: i16 = 300;
        let Some(pad) = pad else {
            return Self::None;
        };
        let x = pad.x;
        let y = pad.y;
        if y > DPAD4_THRESHOLD && y > x.abs() {
            Self::Up
        } else if y < -DPAD4_THRESHOLD && -y > x.abs() {
            Self::Down
        } else if x > DPAD4_THRESHOLD && x > y.abs() {
            Self::Right
        } else if x < -DPAD4_THRESHOLD && -x > y.abs() {
            Self::Left
        } else {
            Self::None
        }
    }
}

pub(crate) struct Menu {
    /// Custom menu items.
    app_items: alloc::vec::Vec<MenuItem>,

    /// System menu items.
    sys_items: heapless::Vec<MenuItem, 3>,

    /// For how many frames the menu was open.
    pub frames: u32,

    /// The currently focused menu item.
    selected: i32,

    /// The index of the peer (in [`FrameSyncer::peers`]) that activated the menu.
    actor_idx: u8,

    /// Packed boolean flags.
    flags: u8,

    dpad: DPad4,
}

impl Menu {
    pub fn new() -> Self {
        let mut items = heapless::Vec::<_, 3>::new();
        unsafe {
            items.push_unchecked(MenuItem::ScreenShot);
            items.push_unchecked(MenuItem::Restart);
            items.push_unchecked(MenuItem::Quit);
        }
        let mut menu = Self {
            app_items: alloc::vec::Vec::new(),
            sys_items: items,
            frames: 0,
            selected: 0,
            actor_idx: 0,
            flags: 0,
            dpad: DPad4::None,
        };
        menu.set_dirty(true);
        menu
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
        let input = input.as_ref().unwrap_or(&def);
        self.handle_menu_button(input.menu());
        if !self.active() {
            return None;
        }
        self.frames += 1;
        self.handle_pad(input);
        self.handle_select(input.s() || input.e())
    }

    pub fn handle_net_input(&mut self, syncer: &FrameSyncer) -> Option<&MenuItem> {
        let mut menu_pressed = false;
        let mut actor = false;
        let mut actor_idx = 0;
        for (peer, i) in syncer.peers.iter().zip(0..) {
            let Some(state) = peer.states.get_current() else {
                continue;
            };
            let peer_menu = state.input.buttons & 0b1_0000 != 0;
            if peer_menu {
                menu_pressed = true;
                actor = peer.addr.is_none();
                actor_idx = i;
                break;
            }
        }

        let was_active = self.active();
        self.handle_menu_button(menu_pressed);
        if !self.active() {
            return None;
        }
        if !was_active {
            self.set_actor(actor);
            self.actor_idx = actor_idx;
        }

        let peer = syncer.peers.get(usize::from(self.actor_idx))?;
        let state = peer.states.get_current()?;
        let input: InputState = state.input.into();
        self.handle_pad(&input);
        self.handle_select(input.s() || input.e())
    }

    fn handle_menu_button(&mut self, pressed: bool) {
        let was_pressed = self.menu_pressed();
        self.set_menu_pressed(pressed);

        // When menu is open, close it on releasing the menu button.
        if self.active() {
            if !pressed {
                if self.was_released() && was_pressed {
                    self.deactivate();
                }
                self.set_was_released(true);
            }
            return;
        }

        // When menu is closed, open it on pressing the menu button.
        if !was_pressed && pressed {
            self.activate();
            self.set_was_released(false);
        }
    }

    /// Open the menu.
    pub fn activate(&mut self) {
        self.flags |= MASK_ACTIVE;
        self.set_rendered(false);
        self.set_dirty(true);
    }

    fn handle_pad(&mut self, input: &InputState) {
        let dpad = DPad4::from_pad(input.pad.as_ref());
        let pressed = if dpad != self.dpad { dpad } else { DPad4::None };
        self.dpad = dpad;
        match pressed {
            DPad4::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.set_dirty(true);
                }
            }
            DPad4::Down => {
                let n_items = self.n_items();
                if self.selected < n_items as i32 - 1 {
                    self.selected += 1;
                    self.set_dirty(true);
                }
            }
            DPad4::Left => {
                if self.selected > 0 {
                    self.selected = 0;
                    self.set_dirty(true);
                }
            }
            DPad4::Right => {
                let n_items = self.n_items();
                if self.selected < n_items as i32 - 1 {
                    self.selected = n_items as i32 - 1;
                    self.set_dirty(true);
                }
            }
            DPad4::None => {}
        }
    }

    fn handle_select(&mut self, pressed: bool) -> Option<&MenuItem> {
        if self.select_pressed() {
            if !pressed {
                self.set_select_pressed(false);
                let selected = self.selected as usize;
                self.deactivate();
                if let Some(item) = self.app_items.get(selected) {
                    return Some(item);
                }
                let selected = selected - self.app_items.len();
                return self.sys_items.get(selected);
            }
        } else {
            self.set_select_pressed(pressed);
        }
        None
    }

    fn n_items(&self) -> usize {
        self.app_items.len() + self.sys_items.len()
    }

    pub fn render<D, C, E>(
        &mut self,
        display: &mut D,
        battery: &mut Option<Battery>,
    ) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E>,
        C: RgbColor + FromRGB,
        E: core::fmt::Debug,
    {
        if self.rendered() && !self.dirty() {
            return Ok(());
        }
        if !self.rendered() {
            self.draw_bg(display)?;
        }
        self.set_rendered(true);
        self.set_dirty(false);

        if self.actor() {
            self.draw_items(display)?;
            self.draw_cursor(display, C::PRIMARY, self.selected)?;
        } else {
            self.draw_paused(display)?;
        }

        self.draw_battery(display, battery)
    }

    pub fn draw_bg<D, C, E>(&self, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E>,
        C: RgbColor + FromRGB,
    {
        let top_left = Point::new(OFFSET, OFFSET);
        let width = 240 - OFFSET as u32 * 2;
        let height = 160 - OFFSET as u32 * 2;
        let size = Size::new(width, height);
        let area = Rectangle::new(top_left, size);
        display.fill_solid(&area, C::BG)?;

        // Top border.
        let top_left = Point::new(OFFSET - 1, OFFSET - 1);
        let size = Size::new(width + 2, 1);
        let area = Rectangle::new(top_left, size);
        display.fill_solid(&area, C::PRIMARY)?;

        // Bottom border.
        let top_left = Point::new(OFFSET, 160 - OFFSET);
        let size = Size::new(width + 2, 1);
        let area = Rectangle::new(top_left, size);
        display.fill_solid(&area, C::PRIMARY)?;
        let top_left = Point::new(OFFSET, 160 - OFFSET + 1);
        let area = Rectangle::new(top_left, size);
        display.fill_solid(&area, C::PRIMARY)?;

        // Left border.
        let top_left = Point::new(OFFSET - 1, OFFSET);
        let size = Size::new(1, height + 1);
        let area = Rectangle::new(top_left, size);
        display.fill_solid(&area, C::PRIMARY)?;

        // Right border.
        let top_left = Point::new(240 - OFFSET, OFFSET);
        let size = Size::new(2, height);
        let area = Rectangle::new(top_left, size);
        display.fill_solid(&area, C::PRIMARY)?;

        Ok(())
    }

    pub fn draw_paused<D, C, E>(&self, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E>,
        C: RgbColor + FromRGB,
    {
        let mut style = MonoTextStyle::new(&FONT_6X9, C::ACCENT);
        style.background_color = Some(C::BG);
        let text = "PAUSED";
        let point = Point::new((240 - 6 * 6) / 2, 75);
        let text = Text::new(text, point, style);
        text.draw(display)?;

        let mut style = MonoTextStyle::new(&FONT_6X9, C::MUTED);
        style.background_color = Some(C::BG);
        let text = "by another player";
        let point = Point::new((240 - 17 * 6) / 2, 85);
        let text = Text::new(text, point, style);
        text.draw(display)?;

        Ok(())
    }

    pub fn draw_items<D, C, E>(&self, display: &mut D) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E>,
        C: RgbColor + FromRGB,
    {
        let mut black_style = MonoTextStyle::new(&FONT_6X9, C::PRIMARY);
        black_style.background_color = Some(C::BG);

        // Draw the list of custom items.
        let offset_x = OFFSET + 6;
        let mut offset_y = OFFSET + 9;
        for (item, i) in self.app_items.iter().zip(0..) {
            if i != self.selected {
                self.draw_cursor(display, C::BG, i)?;
            };
            let point = Point::new(offset_x, offset_y + i * LINE_HEIGHT);
            let text = Text::new(item.as_str(), point, black_style);
            text.draw(display)?;
        }

        // Draw the list of system items.
        let n_custom = self.app_items.len() as i32;
        if n_custom != 0 {
            offset_y += 4;
        }
        for (item, i) in self.sys_items.iter().zip(n_custom..) {
            if i != self.selected {
                self.draw_cursor(display, C::BG, i)?;
            };
            let point = Point::new(offset_x, offset_y + i * LINE_HEIGHT);
            let text = Text::new(item.as_str(), point, black_style);
            text.draw(display)?;
        }

        // Draw the separator line.
        if n_custom != 0 {
            let top_left = Point::new(OFFSET, n_custom * LINE_HEIGHT + 4 + OFFSET);
            let size = Size::new(240 - OFFSET as u32 * 2, 1);
            let area = Rectangle::new(top_left, size);
            display.fill_solid(&area, C::PRIMARY)?;
        }
        Ok(())
    }

    /// Indicate which item is currently selected.
    pub fn draw_cursor<D, C, E>(&self, display: &mut D, color: C, i: i32) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E>,
        C: RgbColor,
    {
        let mut top: i32 = OFFSET + 2 + i * LINE_HEIGHT;
        let n_items = self.app_items.len() as i32;
        if n_items != 0 && i >= n_items {
            top += 4;
        }

        // Top.
        let top_left = Point::new(OFFSET + 5, top);
        let size = Size::new(228 - OFFSET as u32 * 2, 1);
        display.fill_solid(&Rectangle::new(top_left, size), color)?;

        // Bottom.
        let top_left = Point::new(OFFSET + 5, top + LINE_HEIGHT - 1);
        let size = Size::new(230 - OFFSET as u32 * 2, 1);
        display.fill_solid(&Rectangle::new(top_left, size), color)?;
        let top_left = Point::new(OFFSET + 6, top + LINE_HEIGHT);
        let size = Size::new(228 - OFFSET as u32 * 2, 1);
        display.fill_solid(&Rectangle::new(top_left, size), color)?;

        // Left.
        let top_left = Point::new(OFFSET + 3, top + 2);
        let size = Size::new(1, LINE_HEIGHT as u32 - 4);
        display.fill_solid(&Rectangle::new(top_left, size), color)?;

        // Right.
        let top_left = Point::new(234 - OFFSET, top + 3);
        let size = Size::new(2, LINE_HEIGHT as u32 - 4);
        display.fill_solid(&Rectangle::new(top_left, size), color)?;

        // Corners.
        {
            let size = Size::new(1, 1);
            // Top-right.
            let top_left = Point::new(234 - OFFSET, top + 2);
            display.fill_solid(&Rectangle::new(top_left, size), color)?;
            let top_left = Point::new(233 - OFFSET, top + 1);
            display.fill_solid(&Rectangle::new(top_left, size), color)?;
            // Top-left.
            let top_left = Point::new(OFFSET + 4, top + 1);
            display.fill_solid(&Rectangle::new(top_left, size), color)?;
            // Bottom-left.
            let top_left = Point::new(OFFSET + 4, top + LINE_HEIGHT - 2);
            display.fill_solid(&Rectangle::new(top_left, size), color)?;
            // Bottom-right.
            let top_left = Point::new(233 - OFFSET, top + LINE_HEIGHT - 2);
            let size = Size::new(2, 1);
            display.fill_solid(&Rectangle::new(top_left, size), color)?;
        }

        Ok(())
    }

    /// Indicate which item is currently selected.
    pub fn draw_battery<D, C, E>(
        &self,
        display: &mut D,
        battery: &mut Option<Battery>,
    ) -> Result<(), E>
    where
        D: DrawTarget<Color = C, Error = E>,
        C: RgbColor + FromRGB,
    {
        const MAX_WIDTH: u32 = 20;
        const HEIGHT: u32 = 9;

        let Some(battery) = battery else {
            return Ok(());
        };
        if !battery.changed {
            return Ok(());
        }
        battery.changed = false;
        let point = Point::new(
            240 - MAX_WIDTH as i32 - 5 - OFFSET,
            160 - HEIGHT as i32 - 5 - OFFSET,
        );
        let corners = CornerRadii::new(Size::new_equal(4));

        // Draw charge percentage.
        {
            let percent = u32::from(battery.percent);
            let width = MAX_WIDTH * percent / 100 + 1;
            let width = width.clamp(1, MAX_WIDTH);
            if width >= 4 {
                let size = Size::new(width, HEIGHT);
                let color = if percent <= 20 { C::DANGER } else { C::ACCENT };
                let box_style = PrimitiveStyle::with_fill(color);
                let rect = Rectangle::new(point, size);
                let rect = RoundedRectangle::new(rect, corners);
                rect.draw_styled(&box_style, display)?;
            }
        }

        // Draw box.
        {
            let size = Size::new(MAX_WIDTH, HEIGHT);
            let box_style = PrimitiveStyle::with_stroke(C::PRIMARY, 1);
            let rect = Rectangle::new(point, size);
            let rect = RoundedRectangle::new(rect, corners);
            rect.draw_styled(&box_style, display)?;
        }

        // Draw nibble on the right end.
        {
            let size = Size::new(1, 3);
            let box_style = PrimitiveStyle::with_fill(C::PRIMARY);
            let point = point + Point::new(MAX_WIDTH as _, 3);
            let rect = Rectangle::new(point, size);
            rect.draw_styled(&box_style, display)?;
        }

        // Draw indicator of charging (a lighting).
        if battery.status.connected && !battery.status.full {
            let center = point + Point::new(MAX_WIDTH as i32 / 2, HEIGHT as i32 / 2);
            let style = PrimitiveStyle::with_fill(C::PRIMARY);

            let triangle = Triangle::new(
                Point::new(center.x - 6, center.y),
                Point::new(center.x, center.y - 3),
                center,
            );
            triangle.draw_styled(&style, display)?;

            let triangle = Triangle::new(
                Point::new(center.x, center.y + 3),
                Point::new(center.x + 6, center.y),
                center,
            );
            triangle.draw_styled(&style, display)?;
        }

        Ok(())
    }
}

const MASK_ACTIVE: u8 = 0b1;
const MASK_RENDERED: u8 = 0b10;
const MASK_DIRTY: u8 = 0b100;
const MASK_MENU_PRESSED: u8 = 0b_1000;
const MASK_SELECT_PRESSED: u8 = 0b1_0000;
const MASK_WAS_RELEASED: u8 = 0b10_0000;
const MASK_ACTOR: u8 = 0b100_0000;

impl Menu {
    /// True if the menu should be currently shown.
    ///
    /// While it is true, the app is paused.
    pub fn active(&self) -> bool {
        self.flags & MASK_ACTIVE != 0
    }

    /// Close the menu (if open).
    pub fn deactivate(&mut self) {
        self.flags &= !MASK_ACTIVE;
    }

    /// True if the menu is currently rendered on the screen.
    fn rendered(&self) -> bool {
        self.flags & MASK_RENDERED != 0
    }

    fn set_rendered(&mut self, v: bool) {
        if v {
            self.flags |= MASK_RENDERED;
        } else {
            self.flags &= !MASK_RENDERED;
        }
    }

    /// True if the cursor's new position is not rendered yet.
    fn dirty(&self) -> bool {
        self.flags & MASK_DIRTY != 0
    }

    fn set_dirty(&mut self, v: bool) {
        if v {
            self.flags |= MASK_DIRTY;
        } else {
            self.flags &= !MASK_DIRTY;
        }
    }

    /// True if the menu button is currently pressed.
    fn menu_pressed(&self) -> bool {
        self.flags & MASK_MENU_PRESSED != 0
    }

    fn set_menu_pressed(&mut self, v: bool) {
        if v {
            self.flags |= MASK_MENU_PRESSED;
        } else {
            self.flags &= !MASK_MENU_PRESSED;
        }
    }

    /// True if the selection button (A) is currently pressed.
    fn select_pressed(&self) -> bool {
        self.flags & MASK_SELECT_PRESSED != 0
    }

    fn set_select_pressed(&mut self, v: bool) {
        if v {
            self.flags |= MASK_SELECT_PRESSED;
        } else {
            self.flags &= !MASK_SELECT_PRESSED;
        }
    }

    /// True if the menu button was released when the menu was open.
    fn was_released(&self) -> bool {
        self.flags & MASK_WAS_RELEASED != 0
    }

    fn set_was_released(&mut self, v: bool) {
        if v {
            self.flags |= MASK_WAS_RELEASED;
        } else {
            self.flags &= !MASK_WAS_RELEASED;
        }
    }

    /// True if the current device is the one that opened the menu.
    fn actor(&self) -> bool {
        self.flags & MASK_ACTOR != 0
    }

    pub fn set_actor(&mut self, v: bool) {
        if v {
            self.flags |= MASK_ACTOR;
        } else {
            self.flags &= !MASK_ACTOR;
        }
    }
}
