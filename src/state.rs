use crate::frame_buffer::FrameBuffer;
use core::str::FromStr;
use firefly_device::DeviceImpl;
use heapless::String;

pub(crate) struct State {
    pub device:    DeviceImpl,
    pub author_id: String<16>,
    pub app_id:    String<16>,
    pub frame:     FrameBuffer,
    pub seed:      u32,
    pub memory:    Option<wasmi::Memory>,
}

impl State {
    pub(crate) fn new(author_id: &str, app_id: &str, device: DeviceImpl) -> Self {
        Self {
            device,
            author_id: String::from_str(author_id).unwrap(),
            app_id: String::from_str(app_id).unwrap(),
            frame: FrameBuffer::new(),
            seed: 0,
            memory: None,
        }
    }
}
