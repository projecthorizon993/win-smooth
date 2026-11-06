mod ipc;
mod shell_ui;
mod dwm_tuning;

use std::sync::mpsc;

fn main() -> anyhow::Result<()> {
    // 1. Start named pipe server (IPC from hook DLL)
    // 2. Create invisible message-only window or overlay window
    // 3. Direct2D render loop (flip-model present)
    // 4. Dispatch pipe events -> UI / window manager
    // 5. DWM tuning applied once at startup
    dwm_tuning::apply_dwm_tuning()?;
    ipc::start_pipe_server()?;
    shell_ui::init_direct2d()?;
    Ok(())
}
