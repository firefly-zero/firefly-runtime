#![expect(static_mut_refs)]
use firefly_audio::Manager;

static mut INTERNAL: bool = true;
static mut MANAGER: Option<Manager> = None;

pub(crate) fn reset() {
    critical_section::with(|_cs| unsafe {
        MANAGER.replace(Manager::new());
        INTERNAL = false;
    });
}

pub(crate) fn exec_internal<F: FnOnce(&mut Manager) -> R, R>(f: F) -> R {
    let mm = critical_section::with(|_cs| unsafe {
        INTERNAL = true;
        &mut MANAGER
    });
    f(mm.as_mut().unwrap())
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
