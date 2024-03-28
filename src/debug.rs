//! Debugging utilities.
use std::sync::atomic::{AtomicBool, Ordering};

static FLAG: AtomicBool = AtomicBool::new(false);

pub fn debug_enabled() -> bool {
    if cfg!(test) {
        true
    } else {
        FLAG.load(Ordering::Acquire)
    }
}

pub fn enable_debug() {
    FLAG.store(true, Ordering::Release);
}

pub fn disable_debug() {
    FLAG.store(false, Ordering::Release);
}

macro_rules! debug {
    ($($arg:tt)*) => {{
        if crate::debug::debug_enabled() {
            println!($($arg)*);
        }
    }}
}
