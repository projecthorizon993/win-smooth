//! Crash handling system for win-smooth shell.
//! Installs a Rust panic hook to capture crash context and log fatal errors
//! before termination. Provides graceful degradation so a shell component
//! crash doesn't take down the whole desktop.

use crate::logging::{global, LogLevel};
use std::panic;
use std::sync::Once;

/// Global crash handler state.
static CRASH_HANDLER_INIT: Once = Once::new();

/// Initialize crash handling: panic hook.
/// Call once at shell startup.
pub fn init_crash_handler() {
    CRASH_HANDLER_INIT.call_once(|| {
        panic::set_hook(Box::new(panic_hook));
    });
}

/// Panic hook callback - called when a Rust panic occurs.
fn panic_hook(info: &panic::PanicInfo) {
    let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()));
    let payload = info.payload();

    if let Some(logger) = crate::logging::global() {
        let _ = logger.log(
            LogLevel::Fatal,
            "crash_handler",
            format!(
                "RUST PANIC at {}: {:?}",
                location.unwrap_or_else(|| "unknown".into()),
                payload
            ),
        );
        let _ = logger.flush();
    }

    // Terminate process after logging
    std::process::abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_handler_init_is_idempotent() {
        // Calling init twice should not panic
        let _ = init_crash_handler();
        let _ = init_crash_handler();
    }
}
