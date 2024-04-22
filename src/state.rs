use crate::config::FullID;
use crate::frame_buffer::FrameBuffer;
use firefly_device::*;
use firefly_meta::ValidationError;

pub(crate) struct State {
    pub device: DeviceImpl,
    pub id:     FullID,
    pub frame:  FrameBuffer,
    pub seed:   u32,
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
            seed: 0,
            memory: None,
            next: None,
            exit: false,
            input: None,
        }
    }

    /// Update the state: read inputs, handle system commands.
    pub(crate) fn update(&mut self) {
        self.input = self.device.read_input();
        if let Some(InputState { buttons, .. }) = self.input {
            // exit if menu button is pressed
            if buttons[4] {
                self.exit = true
            }
        }
    }

    pub(crate) fn log_validation_error(&self, source: &str, msg: &str, err: ValidationError) {
        self.device.log_error(source, msg);
        self.device.log_error(source, err.as_str());
    }
}
