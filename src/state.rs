use crate::config::FullID;
use crate::error::Stats;
use crate::frame_buffer::FrameBuffer;
use crate::menu::{Menu, MenuItem};
use crate::net::*;
use crate::png::save_png;
use core::cell::Cell;
use core::fmt::Display;
use embedded_io::Read;
use firefly_device::*;

pub enum NetHandler {
    None,
    #[allow(private_interfaces)]
    Connector(Connector),
    #[allow(private_interfaces)]
    Connection(Connection),
    #[allow(private_interfaces)]
    FrameSyncer(FrameSyncer),
}

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

    /// The device name. Lazy loaded on demand.
    ///
    /// None if not cached. Empty string if not provided or invalid.
    name: Option<heapless::String<16>>,

    pub net_handler: Cell<NetHandler>,
    pub connect_scene: Option<ConnectScene>,
}

impl State {
    pub(crate) fn new(id: FullID, device: DeviceImpl, net_handler: NetHandler) -> Self {
        let offline = matches!(net_handler, NetHandler::None);
        Self {
            device,
            id,
            frame: FrameBuffer::new(),
            menu: Menu::new(offline),
            seed: 0,
            memory: None,
            next: None,
            exit: false,
            input: None,
            called: "",
            net_handler: Cell::new(net_handler),
            connect_scene: None,
            name: None,
        }
    }

    pub(crate) fn stats(&self) -> Stats {
        Stats {
            last_called: self.called,
        }
    }

    /// Set ID of the next app to run and close the currently running one.
    pub(crate) fn set_next(&mut self, app: FullID) {
        match self.net_handler.get_mut() {
            NetHandler::None => {
                self.next = Some(app);
                self.exit = true;
            }
            NetHandler::Connector(_) => unreachable!("cannot launch app while connecting"),
            // TODO: support restarting an app
            // TODO: support leaving back to menu
            //       (and replacing FrameSyncer with Connection)
            NetHandler::FrameSyncer(_) => todo!("cannot re-launch running app"),
            NetHandler::Connection(c) => {
                let res = c.set_app(app);
                if let Err(err) = res {
                    self.device.log_error("netcode", err);
                }
            }
        };
    }

    pub(crate) fn get_name(&mut self) -> &str {
        if self.name.is_none() {
            let mut name = self.read_name().unwrap_or_default();
            if firefly_meta::validate_id(&name).is_err() {
                self.device.log_error("runtime", "device has invalid name");
                name = heapless::String::new()
            };
            self.name = Some(name);
        }
        self.name.as_ref().unwrap()
    }

    #[cfg(test)]
    pub(crate) fn set_name(&mut self, name: heapless::String<16>) {
        self.name = Some(name)
    }

    /// Update the state: read inputs, handle system commands.
    pub(crate) fn update(&mut self) -> Option<u8> {
        self.input = self.device.read_input();
        self.update_net();

        // Get combined input for all peers.
        //
        // In offline mode, it's just the input.
        // For multi-player game, it is the combined input of all player.
        // We use it to ensure that all players open the menu simultaneously.
        let input = match self.net_handler.get_mut() {
            NetHandler::None => self.input.clone(),
            NetHandler::Connector(_) => return None,
            NetHandler::Connection(_) => return None,
            NetHandler::FrameSyncer(syncer) => {
                // TODO: if menu is open, we need to adjust sync timeout
                // for the frame syncer.
                let mut input = InputState::default();
                for peer in &syncer.peers {
                    let state = peer.states.get_current();
                    if let Some(state) = state {
                        input = input.merge(&state.input.into());
                    };
                }
                Some(input)
            }
        };

        let action = self.menu.handle_input(&input);
        if let Some(action) = action {
            match action {
                MenuItem::Custom(index, _) => return Some(*index),
                MenuItem::Connect => self.connect(),
                MenuItem::ScreenShot => self.take_screenshot(),
                MenuItem::Restart => self.set_next(self.id.clone()),
                // TODO: quit the app for everyone
                MenuItem::Quit => self.exit = true,
            };
        };
        None
    }

    fn update_net(&mut self) {
        let handler = self.net_handler.replace(NetHandler::None);
        let handler = match handler {
            NetHandler::Connector(connector) => self.update_connector(connector),
            NetHandler::None => NetHandler::None,
            NetHandler::Connection(connection) => self.update_connection(connection),
            NetHandler::FrameSyncer(syncer) => self.update_syncer(syncer),
        };
        self.net_handler.replace(handler);
    }

    fn update_connector(&mut self, mut connector: Connector) -> NetHandler {
        connector.update(&self.device);
        let Some(scene) = self.connect_scene.as_mut() else {
            return NetHandler::Connector(connector);
        };
        let conn_status = scene.update(&self.input);
        let Some(conn_status) = conn_status else {
            return NetHandler::Connector(connector);
        };
        match conn_status {
            ConnectStatus::Stopped => {
                let res = connector.pause();
                if let Err(err) = res {
                    self.device.log_error("netcode", err);
                }
                NetHandler::Connector(connector)
            }
            ConnectStatus::Cancelled => {
                self.connect_scene = None;
                let res = connector.cancel();
                if let Err(err) = res {
                    self.device.log_error("netcode", err);
                }
                NetHandler::None
            }
            ConnectStatus::Finished => {
                self.connect_scene = None;
                let connection = connector.finalize();
                NetHandler::Connection(connection)
            }
        }
    }

    fn update_connection(&mut self, mut connection: Connection) -> NetHandler {
        let status = connection.update(&self.device);
        if matches!(status, ConnectionStatus::Launching) {
            self.next = connection.app.clone();
            self.exit = true;
            let syncer = connection.finalize();
            return NetHandler::FrameSyncer(syncer);
        }
        NetHandler::Connection(connection)
    }

    fn update_syncer(&mut self, mut syncer: FrameSyncer) -> NetHandler {
        let frame_state = self.frame_state();
        syncer.advance(&self.device, frame_state);
        while !syncer.ready() {
            let res = syncer.update(&self.device);
            if let Err(err) = res {
                self.device.log_error("netcode", err);
                self.exit = true;
                return NetHandler::None;
            }
        }
        NetHandler::FrameSyncer(syncer)
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
        if !matches!(self.net_handler.get_mut(), NetHandler::None) {
            return;
        }
        let name = self.read_name().unwrap_or_default();
        // TODO: validate the name
        let me = MyInfo { name, version: 1 };
        let net = self.device.network();
        self.net_handler
            .set(NetHandler::Connector(Connector::new(me, net)));
    }

    fn read_name(&mut self) -> Option<heapless::String<16>> {
        let mut buf = heapless::Vec::<u8, 16>::from_slice(&[0; 16]).unwrap();
        let mut file = self.device.open_file(&["sys", "name"])?;
        let size = file.read(&mut buf).ok()?;
        buf.truncate(size);
        let name = heapless::String::<16>::from_utf8(buf).unwrap();
        Some(name)
    }

    fn frame_state(&self) -> FrameState {
        let input = self.input.clone().unwrap_or_default();
        FrameState {
            frame: 0,
            input: Input {
                pad: input.pad.map(Into::into),
                buttons: input.buttons,
            },
        }
    }

    /// Log an error/warning occured in the currently executing host function.
    pub(crate) fn log_error<D: Display>(&self, msg: D) {
        self.device.log_error(self.called, msg);
    }
}
