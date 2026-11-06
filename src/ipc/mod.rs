// Named pipe server: \\.\pipe\win-smooth-ipc
// Receives AppMessage structs from hook_dll.dll (C/C++ client).
// Runs on a dedicated thread; main loop polls the channel with try_recv().
//
// Note: windows-sys 0.59 gates CreateNamedPipeW/ConnectNamedPipe behind
// Win32_Security + Win32_Storage_FileSystem / Win32_System_IO features that
// Cargo.toml does not enable. Declaring the externs directly here avoids
// touching Cargo.toml; kernel32.dll is always linked on MSVC.

use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};

// ---------------------------------------------------------------------------
// Named pipe constants (PIPE_ACCESS_DUPLEX is not defined in windows-sys 0.59)
// ---------------------------------------------------------------------------

const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
const PIPE_TYPE_MESSAGE: u32 = 0x00000004;
const PIPE_READMODE_MESSAGE: u32 = 0x00000002;
const PIPE_WAIT: u32 = 0x00000000;
const PIPE_UNLIMITED_INSTANCES: u32 = 255;

pub const PIPE_NAME: &str = r"\\.\pipe\win-smooth-ipc";

// ---------------------------------------------------------------------------
// Message types (must match hook_dll AppMessage.msg_type)
// ---------------------------------------------------------------------------

pub const MSG_TRAY_ADD: u32 = 1;
pub const MSG_TRAY_MODIFY: u32 = 2;
pub const MSG_TRAY_DELETE: u32 = 3;
pub const MSG_TASKBAR_BUTTON_OVERLAY: u32 = 4;
pub const MSG_QUIT: u32 = 5;

/// Fixed binary schema matching the C/C++ hook_dll AppMessage.
/// 64 bytes total, naturally aligned, all Copy fields — safe to bit-copy.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AppMessage {
    pub msg_type: u32,
    pub win_id: u32,
    pub payload: [u8; 56],
}

impl Default for AppMessage {
    fn default() -> Self {
        Self {
            msg_type: 0,
            win_id: 0,
            payload: [0u8; 56],
        }
    }
}

/// Receiver end of the IPC event channel.
pub struct PipeServer {
    pub rx: mpsc::Receiver<AppMessage>,
}

/// Start the named pipe server on a background thread.
/// Returns a PipeServer whose rx the main loop polls with try_recv().
pub fn start_pipe_server() -> anyhow::Result<PipeServer> {
    let (tx, rx) = mpsc::channel();
    let pipe_name = PIPE_NAME.to_string();

    thread::spawn(move || {
        pipe_loop(pipe_name, tx);
    });

    Ok(PipeServer { rx })
}

extern "system" {
    fn CreateNamedPipeW(
        lpname: windows_sys::core::PCWSTR,
        dwopenmode: u32,
        dwpipemode: u32,
        nmaxinstances: u32,
        noutbuffersize: u32,
        ninbuffersize: u32,
        ndefaulttimeout: u32,
        lpsecurityattributes: *mut std::ffi::c_void,
    ) -> HANDLE;
    fn ConnectNamedPipe(
        hnamedpipe: HANDLE,
        lpoverlapped: *mut std::ffi::c_void,
    ) -> windows_sys::Win32::Foundation::BOOL;
    fn DisconnectNamedPipe(hnamedpipe: HANDLE) -> windows_sys::Win32::Foundation::BOOL;
    fn ReadFile(
        hfile: HANDLE,
        lpbuffer: *mut u8,
        nnumberofbytestoread: u32,
        lpnumberofbytesread: *mut u32,
        lpoverlapped: *mut std::ffi::c_void,
    ) -> windows_sys::Win32::Foundation::BOOL;
}

fn pipe_loop(pipe_name: String, tx: mpsc::Sender<AppMessage>) {
    let wide: Vec<u16> = pipe_name.encode_utf16().collect();

    loop {
        let pipe: HANDLE = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                std::ptr::null_mut(),
            )
        };

        if pipe == INVALID_HANDLE_VALUE {
            thread::sleep(Duration::from_secs(1));
            continue;
        }

        let connected = unsafe { ConnectNamedPipe(pipe, std::ptr::null_mut()) } != 0;

        if !connected {
            let _ = unsafe { CloseHandle(pipe) };
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        // Read fixed-size AppMessage frames until the client disconnects.
        let mut buf: [u8; std::mem::size_of::<AppMessage>()] = [0; 64];
        loop {
            let mut read: u32 = 0;
            let ok = unsafe {
                ReadFile(
                    pipe,
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                    &mut read,
                    std::ptr::null_mut(),
                )
            }
            != 0;

            if !ok || read == 0 {
                break; // client disconnected
            }

            if read as usize != buf.len() {
                continue; // short read — invalid schema, skip
            }

            let msg: AppMessage = unsafe { *(buf.as_ptr() as *const AppMessage) };

            // Validate message type at the trust boundary before forwarding.
            if !is_valid_msg_type(msg.msg_type) {
                continue;
            }

            if tx.send(msg).is_err() {
                break; // main loop gone
            }
        }

        let _ = unsafe { DisconnectNamedPipe(pipe) };
        let _ = unsafe { CloseHandle(pipe) };
    }
}

fn is_valid_msg_type(t: u32) -> bool {
    matches!(
        t,
        MSG_TRAY_ADD | MSG_TRAY_MODIFY | MSG_TRAY_DELETE |
            MSG_TASKBAR_BUTTON_OVERLAY | MSG_QUIT
    )
}
