use crate::canvas::Canvas;
use crate::config::FullID;
use crate::error::RuntimeStats;
use crate::frame_buffer::FrameBuffer;
use crate::menu::{Menu, MenuItem};
use crate::png::save_png;
use crate::utils::{read_all, read_all_into};
use crate::{net::*, Error};
use core::cell::Cell;
use core::fmt::Display;
use core::str::FromStr;
use embedded_io::Write;
use firefly_hal::*;
use firefly_types::Encode;

#[allow(private_interfaces)]
pub enum NetHandler<'a> {
    None,
    Connector(Connector<'a>),
    Connection(Connection<'a>),
    FrameSyncer(FrameSyncer<'a>),
}

pub(crate) struct State<'a> {
    /// Access to peripherals.
    pub device: DeviceImpl<'a>,

    /// The app menu manager.
    pub menu: Menu,

    /// Audio manager.
    pub audio: firefly_audio::Manager,

    /// The id of the currently running app.
    pub id: FullID,

    /// The frame buffer.
    pub frame: FrameBuffer,

    /// An image in the guest memory that, if not None, used to graphics as draw target.
    pub canvas: Option<Canvas>,

    /// The current state of the randomization function.
    pub seed: u32,

    /// If true, the current seed is set by the app and must not be randomized
    /// using true RNG.
    pub lock_seed: bool,

    /// Pointer to the app memory.
    ///
    /// Might be None if the app doesn't have guest memory defined.
    pub memory: Option<wasmi::Memory>,

    /// True if the app should be stopped.
    pub exit: bool,

    /// The next app to run.
    pub next: Option<FullID>,

    /// The last read touch pad and buttons input.
    pub input: Option<InputState>,

    /// The last called host function.
    pub called: &'static str,

    /// The device settings. Lazy loaded on demand.
    ///
    /// None if not cached.
    settings: Option<firefly_types::Settings>,

    pub app_stats: Option<firefly_types::Stats>,
    pub app_stats_dirty: bool,
    pub stash: alloc::vec::Vec<u8>,
    pub stash_dirty: bool,

    pub net_handler: Cell<NetHandler<'a>>,
    pub connect_scene: Option<ConnectScene>,
    action: Action,
}

impl<'a> State<'a> {
    pub(crate) fn new(
        id: FullID,
        device: DeviceImpl<'a>,
        net_handler: NetHandler<'a>,
        launcher: bool,
    ) -> Self {
        let offline = matches!(net_handler, NetHandler::None);
        let seed = match &net_handler {
            NetHandler::FrameSyncer(syncer) => syncer.initial_seed,
            _ => 0,
        };
        Self {
            device,
            id,
            frame: FrameBuffer::new(),
            canvas: None,
            menu: Menu::new(offline, launcher),
            audio: firefly_audio::Manager::new(),
            seed,
            lock_seed: false,
            memory: None,
            next: None,
            exit: false,
            input: None,
            called: "",
            net_handler: Cell::new(net_handler),
            connect_scene: None,
            settings: None,
            app_stats: None,
            app_stats_dirty: false,
            stash: alloc::vec::Vec::new(),
            stash_dirty: false,
            action: Action::None,
        }
    }

    /// Read app stats from FS.
    pub(crate) fn load_app_stats(&mut self) -> Result<(), Error> {
        let path = &["data", self.id.author(), self.id.app(), "stats"];
        let stream = match self.device.open_file(path) {
            Ok(file) => file,
            Err(FSError::NotFound) => return Ok(()),
            Err(err) => return Err(Error::OpenFile(path.join("/"), err)),
        };
        let raw = match read_all(stream) {
            Ok(raw) => raw,
            Err(err) => return Err(Error::ReadFile(path.join("/"), err.into())),
        };
        let stats = match firefly_types::Stats::decode(&raw) {
            Ok(stats) => stats,
            Err(err) => return Err(Error::DecodeStats(err)),
        };
        self.app_stats = Some(stats);
        Ok(())
    }

    /// Read stash from FS.
    pub(crate) fn load_stash(&mut self) -> Result<(), Error> {
        let path = &["data", self.id.author(), self.id.app(), "stash"];
        let stream = match self.device.open_file(path) {
            Ok(file) => file,
            Err(FSError::NotFound) => return Ok(()),
            Err(err) => return Err(Error::OpenFile(path.join("/"), err)),
        };
        let res = read_all_into(stream, &mut self.stash);
        if let Err(err) = res {
            return Err(Error::ReadFile(path.join("/"), err.into()));
        };
        Ok(())
    }

    pub(crate) fn runtime_stats(&self) -> RuntimeStats {
        RuntimeStats {
            last_called: self.called,
        }
    }

    /// Set ID of the next app to run and close the currently running one.
    pub(crate) fn set_next(&mut self, app: Option<FullID>) {
        match self.net_handler.get_mut() {
            NetHandler::None => {
                self.next = app;
                self.exit = true;
            }
            NetHandler::Connector(_) => unreachable!("cannot launch app while connecting"),
            NetHandler::FrameSyncer(_) => {
                let action = match app {
                    Some(id) if id == self.id => Action::Restart,
                    Some(_) => panic!("cannot launch another app in multiplayer"),
                    None => Action::Exit,
                };
                self.action = action;
            }
            NetHandler::Connection(c) => {
                let Some(app) = app else { return };
                let res = c.set_app(&mut self.device, app);
                if let Err(err) = res {
                    self.device.log_error("netcode", err);
                }
            }
        };
    }

    pub(crate) fn get_settings(&mut self) -> &mut firefly_types::Settings {
        use crate::alloc::string::ToString;
        if self.settings.is_none() {
            let settings = self.load_settings();
            match settings {
                Some(settings) => self.settings = Some(settings),
                None => {
                    self.settings = Some(firefly_types::Settings {
                        xp: 0,
                        badges: 0,
                        lang: [b'e', b'n'],
                        name: "anonymous".to_string(),
                        timezone: "Europe/Amsterdam".to_string(),
                    })
                }
            }
        }
        self.settings.as_mut().unwrap()
    }

    fn load_settings(&mut self) -> Option<firefly_types::Settings> {
        let path = &["sys", "config"];
        let file = match self.device.open_file(path) {
            Ok(file) => file,
            Err(_) => {
                self.log_error("failed to open settings");
                return None;
            }
        };
        let raw = match read_all(file) {
            Ok(raw) => raw,
            Err(_) => {
                self.log_error("failed to read settings");
                return None;
            }
        };
        let settings = match firefly_types::Settings::decode(&raw[..]) {
            Ok(settings) => settings,
            Err(_) => {
                self.log_error("failed to parse settings");
                return None;
            }
        };
        Some(settings)
    }

    /// Dump stash (if any) on disk.
    pub(crate) fn save_stash(&mut self) {
        if !self.stash_dirty {
            return;
        }
        let stash_path = &["data", self.id.author(), self.id.app(), "stash"];

        // If the stash is empty, remove the stash file instead of storing an empty file.
        if self.stash.is_empty() {
            let res = self.device.remove_file(stash_path);
            if let Err(err) = res {
                self.log_error(err);
            }
            return;
        };

        let mut stream = match self.device.create_file(stash_path) {
            Ok(stream) => stream,
            Err(err) => {
                self.log_error(err);
                return;
            }
        };
        let res = stream.write_all(&self.stash[..]);
        if let Err(err) = res {
            let err = FSError::from(err);
            self.log_error(err);
        }
    }

    /// Dump app stats (if changed) on disk.
    pub(crate) fn save_app_stats(&mut self) {
        let Some(stats) = &self.app_stats else {
            return;
        };
        if !self.app_stats_dirty {
            return;
        }
        let res = match stats.encode_vec() {
            Ok(res) => res,
            Err(err) => {
                self.log_error(err);
                return;
            }
        };
        let stats_path = &["data", self.id.author(), self.id.app(), "stats"];
        let mut stream = match self.device.create_file(stats_path) {
            Ok(stream) => stream,
            Err(err) => {
                self.log_error(err);
                return;
            }
        };
        let res = stream.write_all(&res);
        if let Err(err) = res {
            let err = FSError::from(err);
            self.log_error(err);
        }
    }

    /// Update the state: read inputs, handle system commands.
    pub(crate) fn update(&mut self) -> Option<u8> {
        self.input = self.device.read_input();
        self.update_net();

        // Get combined input for all peers.
        //
        // In offline mode, it's just the input.
        // For multi-player game, it is the combined input of all player,
        // unless in launcher (Connector or Connection).
        // We use it to ensure that all players open the app menu simultaneously.
        let input = match self.net_handler.get_mut() {
            // single-player
            NetHandler::None => self.input.clone(),
            // shouldn't be reachable
            NetHandler::Connector(_) => return None,
            // in launcher
            NetHandler::Connection(_) => self.input.clone(),
            // in game
            NetHandler::FrameSyncer(syncer) => {
                // TODO: if menu is open, we need to adjust sync timeout
                // for the frame syncer.
                match &self.input {
                    Some(input) => {
                        // In frame syncer, use shared input for the menu button
                        // (if one player presses it, press it for everyone)
                        // and local input for all other buttons.
                        let mut input = input.clone();
                        if syncer.get_combined_input().menu() {
                            input.buttons |= 0b10000;
                        } else {
                            input.buttons &= !0b10000;
                        };
                        Some(input)
                    }
                    None => todo!(),
                }
            }
        };

        let action = self.menu.handle_input(&input);
        if let Some(action) = action {
            match action {
                MenuItem::Custom(index, _) => return Some(*index),
                MenuItem::Connect => self.connect(),
                MenuItem::Disconnect => self.disconnect(),
                MenuItem::ScreenShot => self.take_screenshot(),
                MenuItem::Restart => self.set_next(Some(self.id.clone())),
                // TODO: quit the app for everyone
                MenuItem::Quit => self.set_next(None),
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

    fn update_connector<'b>(&mut self, mut connector: Connector<'b>) -> NetHandler<'b> {
        connector.update(&self.device);
        let Some(scene) = self.connect_scene.as_mut() else {
            return NetHandler::Connector(connector);
        };
        let conn_status = scene.update(&self.input);
        let Some(mut conn_status) = conn_status else {
            return NetHandler::Connector(connector);
        };
        // If the peers list contains only the current device itself,
        // we can't start multiplayer: treat confirmation as cancellation.
        if conn_status == ConnectStatus::Finished && connector.peer_infos().is_empty() {
            conn_status = ConnectStatus::Cancelled;
        }
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
                // Re-render menu with "disconnect" button instead of "connect".
                self.menu = Menu::new(false, true);
                let connection = connector.finalize();
                NetHandler::Connection(connection)
            }
        }
    }

    fn update_connection<'b>(&mut self, mut connection: Connection<'b>) -> NetHandler<'b> {
        let status = connection.update(&mut self.device);
        match status {
            ConnectionStatus::Launching => {
                if let Some(app_id) = &connection.app {
                    self.set_next(Some(app_id.clone()));
                    let syncer = connection.finalize(&mut self.device);
                    return NetHandler::FrameSyncer(syncer);
                }
            }
            ConnectionStatus::Timeout => {
                let msg = "timed out waiting for other devices to launch the app";
                self.device.log_error("netcode", msg);
                self.set_next(None);
                return NetHandler::None;
            }
            _ => (),
        }
        NetHandler::Connection(connection)
    }

    fn update_syncer<'b>(&mut self, mut syncer: FrameSyncer<'b>) -> NetHandler<'b> {
        // * Don't sync seed if it is locked by the app (misc.set_seed was called).
        // * Don't sync seed if misc.get_random was never called.
        // * Don't sync seed too often to avoid poking true RNG too often.
        let sync_rand = !self.lock_seed && self.seed != 0 && syncer.frame % 60 == 21;
        let rand = if sync_rand { self.device.random() } else { 0 };

        let input = self.input.clone().unwrap_or_default();
        let frame_state = FrameState {
            // No need to set frame number here,
            // it will be set by FrameSyncer.advance.
            frame: 0,
            rand,
            input: Input {
                pad: input.pad.map(Into::into),
                buttons: input.buttons,
            },
            action: self.action,
        };

        syncer.advance(&mut self.device, frame_state);
        while !syncer.ready() {
            let res = syncer.update(&self.device);
            if let Err(err) = res {
                self.device.log_error("netcode", err);
                self.set_next(None);
                return NetHandler::None;
            }
        }

        let action = syncer.get_action();
        match action {
            Action::None => (),
            Action::Restart => {
                self.next = Some(self.id.clone());
                self.exit = true;
                self.menu.deactivate();
                // TODO: reset FrameSyncer
            }
            Action::Exit => {
                self.exit = true;
                self.menu.deactivate();
                // TODO: replace FrameSyncer with Connection
            }
        }

        if sync_rand {
            let seed = syncer.get_seed();
            if seed != 0 {
                self.seed = seed;
            }
        }
        NetHandler::FrameSyncer(syncer)
    }

    /// Save the current frame buffer into a PNG file.
    fn take_screenshot(&mut self) {
        let old_app = self.called;
        self.called = "take screenshot";
        let dir_path = &["data", self.id.author(), self.id.app(), "shots"];
        let mut index = 1;
        _ = self.device.iter_dir(dir_path, |_, _| index += 1);
        let file_name = alloc::format!("{}.png", index);
        let path = &["data", self.id.author(), self.id.app(), "shots", &file_name];
        let mut file = match self.device.create_file(path) {
            Ok(file) => file,
            Err(err) => {
                self.log_error(err);
                self.called = old_app;
                return;
            }
        };
        let res = save_png(&mut file, &self.frame.palette, &*self.frame.data);
        if let Err(err) = res {
            let err: firefly_hal::FSError = err.into();
            self.log_error(err);
        }
        self.called = old_app;
    }

    fn connect(&mut self) {
        if self.connect_scene.is_none() {
            self.connect_scene = Some(ConnectScene::new());
        }
        if !matches!(self.net_handler.get_mut(), NetHandler::None) {
            return;
        }
        let name = &self.get_settings().name;
        let name = heapless::String::from_str(name).unwrap_or_default();
        // TODO: validate the name
        let me = MyInfo { name, version: 1 };
        let net = self.device.network();
        self.net_handler
            .set(NetHandler::Connector(Connector::new(me, net)));
    }

    fn disconnect(&mut self) {
        self.connect_scene = None;
        let net_handler = self.net_handler.replace(NetHandler::None);
        if let NetHandler::Connection(conn) = net_handler {
            let res = conn.disconnect();
            if let Err(err) = res {
                self.device.log_error("netcode", err);
            }
        }
        // Re-render menu with "disconnect" replaced by "connect".
        self.menu = Menu::new(true, true);
    }

    /// Log an error/warning occured in the currently executing host function.
    pub(crate) fn log_error<D: Display>(&self, msg: D) {
        self.device.log_error(self.called, msg);
    }
}
