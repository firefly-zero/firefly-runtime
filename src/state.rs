use crate::config::FullID;
use crate::error::Stats;
use crate::frame_buffer::FrameBuffer;
use crate::menu::{Menu, MenuItem};
use crate::net::{ConnectScene, Connector, MyInfo};
use crate::png::save_png;
use core::cell::Cell;
use core::fmt::Display;
use embedded_io::Read;
use firefly_device::*;

pub(crate) struct State {
    /// Access to peripherals.
    pub device: DeviceImpl,

    /// The app menu manager.
    pub menu: Menu,

    /// The id of the currently running app.
    pub id: FullID,

    /// The frame buffer.
    pub frame: FrameBuffer,

    /// The current state of the randomization function.
    pub seed: u32,

    /// True if the netplay is active.
    pub online: bool,

    /// Pointer to the app memory. Might be None if the app doesn't have memory.
    pub memory: Option<wasmi::Memory>,

    /// True if the app should be stopped.
    pub exit: bool,

    /// The next app to run.
    pub next: Option<FullID>,

    /// The last read touch pad and buttons input.
    pub input: Option<InputState>,

    /// The last called host function.
    pub called: &'static str,

    pub connector: Cell<Option<Connector>>,

    pub connect_scene: Option<ConnectScene>,
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
            called: "",
            connector: Cell::new(None),
            connect_scene: None,
        }
    }

    pub(crate) fn stats(&self) -> Stats {
        Stats {
            last_called: self.called,
        }
    }

    /// Update the state: read inputs, handle system commands.
    pub(crate) fn update(&mut self) -> Option<u8> {
        let connector = self.connector.get_mut();
        if let Some(connector) = connector {
            connector.update(&self.device);
            if let Some(scene) = self.connect_scene.as_mut() {
                scene.update();
            }
        }
        self.input = self.device.read_input();
        let action = self.menu.handle_input(&self.input);
        if let Some(action) = action {
            match action {
                MenuItem::Custom(index, _) => return Some(*index),
                MenuItem::Connect => self.connect(),
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

    fn connect(&mut self) {
        if self.connect_scene.is_none() {
            self.connect_scene = Some(ConnectScene::new());
        }
        if self.connector.get_mut().is_some() {
            return;
        }
        let name = self.read_name().unwrap_or_default();
        // TODO: validate the name
        let me = MyInfo { name, version: 1 };
        let net = self.device.network();
        self.connector.set(Some(Connector::new(me, net)));
    }

    fn read_name(&mut self) -> Option<heapless::String<16>> {
        let mut buf = heapless::Vec::<u8, 16>::from_slice(&[0; 16]).unwrap();
        let mut file = self.device.open_file(&["sys", "name"])?;
        let size = file.read(&mut buf).ok()?;
        buf.truncate(size);
        let name = heapless::String::<16>::from_utf8(buf).unwrap();
        Some(name)
    }

    /// Log an error/warning occured in the currently executing host function.
    pub(crate) fn log_error<D: Display>(&self, msg: D) {
        self.device.log_error(self.called, msg);
    }
}
