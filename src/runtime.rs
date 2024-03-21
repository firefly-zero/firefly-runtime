use crate::linking::link;
use crate::state::State;
use embedded_graphics::draw_target::DrawTarget;

pub struct Runtime<D: DrawTarget<Color = C>, C> {
    pub display: D,
}

impl<D: DrawTarget<Color = C>, C> Runtime<D, C> {
    pub fn run(self, stream: impl wasmi::Read) {
        let engine = wasmi::Engine::default();
        let module = wasmi::Module::new(&engine, stream).unwrap();
        let state = State::new();
        let mut store = <wasmi::Store<State>>::new(&engine, state);
        let mut linker = <wasmi::Linker<State>>::new(&engine);
        link(&mut linker).unwrap();
        let instance_pre = linker.instantiate(&mut store, &module).unwrap();
        let instance = instance_pre.start(&mut store).unwrap();

        // Call init functions in the module.
        // The `_initialize` and `_start` functions are defined by wasip1.
        // The `boot` function is defined by our spec.
        if let Ok(start) = instance.get_typed_func::<(), ()>(&store, "_initialize") {
            start.call(&mut store, ()).unwrap();
        }
        if let Ok(start) = instance.get_typed_func::<(), ()>(&store, "_start") {
            start.call(&mut store, ()).unwrap();
        }
        if let Ok(start) = instance.get_typed_func::<(), ()>(&store, "boot") {
            start.call(&mut store, ()).unwrap();
        }

        let update = instance.get_typed_func::<(), ()>(&store, "update").ok();
        let render = instance.get_typed_func::<(), ()>(&store, "render").ok();
        loop {
            if let Some(update) = update {
                update.call(&mut store, ()).unwrap();
            }
            if let Some(render) = render {
                render.call(&mut store, ()).unwrap();
            }
        }
    }
}
