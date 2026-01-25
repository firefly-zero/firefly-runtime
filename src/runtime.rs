use crate::color::FromRGB;
use crate::config::{FullID, RuntimeConfig};
use crate::error::Error;
use crate::frame_buffer::RenderFB;
use crate::linking::populate_externals;
use crate::state::{NetHandler, State};
use crate::stats::StatsTracker;
use crate::utils::read_all;
use alloc::boxed::Box;
use alloc::vec::Vec;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_io::Read;
use firefly_hal::*;
use firefly_types::*;

/// Default frames per second.
const FPS: u8 = 60;
const KB: u32 = 1024;
const FUEL_PER_CALL: u64 = 10_000_000;

pub struct Runtime<'a, D, C>
where
    D: DrawTarget<Color = C> + RenderFB + OriginDimensions,
    C: RgbColor + FromRGB,
{
    display: D,
    instance: wasmi::Instance,
    store: wasmi::Store<Box<State<'a>>>,

    update: Option<wasmi::TypedFunc<(), ()>>,
    render: Option<wasmi::TypedFunc<(), ()>>,
    before_exit: Option<wasmi::TypedFunc<(), ()>>,
    cheat: Option<wasmi::TypedFunc<(i32, i32), (i32,)>>,
    handle_menu: Option<wasmi::TypedFunc<(u32,), ()>>,

    /// Time to render a single frame to match the expected FPS.
    per_frame: Duration,
    /// The last time when the frame was updated.
    prev_time: Instant,
    /// The time that the previous frame took over the `per_frame` limit.
    prev_lag: Duration,
    n_frames: u8,
    lagging_frames: u8,
    fast_frames: u8,
    render_every: u8,

    stats: Option<StatsTracker>,
    serial: SerialImpl,
}

impl<'a, D, C> Runtime<'a, D, C>
where
    D: DrawTarget<Color = C> + RenderFB + OriginDimensions,
    C: RgbColor + FromRGB,
{
    /// Create a new runtime with the wasm module loaded and instantiated.
    pub fn new(mut config: RuntimeConfig<'a, D, C>) -> Result<Self, Error> {
        let id = match config.id {
            Some(id) => id,
            None => match detect_launcher(&mut config.device) {
                Some(id) => id,
                None => return Err(Error::NoLauncher),
            },
        };
        id.validate()?;

        let rom_path = &["roms", id.author(), id.app()];
        let mut rom_dir = match config.device.open_dir(rom_path) {
            Ok(dir) => dir,
            Err(err) => return Err(Error::OpenDir(rom_path.join("/"), err)),
        };
        let file = match rom_dir.open_file("_meta") {
            Ok(file) => file,
            Err(err) => return Err(Error::OpenFile("_meta", err)),
        };
        let bytes = match read_all(file) {
            Ok(bytes) => bytes,
            Err(err) => return Err(Error::ReadFile("_meta", err.into())),
        };
        let meta = match Meta::decode(&bytes[..]) {
            Ok(meta) => meta,
            Err(err) => return Err(Error::DecodeMeta(err)),
        };
        if meta.author_id != id.author() {
            return Err(Error::AuthorIDMismatch);
        }
        if meta.app_id != id.app() {
            return Err(Error::AppIDMismatch);
        }
        let sudo = meta.sudo;
        let launcher = meta.launcher;

        let mut serial = config.device.serial();
        let res = serial.start();
        if let Err(err) = res {
            return Err(Error::SerialStart(err));
        }
        let now = config.device.now();

        let bin_size = match rom_dir.get_file_size("_bin") {
            Ok(0) => Err(Error::FileEmpty("_bin")),
            Ok(bin_size) => Ok(bin_size),
            Err(err) => Err(Error::OpenFile("_bin", err)),
        }?;

        let engine = {
            let mut wasmi_config = wasmi::Config::default();
            wasmi_config.ignore_custom_sections(true);
            wasmi_config.consume_fuel(true);
            if bin_size > 40 * KB {
                wasmi_config.compilation_mode(wasmi::CompilationMode::Lazy);
            }
            wasmi::Engine::new(&wasmi_config)
        };

        let mut state = State::new(
            id.clone(),
            config.device,
            rom_dir,
            config.net_handler,
            launcher,
        );
        state.load_app_stats()?;
        state.load_stash()?;

        // Load the binary wasm file into PSRAM.
        let wasm_bin = {
            let mut stream = match state.rom_dir.open_file("_bin") {
                Ok(stream) => Ok(stream),
                Err(err) => Err(Error::OpenFile("_bin", err)),
            }?;
            let bin_size = bin_size as usize;
            let mut wasm_bin = state.device.alloc_psram(bin_size);
            wasm_bin.resize(bin_size, 0);
            match stream.read_exact(&mut wasm_bin) {
                Ok(_) => {}
                Err(embedded_io::ReadExactError::UnexpectedEof) => {
                    let err = FSError::AllocationError;
                    return Err(Error::OpenFile("_bin", err));
                }
                Err(embedded_io::ReadExactError::Other(err)) => {
                    let err = FSError::from(err);
                    return Err(Error::OpenFile("_bin", err));
                }
            }
            wasm_bin
        };

        let mut store = wasmi::Store::new(&engine, state);
        _ = store.set_fuel(FUEL_PER_CALL);
        let instance = {
            let module = wasmi::Module::new(&engine, wasm_bin)?;
            let mut externals = Vec::new();
            populate_externals(&mut store, &module, sudo, &mut externals)?;
            wasmi::Instance::new(&mut store, &module, &externals)?
        };

        let runtime = Self {
            display: config.display,
            instance,
            store,
            update: None,
            render: None,
            before_exit: None,
            cheat: None,
            handle_menu: None,
            stats: None,
            per_frame: Duration::from_fps(u32::from(FPS)),
            n_frames: 0,
            lagging_frames: 0,
            fast_frames: 0,
            render_every: 2,
            prev_time: now,
            prev_lag: Duration::from_ms(0),
            serial,
        };
        Ok(runtime)
    }

    /// Set how often `render` should be called relative to `update`.
    ///
    /// Exposed excludively for firefly-test to combat FPS auto-adjustment.
    pub fn set_render_every(&mut self, render_every: u8) {
        self.render_every = render_every;
    }

    pub fn display_mut(&mut self) -> &mut D {
        &mut self.display
    }

    /// Run the app until exited or an error occurs.
    pub fn run(mut self) -> Result<(), Error> {
        self.start()?;
        loop {
            self.update()?;
        }
    }

    /// Call init functions in the module.
    pub fn start(&mut self) -> Result<(), Error> {
        self.set_memory();

        let ins = self.instance;
        // The `_initialize` and `_start` functions are defined by wasip1.
        let f = ins.get_typed_func::<(), ()>(&self.store, "_initialize");
        self.call_callback("_initialize", f.ok())?;
        let f = ins.get_typed_func::<(), ()>(&self.store, "_start");
        self.call_callback("_start", f.ok())?;
        // The `boot` function is defined by our spec.
        let f = ins.get_typed_func::<(), ()>(&self.store, "boot");
        self.call_callback("boot", f.ok())?;

        // Other functions defined by our spec.
        self.update = ins.get_typed_func(&self.store, "update").ok();
        self.render = ins.get_typed_func(&self.store, "render").ok();
        self.before_exit = ins.get_typed_func(&self.store, "before_exit").ok();
        self.cheat = ins.get_typed_func(&self.store, "cheat").ok();
        self.handle_menu = ins.get_typed_func(&self.store, "handle_menu").ok();
        Ok(())
    }

    /// Update the app state and flush the frame on the display.
    ///
    /// If there is not enough time passed since the last update,
    /// the update will be delayed to keep the expected frame rate.
    pub fn update(&mut self) -> Result<bool, Error> {
        self.handle_serial()?;
        let state = self.store.data_mut();
        let menu_was_active = state.menu.active();
        let menu_index = state.update();

        if let Some(scene) = &mut state.error {
            let res = scene.render(&mut self.display);
            if res.is_err() {
                return Err(Error::CannotDisplay);
            }
            return Ok(false);
        }

        // TODO: pause audio when opening menu
        let menu_is_active = state.menu.active();
        if menu_is_active {
            if self.n_frames.is_multiple_of(60) {
                if let Some(battery) = &mut state.battery {
                    let res = battery.update(&mut state.device);
                    if let Err(err) = res {
                        state.device.log_error("battery", err);
                    }
                }
            }
            // We render the system menu directly on the screen,
            // bypassing the frame buffer. That way, we preserve
            // the frame buffer rendered by the app.
            // Performance isn't an issue for a simple text menu.
            let res = state.menu.render(&mut self.display, &state.battery);
            if res.is_err() {
                return Err(Error::CannotDisplay);
            }
            self.delay();
            return Ok(false);
        } else if menu_was_active {
            state.frame.dirty = true;
            if self.render.is_none() {
                // When menu was open but now closed, if the app doesn't have the `render`
                // callback defined, the screen flushing will never be called.
                // As a result, the menu image will stuck on the display.
                // To avoid that, we fill the screen with a color.
                //
                // The color is the same as the menu background color
                // to avoid flashing that may cause an epilepsy episode.
                _ = self.display.clear(C::BG);
            }
        }

        // If a custom menu item is selected, trigger the handle_menu callback.
        if let Some(custom_menu) = menu_index {
            if let Some(handle_menu) = self.handle_menu {
                if let Err(err) = handle_menu.call(&mut self.store, (custom_menu as u32,)) {
                    let stats = self.store.data().runtime_stats();
                    return Err(Error::FuncCall("handle_menu", err, stats));
                };
            }
        }

        // TODO: continue execution even if an update fails.
        let fuel_update = self.call_callback("update", self.update)?;
        if let Some(stats) = &mut self.stats {
            stats.update_fuel.add(fuel_update);
        }
        {
            let state = self.store.data_mut();
            let audio_buf = state.device.get_audio_buffer();
            if !audio_buf.is_empty() {
                state.audio.write(audio_buf);
            }
        }

        // Check if the app is lagging.
        // Adjust, if needed, how often "render" is called.
        // If we have time to spare, delay rendering to keep steady frame rate.
        if self.fast_frames >= FPS {
            self.render_every = (self.render_every - 1).max(1);
            self.fast_frames = 0;
        } else if self.lagging_frames >= FPS {
            self.render_every = (self.render_every + 1).min(8);
            self.lagging_frames = 0;
        }
        self.delay();

        let state = self.store.data();
        let should_render = state.exit || self.n_frames.is_multiple_of(self.render_every);
        // The frame number must be updated after calculating "should_render"
        // so that "render" is always called on the first "update" run
        // (when the app is just launched).
        self.n_frames = (self.n_frames + 1) % (FPS * 4);
        if should_render {
            let fuel_render = self.call_callback("render", self.render)?;
            if let Some(stats) = &mut self.stats {
                stats.render_fuel.add(fuel_render);
            }
            let state = self.store.data();
            if state.frame.dirty {
                self.flush_frame()?;
            }
        }
        let state = self.store.data();
        Ok(state.exit)
    }

    // Delay the screen flushing to adjust the frame rate.
    fn delay(&mut self) {
        let state = self.store.data();
        let now = state.device.now();
        let elapsed = now - self.prev_time;
        if elapsed < self.per_frame {
            let delay = self.per_frame - elapsed;
            if delay > self.prev_lag {
                let delay = delay - self.prev_lag;
                if let Some(stats) = &mut self.stats {
                    stats.delays += delay;
                    // we shaved off the previous lag, yay!
                    stats.lags -= self.prev_lag;
                }
                state.device.delay(delay);
            }
            self.fast_frames = (self.fast_frames + 1) % (FPS * 4);
            self.prev_lag = Duration::from_ms(0);
            self.lagging_frames = 0;
        } else {
            if let Some(stats) = &mut self.stats {
                stats.lags += elapsed - self.per_frame;
            }
            self.prev_lag = elapsed - self.per_frame;
            self.lagging_frames = (self.lagging_frames + 1) % (FPS * 4);
            self.fast_frames = 0;
        }
        self.prev_time = state.device.now();
    }

    /// Gracefully stop the runtime.
    ///
    /// 1. Calls `before_exit` callback.
    /// 2. Persists stash and update stats.
    /// 3. Releases [`Device`] ownership.
    /// 3. Tells which app to run next.
    pub fn finalize(mut self) -> Result<RuntimeConfig<'a, D, C>, Error> {
        self.call_callback("before_exit", self.before_exit)?;
        let mut state = self.store.into_data();
        state.save_stash();
        state.update_app_stats();
        state.save_app_stats();
        let net_handler = state.net_handler.replace(NetHandler::None);
        let config = RuntimeConfig {
            id: state.next,
            device: state.device,
            display: self.display,
            net_handler,
        };
        Ok(config)
    }

    pub fn device_mut(&mut self) -> &mut DeviceImpl<'a> {
        let state = self.store.data_mut();
        &mut state.device
    }

    /// Draw the frame buffer on the actual screen.
    fn flush_frame(&mut self) -> Result<(), Error> {
        let state = self.store.data_mut();
        let res = self.display.render_fb(&mut state.frame);
        if res.is_err() {
            return Err(Error::CannotDisplay);
        }
        Ok(())
    }

    /// Find exported memory in the instance and add it into the state.
    fn set_memory(&mut self) {
        let memory = self.instance.get_memory(&self.store, "memory");
        let state = self.store.data_mut();
        state.memory = memory;
    }

    /// Handle requests and responses on the USB serial port.
    fn handle_serial(&mut self) -> Result<(), Error> {
        let maybe_msg = match self.serial.recv() {
            Ok(msg) => msg,
            Err(err) => return Err(Error::SerialRecv(err)),
        };
        if let Some(raw_msg) = maybe_msg {
            match serial::Request::decode(&raw_msg) {
                Ok(req) => self.handle_serial_request(req)?,
                Err(err) => return Err(Error::SerialDecode(err)),
            }
        }
        self.send_stats()?;
        Ok(())
    }

    /// Send runtime stats to the serial port.
    fn send_stats(&mut self) -> Result<(), Error> {
        let Some(stats) = &mut self.stats else {
            return Ok(());
        };
        let state = self.store.data();
        let now = state.device.now();
        if let Some(memory) = state.memory {
            let data = memory.data(&self.store);
            stats.analyze_memory(data);
        }
        let Some(resp) = stats.as_message(now) else {
            return Ok(());
        };
        let encoded = match resp.encode_vec() {
            Ok(encoded) => encoded,
            Err(err) => return Err(Error::SerialEncode(err)),
        };
        let res = self.serial.send(&encoded);
        if let Err(err) = res {
            return Err(Error::SerialSend(err));
        }
        Ok(())
    }

    fn handle_serial_request(&mut self, req: serial::Request) -> Result<(), Error> {
        match req {
            serial::Request::Cheat(a, b) => {
                let Some(cheat) = self.cheat else {
                    return Err(Error::CheatUndefined);
                };
                let state = self.store.data_mut();
                if !matches!(state.net_handler.get_mut(), NetHandler::None) {
                    return Err(Error::CheatInNet);
                }
                match cheat.call(&mut self.store, (a, b)) {
                    Ok((result,)) => {
                        let resp = serial::Response::Cheat(result);
                        self.serial_send(resp)?;
                    }
                    Err(err) => {
                        let stats = self.store.data().runtime_stats();
                        return Err(Error::FuncCall("cheat", err, stats));
                    }
                }
            }
            serial::Request::Stats(stats) => {
                let state = self.store.data_mut();
                let now = state.device.now();
                if stats && self.stats.is_none() {
                    self.stats = Some(StatsTracker::new(now));
                };
                if !stats && self.stats.is_some() {
                    self.stats = None;
                };
            }
            serial::Request::AppId => {
                let state = self.store.data();
                let author = state.id.author().into();
                let app = state.id.app().into();
                let resp = serial::Response::AppID((author, app));
                self.serial_send(resp)?;
            }
            serial::Request::Screenshot => {
                let state = self.store.data_mut();
                state.take_screenshot();
                let resp = serial::Response::Ok;
                self.serial_send(resp)?;
            }
            serial::Request::Launch((author, app)) => {
                let state = self.store.data_mut();
                let resp = if let Some(id) = FullID::from_str(&author, &app) {
                    state.next = Some(id);
                    state.exit = true;
                    serial::Response::Ok
                } else {
                    serial::Response::Log("ERROR(runtime): app ID is too long".into())
                };
                self.serial_send(resp)?;
            }
            serial::Request::Exit => {
                let state = self.store.data_mut();
                state.exit = true;
                let resp = serial::Response::Ok;
                self.serial_send(resp)?;
            }
            serial::Request::Buttons(_) => todo!(),
            serial::Request::Data(_) => todo!(),
        }
        Ok(())
    }

    fn serial_send(&mut self, resp: serial::Response) -> Result<(), Error> {
        let encoded = match resp.encode_vec() {
            Ok(encoded) => encoded,
            Err(err) => return Err(Error::SerialEncode(err)),
        };
        let res = self.serial.send(&encoded);
        if let Err(err) = res {
            return Err(Error::SerialSend(err));
        }
        Ok(())
    }

    /// Call a guest function. Returns the amount of fuel consumed.
    fn call_callback(
        &mut self,
        name: &'static str,
        f: Option<wasmi::TypedFunc<(), ()>>,
    ) -> Result<u32, Error> {
        _ = self.store.set_fuel(FUEL_PER_CALL);
        if let Some(f) = f {
            if let Err(err) = f.call(&mut self.store, ()) {
                let stats = self.store.data().runtime_stats();
                return Err(Error::FuncCall(name, err, stats));
            }
        }
        let Ok(left) = self.store.get_fuel() else {
            return Ok(0);
        };
        let consumed = FUEL_PER_CALL - left;
        let consumed = u32::try_from(consumed).unwrap_or_default();
        Ok(consumed)
    }
}

fn detect_launcher(device: &mut DeviceImpl) -> Option<FullID> {
    let mut dir = device.open_dir(&["sys"]).ok()?;
    if let Some(id) = get_short_meta(&mut dir, "launcher") {
        return Some(id);
    }
    get_short_meta(&mut dir, "new-app")
}

fn get_short_meta(dir: &mut DirImpl, fname: &str) -> Option<FullID> {
    let file = dir.open_file(fname).ok()?;
    let bytes = read_all(file).ok()?;
    let meta = ShortMeta::decode(&bytes[..]).ok()?;
    let author = meta.author_id.try_into().ok()?;
    let app = meta.app_id.try_into().ok()?;
    let id = FullID::new(author, app);
    Some(id)
}
