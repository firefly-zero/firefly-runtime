use crate::config::FullID;
use crate::host::misc::*;
use crate::state::{NetHandler, State};
use firefly_hal::{DeviceConfig, DeviceImpl};
use std::path::PathBuf;

#[test]
fn test_log_debug_smoke() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, log_debug);
    let inputs = wrap_input(&[0, 0]);
    let mut outputs = Vec::new();
    func.call(&mut store, &inputs, &mut outputs).unwrap();
    assert_eq!(outputs.len(), 0);
}

#[test]
fn test_log_error_smoke() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, log_error);
    let inputs = wrap_input(&[0, 0]);
    let mut outputs = Vec::new();
    func.call(&mut store, &inputs, &mut outputs).unwrap();
    assert_eq!(outputs.len(), 0);
}

#[test]
fn test_set_seed() {
    let mut store = make_store();
    let func = wasmi::Func::wrap(&mut store, set_seed);
    let inputs = wrap_input(&[131415]);
    let mut outputs = Vec::new();
    func.call(&mut store, &inputs, &mut outputs).unwrap();
    assert_eq!(outputs.len(), 0);
    let state = store.data();
    assert_eq!(state.seed, 131415)
}

#[test]
fn test_get_random() {
    let mut store = make_store();
    let state = store.data_mut();
    state.seed = 13;
    let func = wasmi::Func::wrap(&mut store, get_random);
    let mut outputs = wrap_input(&[0]);
    func.call(&mut store, &[], &mut outputs).unwrap();
    assert_eq!(outputs.len(), 1);
    // Hardcoded value that might change if we change the random algorithm.
    let expected = 3514797;
    assert_eq!(outputs[0].i32(), Some(expected));
    let state = store.data();
    assert_eq!(state.seed, expected as u32)
}

#[test]
fn test_get_name() {
    let mut store = make_store();
    let state = store.data_mut();
    state.get_settings().name = "hello".to_string();
    let memory = make_memory(&mut store);

    let func = wasmi::Func::wrap(&mut store, get_name);
    let inputs = wrap_input(&[0, 10]);
    let mut outputs = wrap_input(&[0]);
    func.call(&mut store, &inputs, &mut outputs).unwrap();

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].i32(), Some(5));
    let data = memory.data_mut(&mut store);
    assert_eq!(&data[10..15], b"hello");
}

fn wrap_input(a: &[i32]) -> Vec<wasmi::Val> {
    let mut res = Vec::new();
    for el in a {
        res.push(wasmi::Val::I32(*el))
    }
    res
}

fn make_store<'a>() -> wasmi::Store<State<'a>> {
    let engine = wasmi::Engine::default();
    let root = PathBuf::from("/tmp");
    let config = DeviceConfig {
        root,
        ..Default::default()
    };
    let device = DeviceImpl::new(config);
    let id = FullID::new(
        "test-author".try_into().unwrap(),
        "test-app".try_into().unwrap(),
    );
    let state = State::new(id, device, NetHandler::None, false);
    wasmi::Store::new(&engine, state)
}

fn make_memory(store: &mut wasmi::Store<State>) -> wasmi::Memory {
    let memory = make_memory_inner(store);
    let state = store.data_mut();
    state.memory = Some(memory);
    memory
}

fn make_memory_inner(store: &mut wasmi::Store<State>) -> wasmi::Memory {
    let limits = wasmi::MemoryType::new(1, Some(1)).unwrap();
    wasmi::Memory::new(store, limits).unwrap()
}
