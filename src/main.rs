mod ipc;
mod shell_ui;
mod dwm_tuning;
mod window_manager;
mod rendering;

use crate::crash_handler::{init_crash_handler, install_seh_handler};
use crate::ipc::{
    AppMessage, MSG_TRAY_ADD, MSG_TRAY_DELETE, MSG_TRAY_MODIFY,
    MSG_TASKBAR_BUTTON_OVERLAY, PipeServer,
};
use crate::logging::{global, init_global, LogLevel};
use crate::rendering::{PipelineConfig, PresentMode, RenderBackend, Renderer};
use crate::shell_ui::animation::init_defaults;
use crate::shell_ui::control_center::ControlCenter;
use crate::shell_ui::launcher::Launcher;
use crate::shell_ui::notifications::NotificationCenter;
use crate::shell_ui::render_context::DrawContext;
use crate::shell_ui::spring::AnimationManager;
use crate::shell_ui::taskbar::Taskbar;
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PeekMessageW, PM_REMOVE, TranslateMessage, WM_QUIT,
};

fn main() -> anyhow::Result<()> {
    // 0. Initialize logging + crash handling FIRST (before anything else).
    let log_path = std::env::temp_dir().join("win-smooth.log");
    init_global(Some(log_path), LogLevel::Info)?;
    let _seh_handler = install_seh_handler()?;
    init_crash_handler();

    if let Some(logger) = global() {
        logger.log(LogLevel::Info, "main", "win-smooth shell starting")?;
    }

    // 1. DWM tuning applied once at startup.
    dwm_tuning::apply_dwm_tuning()?;

    // 2. Named pipe server (IPC from hook DLL).
    let pipe = ipc::start_pipe_server()?;

    // 3. Window manager (tiling + floating) runs on a background thread,
    //    re-tiling top-level windows every 2s while working with DWM.
    std::thread::spawn(move || {
        window_manager::WindowManager::new()
            .expect("window manager init")
            .run(Duration::from_secs(2));
    });

    // 4. Create and show the glass taskbar overlay window.
    let mut taskbar = Taskbar::new();
    taskbar.init_window()?;
    taskbar.show();

    let hwnd: HWND = taskbar
        .hwnd
        .expect("taskbar window handle after init_window");

    // 4. Direct2D render target for the taskbar window.
    let mut dc = DrawContext::create_for_window(hwnd)?;

    // 5. Multi-backend rendering pipeline (Direct2D/D3D11/D3D12).
    //    Defaults to Direct3D11 with mailbox present mode for smoothness.
    let mut renderer = Renderer::new(
        RenderBackend::Direct3D11,
        PipelineConfig {
            backend: RenderBackend::Direct3D11,
            target_fps: 60,
            enable_vsync: true,
            triple_buffering: true,
            present_mode: PresentMode::Mailbox,
            flush_gpu_after_present: false,
        },
    )?;
    renderer.init(hwnd)?;

    // 5. Other UI components.
    let mut launcher = Launcher::new()?;
    let mut control_center = ControlCenter::new();
    let mut notifications = NotificationCenter::new();
    let mut anim = AnimationManager::new();
    init_defaults(&mut anim);

    // 6. Main render loop: pump messages → process IPC → update UI → paint → present.
    let mut last = Instant::now();
    let mut running = true;

    while running {
        // Pump all pending Win32 messages (non-blocking drain).
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    running = false;
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        if !running {
            break;
        }

        // Process IPC events (non-blocking).
        while let Ok(msg) = pipe.rx.try_recv() {
            dispatch_ipc(
                &msg,
                &mut taskbar,
                &mut launcher,
                &mut control_center,
                &mut notifications,
            );
        }

        // Tick animations each frame.
        let now = Instant::now();
        let dt = now
            .duration_since(last)
            .as_secs_f32()
            .clamp(0.0, 1.0);
        last = now;

        let _ = anim.tick(); // AnimationManager springs (e.g. control center translate)
        taskbar.tick(dt);
        launcher.tick(dt);
        let _ = control_center.update(dt);

        // Paint all visible components each frame.
        dc.begin_frame();
        taskbar.paint(&mut dc)?;
        if launcher.is_open() {
            launcher.paint(&mut dc)?;
        }
        // ControlCenter and NotificationCenter track visibility/animation state;
        // their Direct2D paint methods are not yet implemented in this change.
        dc.end_frame()?;

        // Present frame via the multi-backend rendering pipeline.
        let _frame_ms = renderer.present_frame()?;

        // Cap at ~60fps to avoid busy-waiting when idle.
        let elapsed = last.elapsed();
        let target = Duration::from_millis(16);
        if elapsed < target {
            thread::sleep(target - elapsed);
        }
    }

    Ok(())
}

/// Dispatch an inbound IPC message into shell UI state.
/// Runs at a trust boundary: msg_type already validated by the pipe reader.
fn dispatch_ipc(
    msg: &AppMessage,
    taskbar: &mut Taskbar,
    launcher: &mut Launcher,
    control_center: &mut ControlCenter,
    notifications: &mut NotificationCenter,
) {
    match msg.msg_type {
        MSG_TRAY_ADD => {
            // Hook DLL reports a tray icon added; refresh tray paint next frame.
            // Tray items are currently static defaults; overlay state is tracked here.
        }
        MSG_TRAY_MODIFY => {
            // Tray icon state changed (e.g. volume level, network status).
        }
        MSG_TRAY_DELETE => {
            // Tray icon removed.
        }
        MSG_TASKBAR_BUTTON_OVERLAY => {
            // Taskbar button overlay (badge/notification) changed.
        }
        _ => {}
    }

    // Components are mutated above via their public state setters as they are
    // added. Kept as parameters so dispatch can forward to each owner.
    let _ = taskbar;
    let _ = launcher;
    let _ = control_center;
    let _ = notifications;
}
