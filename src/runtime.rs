use crate::color::{ColorAdapter, FromRGB};
use crate::device::*;
use crate::linking::link;
use crate::state::State;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::image::ImageDrawable;
use embedded_graphics::pixelcolor::RgbColor;
use fugit::ExtU32;

pub struct Runtime<D, C, T, I, S, R>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
    T: Timer,
    I: Input,
    S: Storage<R>,
    R: embedded_io::Read + wasmi::Read,
{
    device:   Device<D, C, T, I, S, R>,
    instance: wasmi::Instance,
    store:    wasmi::Store<State>,
}

impl<D, C, T, I, S, R> Runtime<D, C, T, I, S, R>
where
    D: DrawTarget<Color = C> + OriginDimensions,
    C: RgbColor + FromRGB,
    T: Timer,
    I: Input,
    S: Storage<R>,
    R: embedded_io::Read + wasmi::Read,
{
    /// Create a new runtime with the wasm module loaded and instantiated.
    pub fn new(device: Device<D, C, T, I, S, R>, cart_id: &str) -> Result<Self, wasmi::Error> {
        let engine = wasmi::Engine::default();
        // TODO: validate ID to ensure it doesn't mess with the path.
        // Using `/` or `..` in ID may lead to arbitrary file read.
        let path = &["roms", cart_id, "cart.wasm"];
        // TODO: handle "file not found".
        let stream = device.storage.open_file(path).unwrap();
        let module = wasmi::Module::new(&engine, stream)?;
        let state = State::new();
        let mut store = <wasmi::Store<State>>::new(&engine, state);
        let mut linker = <wasmi::Linker<State>>::new(&engine);
        link(&mut linker)?;
        let instance_pre = linker.instantiate(&mut store, &module)?;
        let instance = instance_pre.start(&mut store)?;
        let runtime = Self {
            device,
            instance,
            store,
        };
        Ok(runtime)
    }

    /// Run the game until exited or an error occurs.
    pub fn run(mut self) -> Result<(), wasmi::Error> {
        _ = self.device.display.clear(C::BLACK);
        self.start()?;
        let ins = self.instance;
        let update = ins.get_typed_func::<(), ()>(&self.store, "update").ok();
        let render = ins.get_typed_func::<(), ()>(&self.store, "render").ok();
        let mut prev_time = self.device.timer.now();
        let per_frame = (1000 / 30).millis();
        loop {
            if let Some(update) = update {
                // TODO: continue execution even if an update fails.
                update.call(&mut self.store, ())?;
            }
            if let Some(render) = render {
                render.call(&mut self.store, ())?;
            }

            // delay the screen flashing to adjust the frame rate
            let now = self.device.timer.now();
            let elapsed = now - prev_time;
            if elapsed < per_frame {
                let delay = per_frame - elapsed;
                self.device.timer.delay(delay);
            }
            prev_time = self.device.timer.now();

            self.flash_frame();
        }
    }

    /// Call init functions in the module.
    fn start(&mut self) -> Result<(), wasmi::Error> {
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
        Ok(())
    }

    /// Draw the frame buffer on the actual screen.
    fn flash_frame(&mut self) {
        let state = self.store.data();
        let mut adapter = ColorAdapter {
            state,
            target: &mut self.device.display,
        };
        let image = state.frame.as_image();
        // TODO: handle error
        _ = image.draw(&mut adapter);
    }
}
