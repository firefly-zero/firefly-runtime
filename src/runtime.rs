use crate::color::FromRGB;
use crate::config::RuntimeConfig;
use crate::error::Error;
use crate::frame_buffer::HEIGHT;
use crate::linking::link;
use crate::state::{NetHandler, State};
use crate::FullID;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::RgbColor;
use embedded_io::Read;
use firefly_device::*;
use firefly_meta::ShortMeta;

/// Default frames per second.
const FPS: u32 = 60;

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
    handle_menu: Option<wasmi::TypedFunc<(u32,), ()>>,
    render_line: Option<wasmi::TypedFunc<(i32,), (i32,)>>,

    /// Time to render a single frame to match the expected FPS.
    per_frame: Duration,

    /// The last time when the frame was updated.
    prev_time: Instant,
}

impl<D, C> Runtime<D, C>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
{
    /// Create a new runtime with the wasm module loaded and instantiated.
    pub fn new(config: RuntimeConfig<D, C>) -> Result<Self, Error> {
        let engine = wasmi::Engine::default();
        let id = match config.id {
            Some(id) => id,
            None => match detect_launcher(&config.device) {
                Some(id) => id,
                None => return Err(Error::NoLauncher),
            },
        };
        id.validate()?;

        let path = &["roms", id.author(), id.app(), "_bin"];
        let Some(stream) = config.device.open_file(path) else {
            return Err(Error::FileNotFound(path.join("/")));
        };
        let module = wasmi::Module::new_streaming(&engine, stream)?;
        let now = config.device.now();
        let state = State::new(id, config.device, config.net_handler);
        let mut store = wasmi::Store::<State>::new(&engine, state);
        let mut linker = wasmi::Linker::<State>::new(&engine);
        link(&mut linker)?;
        let instance_pre = linker.instantiate(&mut store, &module)?;
        let instance = instance_pre.start(&mut store)?;
        let runtime = Self {
            display: config.display,
            instance,
            store,
            update: None,
            render: None,
            handle_menu: None,
            render_line: None,
            per_frame: Duration::from_fps(FPS),
            prev_time: now,
        };
        Ok(runtime)
    }

    pub fn display(&self) -> &D {
        &self.display
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
        self.handle_menu = ins.get_typed_func(&self.store, "handle_menu").ok();
        self.render_line = ins.get_typed_func(&self.store, "render_line").ok();
        Ok(())
    }

    /// Update the app state and flush the frame on the display.
    ///
    /// If there is not enough time passed since the last update,
    /// the update will be delayed to keep the expected frame rate.
    pub fn update(&mut self) -> Result<bool, Error> {
        let state = self.store.data_mut();
        let menu_index = state.update();

        if let Some(scene) = &state.connect_scene {
            let res = scene.render(state, &mut self.display);
            if res.is_err() {
                return Err(Error::CannotDisplay);
            }
            return Ok(false);
        }

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
                    let stats = self.store.data().stats();
                    return Err(Error::FuncCall("handle_menu", err, stats));
                };
            }
        }

        // TODO: continue execution even if an update fails.
        self.call_callback("update", self.update)?;
        self.call_callback("render", self.render)?;
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
            state.device.delay(delay);
        }
        self.prev_time = state.device.now();
    }

    /// When runtime is created, it takes ownership of [Device]. This method releases it.
    pub fn into_config(self) -> RuntimeConfig<D, C> {
        let state = self.store.into_data();
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
                        let stats = self.store.data().stats();
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

    fn call_callback(
        &mut self,
        name: &'static str,
        f: Option<wasmi::TypedFunc<(), ()>>,
    ) -> Result<(), Error> {
        if let Some(f) = f {
            if let Err(err) = f.call(&mut self.store, ()) {
                let stats = self.store.data().stats();
                return Err(Error::FuncCall(name, err, stats));
            }
        }
        Ok(())
    }
}

fn detect_launcher(device: &DeviceImpl) -> Option<FullID> {
    if let Some(id) = get_short_meta("launcher", device) {
        return Some(id);
    }
    get_short_meta("new-app", device)
}

fn get_short_meta(fname: &str, device: &DeviceImpl) -> Option<FullID> {
    let path = &["sys", fname];
    let mut file = device.open_file(path)?;
    let bytes = &mut [0; 64];
    file.read(bytes).ok()?;
    let meta = ShortMeta::decode(bytes).ok()?;
    let author = meta.author_id.try_into().ok()?;
    let app = meta.app_id.try_into().ok()?;
    let id = FullID::new(author, app);
    Some(id)
}
