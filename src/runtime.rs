use crate::color::{ColorAdapter, FromRGB};
use crate::linking::link;
use crate::state::State;
use crate::Error;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::image::ImageDrawable;
use embedded_graphics::pixelcolor::RgbColor;
use firefly_device::*;
use firefly_meta::valid_id;
use fugit::ExtU32;

/// Default frames per second.
const FPS: u32 = 30;

pub struct Runtime<D, C>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
{
    display:  D,
    instance: wasmi::Instance,
    store:    wasmi::Store<State>,
    update:   Option<wasmi::TypedFunc<(), ()>>,
    render:   Option<wasmi::TypedFunc<(), ()>>,

    /// Time to render a single frame to match the expected FPS.
    per_frame: Delay,

    /// The last time when the frame was updated.
    prev_time: Time,
}

impl<D, C> Runtime<D, C>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
{
    /// Create a new runtime with the wasm module loaded and instantiated.
    pub fn new(
        device: DeviceImpl<'static>,
        display: D,
        author_id: &str,
        app_id: &str,
    ) -> Result<Self, Error> {
        let engine = wasmi::Engine::default();
        if !valid_id(author_id) {
            return Err(Error::InvalidAuthorID);
        }
        if !valid_id(app_id) {
            return Err(Error::InvalidAppID);
        }
        let path = &["roms", author_id, app_id, "cart.wasm"];
        let Some(stream) = device.open_file(path) else {
            return Err(Error::FileNotFound);
        };
        let module = wasmi::Module::new(&engine, stream)?;
        let now = device.now();
        let state = State::new(device);
        let mut store = wasmi::Store::<State>::new(&engine, state);
        let mut linker = wasmi::Linker::<State>::new(&engine);
        link(&mut linker)?;
        let instance_pre = linker.instantiate(&mut store, &module)?;
        let instance = instance_pre.start(&mut store)?;
        let runtime = Self {
            display,
            instance,
            store,
            update: None,
            render: None,
            per_frame: (1000 / FPS).millis(),
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
        if let Ok(start) = ins.get_typed_func::<(), ()>(&self.store, "_initialize") {
            start.call(&mut self.store, ())?;
        }
        if let Ok(start) = ins.get_typed_func::<(), ()>(&self.store, "_start") {
            start.call(&mut self.store, ())?;
        }
        // The `boot` function is defined by our spec.
        if let Ok(start) = ins.get_typed_func::<(), ()>(&self.store, "boot") {
            start.call(&mut self.store, ())?;
        }

        // Other functions defined by our spec.
        self.update = ins.get_typed_func(&self.store, "update").ok();
        self.render = ins.get_typed_func(&self.store, "render").ok();
        Ok(())
    }

    /// Update the app state and flush the frame on the display.
    ///
    /// If there is not enough time passed since the last update,
    /// the update will be delayed to keep the expected frame rate.
    pub fn update(&mut self) -> Result<(), Error> {
        if let Some(update) = self.update {
            // TODO: continue execution even if an update fails.
            update.call(&mut self.store, ())?;
        }
        if let Some(render) = self.render {
            render.call(&mut self.store, ())?;
        }

        // delay the screen flashing to adjust the frame rate
        let state = self.store.data();
        let now = state.device.now();
        let elapsed = now - self.prev_time;
        if elapsed < self.per_frame {
            let delay = self.per_frame - elapsed;
            state.device.delay(delay);
        }
        self.prev_time = state.device.now();

        self.flush_frame();
        Ok(())
    }

    /// Draw the frame buffer on the actual screen.
    fn flush_frame(&mut self) {
        let state = self.store.data();
        let mut adapter = ColorAdapter {
            state,
            target: &mut self.display,
        };
        let image = state.frame.as_image();
        // TODO: handle error
        _ = image.draw(&mut adapter);
    }

    /// Find exported memory in the instance and add it into the state.
    fn set_memory(&mut self) {
        let memory = self.instance.get_memory(&self.store, "memory");
        let state = self.store.data_mut();
        state.memory = memory;
    }
}
