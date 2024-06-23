use crate::config::FullID;
use crate::frame_buffer::FrameBuffer;
use crate::menu::{Menu, MenuItem};
use crate::png::save_png;
use firefly_device::*;

pub(crate) struct State {
    pub device: DeviceImpl,
    pub menu:   Menu,
    pub id:     FullID,
    pub frame:  FrameBuffer,
    pub seed:   u32,
    pub online: bool,
    pub memory: Option<wasmi::Memory>,
    pub exit:   bool,
    pub next:   Option<FullID>,
    pub input:  Option<InputState>,
}

impl State {
    pub(crate) fn new(id: FullID, device: DeviceImpl) -> Self {
        Self {
            device,
            id,
            frame: FrameBuffer::new(),
            menu: Menu::new(),
            seed: 0,
            memory: None,
            next: None,
            exit: false,
            online: false,
            input: None,
        }
    }

    /// Update the state: read inputs, handle system commands.
    pub(crate) fn update(&mut self) -> Option<u8> {
        self.input = self.device.read_input();
        let action = self.menu.handle_input(&self.input);
        if let Some(action) = action {
            match action {
                MenuItem::Custom(index, _) => return Some(*index),
                MenuItem::Connect => todo!("network game is not implemented yet"),
                MenuItem::ScreenShot => self.take_screenshot(),
                MenuItem::Restart => {
                    self.next = Some(self.id.clone());
                    self.exit = true;
                }
                MenuItem::Quit => self.exit = true,
            };
        };
        None
    }

    /// Save the current frame buffer into a PNG file.
    fn take_screenshot(&mut self) {
        let dir_path = &["data", self.id.author(), self.id.app(), "shots"];
        let mut index = 1;
        self.device.iter_dir(dir_path, |_, _| index += 1);
        let file_name = alloc::format!("{}.png", index);
        let path = &["data", self.id.author(), self.id.app(), "shots", &file_name];
        let mut file = self.device.create_file(path).unwrap();
        save_png(&mut file, &self.frame.palette, &*self.frame.data).unwrap();
    }
}
