mod ipc;
mod shell_ui;
mod dwm_tuning;

use crate::shell_ui::render_context::DrawContext;
use crate::shell_ui::taskbar::Taskbar;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

fn main() -> anyhow::Result<()> {
    // 1. DWM tuning applied once at startup.
    dwm_tuning::apply_dwm_tuning()?;
    // 2. Named pipe server (IPC from hook DLL).
    ipc::start_pipe_server()?;

    // 3. Create and show the glass taskbar overlay window.
    let mut taskbar = Taskbar::new();
    taskbar.init_window()?;
    taskbar.show();

    let hwnd = taskbar
        .hwnd
        .expect("taskbar window handle after init_window");

    // 4. Direct2D render target for the taskbar window.
    let mut dc = DrawContext::create_for_window(hwnd)?;

    // 5. Render loop: pump messages + paint one glass taskbar frame each pass.
    unsafe {
        let mut msg = MSG::default();
        loop {
            let have_msg = GetMessageW(&mut msg, None, 0, 0).as_bool();
            if !have_msg {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);

            dc.begin_frame();
            taskbar.paint(&dc)?;
            dc.end_frame()?;
        }
    }

    Ok(())
}
