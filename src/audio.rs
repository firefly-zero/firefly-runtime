#![expect(static_mut_refs)]
use firefly_audio::Manager;

static mut MANAGER: Option<Manager> = None;

pub fn reset() {
    set(Manager::new());
}

pub fn take() -> Manager {
    loop {
        let mm = critical_section::with(|_cs| unsafe { MANAGER.take() });
        if let Some(m) = mm {
            return m;
        }
    }
}

pub fn try_take() -> Option<Manager> {
    critical_section::with(|_cs| unsafe { MANAGER.take() })
}

pub fn set(m: Manager) {
    critical_section::with(|_cs| unsafe {
        MANAGER.replace(m);
    });
}
