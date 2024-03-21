use crate::device::Device;
use crate::linking::link;
use crate::state::State;

pub struct Runtime<Display, Delay, Storage>
where
    Display: embedded_graphics::draw_target::DrawTarget,
    Delay: Fn(u32),
    Storage: embedded_storage::Storage,
{
    device:   Device<Display, Delay, Storage>,
    instance: wasmi::Instance,
    store:    wasmi::Store<State>,
}

impl<Display, Delay, Storage> Runtime<Display, Delay, Storage>
where
    Display: embedded_graphics::draw_target::DrawTarget,
    Delay: Fn(u32),
    Storage: embedded_storage::Storage,
{
    pub fn new(
        device: Device<Display, Delay, Storage>,
        stream: impl wasmi::Read,
    ) -> Result<Self, wasmi::Error> {
        let engine = wasmi::Engine::default();
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

    pub fn run(mut self) -> Result<(), wasmi::Error> {
        self.start()?;
        let ins = self.instance;
        let update = ins.get_typed_func::<(), ()>(&self.store, "update").ok();
        let render = ins.get_typed_func::<(), ()>(&self.store, "render").ok();
        loop {
            if let Some(update) = update {
                update.call(&mut self.store, ())?;
            }
            if let Some(render) = render {
                render.call(&mut self.store, ())?;
            }
        }
    }

    /// Call init functions in the module.
    ///
    /// The `_initialize` and `_start` functions are defined by wasip1.
    /// The `boot` function is defined by our spec.
    fn start(&mut self) -> Result<(), wasmi::Error> {
        let ins = self.instance;
        if let Ok(start) = ins.get_typed_func::<(), ()>(&self.store, "_initialize") {
            start.call(&mut self.store, ())?;
        }
        if let Ok(start) = ins.get_typed_func::<(), ()>(&self.store, "_start") {
            start.call(&mut self.store, ())?;
        }
        if let Ok(start) = ins.get_typed_func::<(), ()>(&self.store, "boot") {
            start.call(&mut self.store, ())?;
        }
        Ok(())
    }
}
