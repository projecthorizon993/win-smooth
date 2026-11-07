//! Crash handling system for win-smooth shell.
//! Installs a structured exception handler (SEH) and panic hook to capture
//! crash context, write minidumps, and log fatal errors before termination.
//! Provides graceful degradation so a shell component crash doesn't take
//! down the whole desktop.

use crate::logging::{global, LogLevel};
use anyhow::{anyhow, Context, Result};
use std::panic;
use std::sync::Once;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, GENERIC_WRITE, OPEN_ALWAYS,
};
use windows::Win32::System::Diagnostics::{
    MiniDumpWriteDump, MINIDUMP_TYPE, MiniDumpNormal,
};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::WindowsProgramming::EXCEPTION_POINTERS;

/// Global crash handler state.
static CRASH_HANDLER_INIT: Once = Once::new();

/// Initialize crash handling: panic hook + SEH vectored exception handler.
/// Call once at shell startup.
pub fn init_crash_handler() -> Result<()> {
    CRASH_HANDLER_INIT.call_once(|| {
        // Install Rust panic hook
        panic::set_hook(Box::new(|info| {
            let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()));
            let message = info.message().map(|m| m.to_string());

            if let Some(logger) = crate::logging::global() {
                let _ = logger.log(
                    LogLevel::Fatal,
                    "crash_handler",
                    format!(
                        "RUST PANIC at {}: {:?}",
                        location.unwrap_or_else(|| "unknown".into()),
                        message.unwrap_or_else(|| "no message".into())
                    ),
                );
                let _ = logger.flush();
            }

            // Write minidump before aborting
            let _ = write_minidump(std::ptr::null_mut());

            // Terminate process after logging
            std::process::abort();
        });
    });

    Ok(())
}

/// Write a Windows minidump for post-mortem debugging.
/// If `exception_pointers` is provided (from SEH), includes exception context.
pub fn write_minidump(exception_pointers: *mut EXCEPTION_POINTERS) -> Result<()> {
    let log_dir = std::env::temp_dir();
    let dump_path = log_dir.join(format!(
        "win-smooth-crash-{}.dmp",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    ));

    let file = unsafe {
        CreateFileW(
            &dump_path,
            GENERIC_WRITE,
            FILE_SHARE_READ,
            None,
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };

    let file = match file {
        Ok(f) => f,
        Err(e) => {
            if let Some(logger) = crate::logging::global() {
                let _ = logger.log(
                    LogLevel::Error,
                    "crash_handler",
                    format!("failed to create minidump file: {}", e),
                );
            }
            return Err(e.into());
        }
    };

    // Write minidump
    let result = unsafe {
        MiniDumpWriteDump(
            GetCurrentProcess(),
            0, // process ID
            file,
            MiniDumpNormal,
            exception_pointers,
            None,
            None,
        )
    };

    let _ = CloseHandle(file);

    if result.as_bool() {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(
                LogLevel::Info,
                "crash_handler",
                format!("minidump written to {}", dump_path.display()),
            );
        }
        Ok(())
    } else {
        Err(anyhow!("MiniDumpWriteDump failed"))
    }
}

/// Vectored exception handler for SEH (structured exception handling).
/// Catches hardware exceptions (access violations, etc.) from C/C++ code
/// in the hook DLL and logs them before allowing the process to terminate.
pub unsafe extern "system" fn crash_exception_handler(
    exception_pointers: *mut EXCEPTION_POINTERS,
) -> i32 {
    if let Some(logger) = crate::logging::global() {
        let _ = logger.log(
            LogLevel::Fatal,
            "crash_handler",
            "SEH exception caught (possible access violation / hardware exception)",
        );
        let _ = logger.flush();
    }

    let _ = write_minidump(exception_pointers);

    // EXCEPTION_CONTINUE_SEARCH = 0: let the default handler terminate the process
    0
}

/// Install the SEH vectored exception handler.
pub fn install_seh_handler() -> Result<HANDLE> {
    use windows::Win32::System::Diagnostics::AddVectoredExceptionHandler;

    let handler = unsafe { AddVectoredExceptionHandler(1, Some(crash_exception_handler)) };

    if handler.is_invalid() {
        return Err(anyhow!("failed to install vectored exception handler"));
    }

    if let Some(logger) = crate::logging::global() {
        let _ = logger.log(LogLevel::Info, "crash_handler", "SEH handler installed");
    }

    Ok(handler)
}

/// Uninstall the SEH handler.
pub fn uninstall_seh_handler(handler: HANDLE) -> Result<()> {
    use windows::Win32::System::Diagnostics::RemoveVectoredExceptionHandler;

    let result = unsafe { RemoveVectoredExceptionHandler(handler) };

    if result.as_bool() {
        if let Some(logger) = crate::logging::global() {
            let _ = logger.log(LogLevel::Info, "crash_handler", "SEH handler removed");
        }
        Ok(())
    } else {
        Err(anyhow!("failed to remove vectored exception handler"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_serialization() {
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Fatal.as_str(), "FATAL");
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_crash_handler_init_is_idempotent() {
        // Calling init twice should not panic
        let _ = init_crash_handler();
        let _ = init_crash_handler();
    }
}
