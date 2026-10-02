#![expect(static_mut_refs)]
use firefly_audio::Manager;

/// Indicates if the audio manager is currently owned by the runtime.
///
/// While runtime locks the manager, it cannot be used by the audio thread.
static mut INTERNAL: bool = true;

static mut MANAGER: Option<Manager> = None;

pub(crate) fn reset() {
    critical_section::with(|_cs| unsafe {
        MANAGER.replace(Manager::new());
        INTERNAL = false;
    });
}

pub(crate) fn get() -> &'static mut Manager {
    critical_section::with(|_cs| unsafe {
        INTERNAL = true;
        MANAGER.as_mut().unwrap()
    })
}

pub(crate) fn release_internal() {
    critical_section::with(|_cs| unsafe {
        INTERNAL = false;
    });
}

pub fn exec_external<F: FnOnce(&mut Manager)>(f: F) {
    while critical_section::with(|_cs| unsafe { INTERNAL }) {}
    critical_section::with(|_cs| unsafe {
        f(MANAGER.as_mut().unwrap());
    });
}
