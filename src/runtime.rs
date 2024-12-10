use crate::color::FromRGB;
use crate::config::{FullID, RuntimeConfig};
use crate::error::Error;
use crate::frame_buffer::HEIGHT;
use crate::linking::link;
use crate::state::{NetHandler, State};
use crate::stats::StatsTracker;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_io::Read;
use firefly_hal::*;
use firefly_types::*;

/// Default frames per second.
const FPS: u32 = 60;
const KB: u32 = 1024;
const FUEL_PER_CALL: u64 = 1_000_000;

pub struct Runtime<D, C>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
{
    display: D,
    instance: wasmi::Instance,
    store: wasmi::Store<State>,
    update: Option<wasmi::TypedFunc<(), ()>>,
    render: Option<wasmi::TypedFunc<(), ()>>,
    cheat: Option<wasmi::TypedFunc<(i32, i32), (i32,)>>,
    handle_menu: Option<wasmi::TypedFunc<(u32,), ()>>,
    render_line: Option<wasmi::TypedFunc<(i32,), (i32,)>>,

    /// Time to render a single frame to match the expected FPS.
    per_frame: Duration,
    /// The last time when the frame was updated.
    prev_time: Instant,

    stats: Option<StatsTracker>,
    serial: SerialImpl,
}

impl<D, C> Runtime<D, C>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
{
    /// Create a new runtime with the wasm module loaded and instantiated.
    pub fn new(mut config: RuntimeConfig<D, C>) -> Result<Self, Error> {
        let id = match config.id {
            Some(id) => id,
            None => match detect_launcher(&mut config.device) {
                Some(id) => id,
                None => return Err(Error::NoLauncher),
            },
        };
        id.validate()?;

        let meta_path = &["roms", id.author(), id.app(), "_meta"];
        let mut file = match config.device.open_file(meta_path) {
            Ok(file) => file,
            Err(err) => return Err(Error::OpenFile(meta_path.join("/"), err)),
        };
        let bytes = &mut [0; 64];
        let res = file.read(bytes);
        if res.is_err() {
            return Err(Error::ReadMeta);
        }
        let meta = match Meta::decode(bytes) {
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

        let mut serial = config.device.serial();
        let res = serial.start();
        if let Err(err) = res {
            return Err(Error::SerialStart(err));
        }
        let now = config.device.now();
        let bin_path = &["roms", id.author(), id.app(), "_bin"];
        let engine = {
            let mut wasmi_config = wasmi::Config::default();
            wasmi_config.consume_fuel(true);
            if let Ok(bin_size) = config.device.get_file_size(bin_path) {
                if bin_size == 0 {
                    return Err(Error::FileEmpty(bin_path.join("/")));
                }
                if bin_size > 200 * KB {
                    wasmi_config.compilation_mode(wasmi::CompilationMode::Lazy);
                }
            }
            wasmi::Engine::new(&wasmi_config)
        };

        let mut state = State::new(id.clone(), config.device, config.net_handler);
        state.load_app_stats()?;

        let stream = match state.device.open_file(bin_path) {
            Ok(file) => file,
            Err(err) => return Err(Error::OpenFile(bin_path.join("/"), err)),
        };
        let mut store = wasmi::Store::<State>::new(&engine, state);
        _ = store.set_fuel(FUEL_PER_CALL);
        let instance = {
            let module = wasmi::Module::new_streaming(&engine, stream)?;
            let mut linker = wasmi::Linker::<State>::new(&engine);
            link(&mut linker, sudo)?;
            let instance_pre = linker.instantiate(&mut store, &module)?;
            instance_pre.start(&mut store)?
        };

        let runtime = Self {
            display: config.display,
            instance,
            store,
            update: None,
            render: None,
            cheat: None,
            handle_menu: None,
            render_line: None,
            stats: None,
            per_frame: Duration::from_fps(FPS),
            prev_time: now,
            serial,
        };
        Ok(runtime)
    }

    pub fn display(&mut self) -> &mut D {
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
        _ = self.display.clear(C::BLACK);
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
        self.cheat = ins.get_typed_func(&self.store, "cheat").ok();
        self.handle_menu = ins.get_typed_func(&self.store, "handle_menu").ok();
        self.render_line = ins.get_typed_func(&self.store, "render_line").ok();
        Ok(())
    }

    /// Update the app state and flush the frame on the display.
    ///
    /// If there is not enough time passed since the last update,
    /// the update will be delayed to keep the expected frame rate.
    pub fn update(&mut self) -> Result<bool, Error> {
        self.handle_serial()?;
        let state = self.store.data_mut();
        let menu_index = state.update();

        if let Some(scene) = &state.connect_scene {
            let res = scene.render(state, &mut self.display);
            if res.is_err() {
                return Err(Error::CannotDisplay);
            }
            return Ok(false);
        }

        // TODO: pause audio when opening menu
        if state.menu.active() {
            // We render the system menu directly on the screen,
            // bypassing the frame buffer. That way, we preserve
            // the frame buffer rendered by the app.
            // Performance isn't an issue for a simple text menu.
            let res = state.menu.render(&mut self.display);
            if res.is_err() {
                return Err(Error::CannotDisplay);
            }
            self.delay();
            return Ok(false);
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
        {
            let state = self.store.data_mut();
            let audio_buf = state.device.get_audio_buffer();
            if !audio_buf.is_empty() {
                state.audio.write(audio_buf);
            }
        }
        let fuel_render = self.call_callback("render", self.render)?;
        if let Some(stats) = &mut self.stats {
            stats.update_fuel.add(fuel_update);
            stats.render_fuel.add(fuel_render);
        }
        self.delay();
        self.flush_frame()?;
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
            if let Some(stats) = &mut self.stats {
                stats.delays += delay;
            }
            state.device.delay(delay);
        } else if let Some(stats) = &mut self.stats {
            stats.lags += elapsed - self.per_frame;
        }
        self.prev_time = state.device.now();
    }

    /// When runtime is created, it takes ownership of [Device]. This method releases it.
    pub fn into_config(self) -> RuntimeConfig<D, C> {
        let mut state = self.store.into_data();
        state.save_stash();
        state.save_app_stats();
        let net_handler = state.net_handler.replace(NetHandler::None);
        RuntimeConfig {
            id: state.next,
            device: state.device,
            display: self.display,
            net_handler,
        }
    }

    pub fn device_mut(&mut self) -> &mut DeviceImpl {
        let state = self.store.data_mut();
        &mut state.device
    }

    /// Draw the frame buffer on the actual screen.
    fn flush_frame(&mut self) -> Result<(), Error> {
        // self.display.clear(C::BLACK);
        if let Some(render_line) = self.render_line {
            let mut min_y: i32 = 0;
            while min_y < HEIGHT as i32 {
                let (max_y,) = match render_line.call(&mut self.store, (min_y,)) {
                    Ok(max_y) => max_y,
                    Err(err) => {
                        let stats = self.store.data().runtime_stats();
                        return Err(Error::FuncCall("render_line", err, stats));
                    }
                };
                let max_y: i32 = if max_y == 0 { 1000 } else { max_y };
                let state = self.store.data_mut();
                _ = state
                    .frame
                    .draw_range(&mut self.display, min_y as usize, max_y as usize);
                // make sure the line number only grows
                if min_y > max_y {
                    break;
                }
                min_y = max_y;
            }
        } else {
            let state = self.store.data_mut();
            let res = state.frame.draw(&mut self.display);
            if res.is_err() {
                return Err(Error::CannotDisplay);
            }
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
                        let encoded = match resp.encode_vec() {
                            Ok(encoded) => encoded,
                            Err(err) => return Err(Error::SerialEncode(err)),
                        };
                        let res = self.serial.send(&encoded);
                        if let Err(err) = res {
                            return Err(Error::SerialSend(err));
                        }
                    }
                    Err(err) => {
                        let stats = self.store.data().runtime_stats();
                        return Err(Error::FuncCall("cheat", err, stats));
                    }
                }
            }
            serial::Request::Stats(stats) => {
                let now = self.device_mut().now();
                if stats && self.stats.is_none() {
                    self.stats = Some(StatsTracker::new(now));
                };
                if !stats && self.stats.is_some() {
                    self.stats = None;
                };
            }
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
    if let Some(id) = get_short_meta("launcher", device) {
        return Some(id);
    }
    get_short_meta("new-app", device)
}

fn get_short_meta(fname: &str, device: &mut DeviceImpl) -> Option<FullID> {
    let path = &["sys", fname];
    let mut file = device.open_file(path).ok()?;
    let bytes = &mut [0; 64];
    file.read(bytes).ok()?;
    let meta = ShortMeta::decode(bytes).ok()?;
    let author = meta.author_id.try_into().ok()?;
    let app = meta.app_id.try_into().ok()?;
    let id = FullID::new(author, app);
    Some(id)
}
