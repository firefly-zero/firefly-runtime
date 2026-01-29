use crate::battery::Battery;
use crate::canvas::Canvas;
use crate::color::Rgb16;
use crate::config::FullID;
use crate::error::RuntimeStats;
use crate::error_scene::ErrorScene;
use crate::frame_buffer::FrameBuffer;
use crate::menu::{Menu, MenuItem};
use crate::net::*;
use crate::utils::{read_all, read_all_into};
use crate::Error;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use core::cell::Cell;
use core::fmt::Display;
use core::str::FromStr;
use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use embedded_io::Write;
use firefly_hal::*;
use firefly_types::Encode;

#[allow(private_interfaces)]
pub enum NetHandler<'a> {
    None,
    Connector(Box<Connector<'a>>),
    Connection(Box<Connection<'a>>),
    FrameSyncer(Box<FrameSyncer<'a>>),
}

pub(crate) struct State<'a> {
    /// Access to peripherals.
    pub device: DeviceImpl<'a>,

    pub rom_dir: DirImpl,

    /// The app menu manager.
    pub menu: Menu,

    launcher: bool,

    pub error: Option<ErrorScene>,

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

    /// The last read touch pad and buttons input of the current device.
    pub input: Option<InputState>,

    /// The last called host function.
    pub called: &'static str,

    /// The device settings. Lazy loaded on demand.
    ///
    /// None if not cached.
    settings: Option<firefly_types::Settings>,

    /// The battery status (State of Charge, aka SoC).
    pub battery: Option<Battery>,

    pub app_stats: Option<firefly_types::Stats>,
    pub app_stats_dirty: bool,
    pub stash: alloc::vec::Vec<u8>,
    pub stash_dirty: bool,

    pub net_handler: Cell<NetHandler<'a>>,
    action: Action,
}

impl<'a> State<'a> {
    /// Allocate new state on heap.
    ///
    /// We automatically box the state because it's relatively fat
    /// (arund 1 Kb) for the embedded heap.
    pub(crate) fn new(
        id: FullID,
        device: DeviceImpl<'a>,
        rom_dir: DirImpl,
        net_handler: NetHandler<'a>,
        launcher: bool,
    ) -> Box<Self> {
        let seed = match &net_handler {
            NetHandler::FrameSyncer(syncer) => syncer.shared_seed,
            _ => 0,
        };
        let mut device = device;
        let maybe_battery = Battery::new(&mut device);
        Box::new(Self {
            device,
            rom_dir,
            id,
            frame: FrameBuffer::new(),
            canvas: None,
            menu: Menu::new(),
            launcher,
            error: None,
            audio: firefly_audio::Manager::new(),
            battery: maybe_battery.ok(),
            seed,
            lock_seed: false,
            memory: None,
            next: None,
            exit: false,
            input: None,
            called: "",
            net_handler: Cell::new(net_handler),
            settings: None,
            app_stats: None,
            app_stats_dirty: false,
            stash: alloc::vec::Vec::new(),
            stash_dirty: false,
            action: Action::None,
        })
    }

    /// Read app stats from FS.
    pub(crate) fn load_app_stats(&mut self) -> Result<(), Error> {
        let dir_path = &["data", self.id.author(), self.id.app()];
        // TODO(@orsinium): figure out prettier error handling
        //     without more overhead. `anyhow`?
        let mut dir = match self.device.open_dir(dir_path) {
            Ok(dir) => dir,
            Err(err) => return Err(Error::OpenDir(dir_path.join("/"), err)),
        };

        let stream = match dir.open_file("stats") {
            Ok(file) => file,
            Err(err) => return Err(Error::OpenFile("stats", err)),
        };
        let raw = match read_all(stream) {
            Ok(raw) => raw,
            Err(err) => return Err(Error::ReadFile("stats", err.into())),
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
        let dir_path = &["data", self.id.author(), self.id.app()];
        let mut dir = match self.device.open_dir(dir_path) {
            Ok(dir) => dir,
            Err(err) => return Err(Error::OpenDir(dir_path.join("/"), err)),
        };

        let stream = match dir.open_file("stash") {
            Ok(file) => file,
            Err(FSError::NotFound) => return Ok(()),
            Err(err) => return Err(Error::OpenFile("stash", err)),
        };
        let res = read_all_into(stream, &mut self.stash);
        if let Err(err) = res {
            return Err(Error::ReadFile("stash", err.into()));
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
            NetHandler::None | NetHandler::Connector(_) => {
                self.next = app;
                self.exit = true;
            }
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
            self.settings = match settings {
                Some(settings) => Some(settings),
                None => Some(firefly_types::Settings {
                    xp: 0,
                    badges: 0,
                    lang: [b'e', b'n'],
                    name: "anonymous".to_string(),
                    timezone: "Europe/Amsterdam".to_string(),
                }),
            }
        }
        self.settings.as_mut().unwrap()
    }

    fn load_settings(&mut self) -> Option<firefly_types::Settings> {
        let mut dir = match self.device.open_dir(&["sys"]) {
            Ok(dir) => dir,
            Err(err) => {
                self.device.log_error("settings", err);
                return None;
            }
        };
        let file = match dir.open_file("config") {
            Ok(file) => file,
            Err(err) => {
                self.device.log_error("settings", err);
                return None;
            }
        };
        let raw = match read_all(file) {
            Ok(raw) => raw,
            Err(err) => {
                self.device.log_error("settings", FSError::from(err));
                return None;
            }
        };
        let settings = match firefly_types::Settings::decode(&raw[..]) {
            Ok(settings) => settings,
            Err(err) => {
                self.device.log_error("settings", err);
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
        let dir_path = &["data", self.id.author(), self.id.app()];
        let mut dir = match self.device.open_dir(dir_path) {
            Ok(dir) => dir,
            Err(err) => {
                self.device.log_error("stash", err);
                return;
            }
        };

        // If the stash is empty, remove the stash file instead of storing an empty file.
        if self.stash.is_empty() {
            let res = dir.remove_file("stash");
            if let Err(err) = res {
                self.device.log_error("stash", err);
            }
            return;
        };

        let mut stream = match dir.create_file("stash") {
            Ok(stream) => stream,
            Err(err) => {
                self.device.log_error("stash", err);
                return;
            }
        };
        let res = stream.write_all(&self.stash[..]);
        if let Err(err) = res {
            let err = FSError::from(err);
            self.device.log_error("stash", err);
        }
    }

    /// Save into stats the stats from the current play.
    ///
    /// Called jut before saving the stats to the disk.
    pub(crate) fn update_app_stats(&mut self) {
        let players = self.player_count();
        let idx = players - 1;
        let Some(stats) = self.app_stats.as_mut() else {
            return;
        };
        self.app_stats_dirty = true;
        stats.launches[idx] += 1;
    }

    /// Get the number of players currently online.
    fn player_count(&mut self) -> usize {
        match self.net_handler.get_mut() {
            NetHandler::None => 1,
            NetHandler::Connector(connector) => connector.peer_infos().len() + 1,
            NetHandler::Connection(connection) => connection.peers.len(),
            NetHandler::FrameSyncer(frame_syncer) => frame_syncer.peers.len(),
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
                self.device.log_error("stats", err);
                return;
            }
        };
        let dir_path = &["data", self.id.author(), self.id.app()];
        let mut dir = match self.device.open_dir(dir_path) {
            Ok(dir) => dir,
            Err(err) => {
                self.device.log_error("stats", err);
                return;
            }
        };
        let mut stream = match dir.create_file("stats") {
            Ok(stream) => stream,
            Err(err) => {
                self.device.log_error("stats", err);
                return;
            }
        };
        let res = stream.write_all(&res);
        if let Err(err) = res {
            let err = FSError::from(err);
            self.device.log_error("stats", err);
        }
    }

    /// Update the state: read inputs, handle system commands.
    pub(crate) fn update(&mut self) -> Option<u8> {
        if let Some(scene) = self.error.as_mut() {
            let close = scene.update(&mut self.device);
            if close {
                self.error = None;
            }
        }

        if self.error.is_none() {
            self.input = self.device.read_input();
        }
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
                    None => {
                        if syncer.get_combined_input().menu() {
                            Some(InputState {
                                pad: None,
                                buttons: 0b10000,
                            })
                        } else {
                            None
                        }
                    }
                }
            }
        };

        if !self.launcher {
            let action = self.menu.handle_input(&input);
            if let Some(action) = action {
                match action {
                    MenuItem::Custom(index, _) => return Some(*index),
                    MenuItem::ScreenShot => self.take_screenshot(),
                    MenuItem::Restart => self.set_next(Some(self.id.clone())),
                    MenuItem::Quit => self.set_next(None),
                };
            };
        }
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

    fn update_connector<'b>(&mut self, mut connector: Box<Connector<'b>>) -> NetHandler<'b> {
        let res = connector.update(&self.device);
        if let Err(err) = res {
            self.error = Some(ErrorScene::new(alloc::format!("{err}")));
            self.device.log_error("netcode", err);
            return NetHandler::Connector(connector);
        }
        let Some(mut conn_status) = connector.status else {
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
                self.set_next(None);
                let res = connector.cancel();
                if let Err(err) = res {
                    self.device.log_error("netcode", err);
                }
                NetHandler::None
            }
            ConnectStatus::Finished => {
                if let Err(err) = connector.validate() {
                    self.error = Some(ErrorScene::new(err.to_owned()))
                }
                self.set_next(None);
                let connection = connector.finalize();
                NetHandler::Connection(connection)
            }
        }
    }

    fn update_connection<'b>(&mut self, mut connection: Box<Connection<'b>>) -> NetHandler<'b> {
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

    fn update_syncer<'b>(&mut self, mut syncer: Box<FrameSyncer<'b>>) -> NetHandler<'b> {
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
            }
            Action::Exit => {
                self.exit = true;
                self.menu.deactivate();
                return NetHandler::Connection(syncer.into_connection());
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
    pub fn take_screenshot(&mut self) {
        let dir_path = &["data", self.id.author(), self.id.app(), "shots"];
        let mut dir = match self.device.open_dir(dir_path) {
            Ok(dir) => dir,
            Err(err) => {
                self.device.log_error("shot", err);
                return;
            }
        };

        let mut index = 1;
        _ = dir.iter_dir(|_, _| index += 1);
        let file_name = alloc::format!("{index:03}.ffs");

        let mut file = match dir.create_file(&file_name) {
            Ok(file) => file,
            Err(err) => {
                self.device.log_error("shot", err);
                return;
            }
        };
        let res = write_shot(&mut file, &self.frame.palette, &*self.frame.data);
        if let Err(err) = res {
            let err: firefly_hal::FSError = err.into();
            self.device.log_error("shot", err);
        }
    }

    pub fn connect(&mut self) {
        if !matches!(self.net_handler.get_mut(), NetHandler::None) {
            return;
        }
        let name = &self.get_settings().name;
        let name = heapless::String::from_str(name).unwrap_or_default();
        // TODO: validate the name
        let me = MyInfo { name, version: 1 };
        let net = self.device.network();
        self.net_handler
            .set(NetHandler::Connector(Box::new(Connector::new(me, net))));
        let id = FullID::from_str("sys", "connector").unwrap();
        self.set_next(Some(id));
    }

    pub fn disconnect(&self) {
        let net_handler = self.net_handler.replace(NetHandler::None);
        if let NetHandler::Connection(conn) = net_handler {
            let res = conn.disconnect();
            if let Err(err) = res {
                self.device.log_error("netcode", err);
            }
        }
    }

    /// Log an error/warning occured in the currently executing host function.
    pub(crate) fn log_error<D: Display>(&self, msg: D) {
        self.device.log_error(self.called, msg);
    }
}

/// Write the frame buffer as a screenshot file.
pub(crate) fn write_shot<W, E>(mut w: W, palette: &[Rgb16; 16], frame: &[u8]) -> Result<(), E>
where
    W: embedded_io::Write<Error = E>,
{
    w.write_all(&[0x41])?;
    // TODO(@orsinium): what is faster: to write each byte directly into the file
    // or, as it is now, create an array first and then write it in one go?
    let palette = encode_palette(palette);
    w.write_all(&palette)?;
    w.write_all(frame)?;
    Ok(())
}

/// Serialize the palette as continious RGB bytes.
fn encode_palette(palette: &[Rgb16; 16]) -> [u8; 16 * 3] {
    let mut encoded: [u8; 16 * 3] = [0; 16 * 3];
    for (i, color) in palette.iter().enumerate() {
        let color: Rgb888 = (*color).into();
        let i = i * 3;
        encoded[i] = color.r();
        encoded[i + 1] = color.g();
        encoded[i + 2] = color.b();
    }
    encoded
}
