#![expect(static_mut_refs)]
use firefly_audio::Manager;

/// Indicates if the audio manager is currently owned by the runtime.
///
/// While runtime locks the manager, it cannot be used by the audio thread.
static mut INTERNAL: bool = true;

static mut MANAGER: Option<Manager> = None;

/// Init a new clean audio manager.
pub(crate) fn reset() {
    critical_section::with(|_cs| unsafe {
        MANAGER.replace(Manager::new());
        INTERNAL = false;
    });
}

/// Internal function for accessing audio manager from wasm runtime.
///
/// After it is called for the first time, the external audio thread will be
/// locked until [`release`] is called at the end of the wasm update cycle.
pub(crate) fn get() -> &'static mut Manager {
    critical_section::with(|_cs| unsafe {
        INTERNAL = true;
        MANAGER.as_mut().unwrap()
    })
}

/// Allow the external audio thread to access the audio manager.
pub(crate) fn release() {
    critical_section::with(|_cs| unsafe {
        INTERNAL = false;
    });
}

/// Called by the external audio thread to access audio manager.
pub fn exec_external<F: FnOnce(&mut Manager)>(f: F) {
    while critical_section::with(|_cs| unsafe { INTERNAL }) {}
    critical_section::with(|_cs| unsafe {
        f(MANAGER.as_mut().unwrap());
    });
}
