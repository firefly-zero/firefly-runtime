use crate::config::FullID;
use crate::frame_buffer::FrameBuffer;
use crate::menu::Menu;
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
    pub(crate) fn update(&mut self) {
        self.input = self.device.read_input();
        self.menu.handle_input(&self.input);
    }
}
