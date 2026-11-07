//! Tiling layout engine.
//!
//! For each monitor, collects the non-floating, non-fullscreen windows and
//! divides the monitor's work area into a grid. Applies placements with a
//! single DeferWindowPos batch (falling back to SetWindowPos per window).
//! Works with DWM — it only moves/resizes windows, never replaces the
//! compositor.
//!
//! shortcut: simple grid tiler (not a true BSP tree); upgrade to binary-space
//! partitioning when users request spiral/split layouts.

use crate::window_manager::floating::Floating;
use crate::window_manager::rules::Rules;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HMONITOR, HDC, MONITORINFO, MONITORINFOF_PRIMARY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, DeferWindowPos, EndDeferWindowPos, EnumWindows, GetClassNameW,
    GetWindowLongW, GetWindowTextW, GetWindowRect, GWL_STYLE, IsIconic, IsWindowVisible,
    SetWindowPos, SWP_FRAMECHANGED, SWP_NOZORDER, SWP_SHOWWINDOW, WS_CHILD, WS_POPUP, WS_VISIBLE,
    HWND_TOP, WNDENUMPROC,
};
use windows::core::{BOOL, Result};

/// A monitor and its usable area.
#[derive(Debug, Clone)]
pub struct Monitor {
    pub rect: RECT,      // full monitor rect
    pub work: RECT,      // work area (taskbar-reserved)
    pub is_primary: bool,
}

/// Snapshot of a top-level window relevant to tiling.
#[derive(Debug, Clone)]
pub struct ManagedWindow {
    pub hwnd: HWND,
    pub class: String,
    pub title: String,
    pub style: u32,
    pub rect: RECT,
}

impl ManagedWindow {
    /// Is this an exclusive fullscreen game? WS_POPUP style and the window
    /// rect covers an entire monitor.
    pub fn is_fullscreen(&self, monitors: &[Monitor]) -> bool {
        if (self.style & WS_POPUP.0) == 0 {
            return false;
        }
        monitors.iter().any(|m| {
            self.rect.left == m.rect.left
                && self.rect.top == m.rect.top
                && self.rect.right == m.rect.right
                && self.rect.bottom == m.rect.bottom
        })
    }

    /// Should this window be managed at all? Visible, top-level, not a child.
    pub fn is_manageable(&self) -> bool {
        (self.style & WS_VISIBLE.0) != 0 && (self.style & WS_CHILD.0) == 0
    }
}

/// Result of a layout pass: one rect per window to apply.
#[derive(Debug, Clone)]
pub struct Placement {
    pub hwnd: HWND,
    pub rect: RECT,
}

/// Enumerate all visible top-level windows via EnumWindows.
pub fn enumerate_windows() -> Result<Vec<ManagedWindow>> {
    let mut out: Vec<ManagedWindow> = Vec::new();
    let ptr: *mut Vec<ManagedWindow> = &mut out;
    let lparam = LPARAM(ptr as isize);
    unsafe {
        let callback: WNDENUMPROC = Some(enum_callback);
        EnumWindows(callback, lparam)?;
    }
    Ok(out)
}

unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows: &mut Vec<ManagedWindow> =
        unsafe { &mut *(lparam.0 as *mut Vec<ManagedWindow>) };

    // Skip invisible or minimized windows.
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return true.into();
    }
    if unsafe { IsIconic(hwnd) }.as_bool() {
        return true.into();
    }

    let mut class_buf = [0u16; 256];
    let mut title_buf = [0u16; 512];

    let class_len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
    let title_len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };

    let class = wide_to_string(&class_buf[..class_len as usize]);
    let title = wide_to_string(&title_buf[..title_len as usize]);

    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;

    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return true.into(); // skip windows we can't read
    }

    let win = ManagedWindow { hwnd, class, title, style, rect };
    if win.is_manageable() {
        windows.push(win);
    }
    true.into()
}

fn wide_to_string(wide: &[u16]) -> String {
    String::from_utf16_lossy(wide)
}

/// Enumerate display monitors via EnumDisplayMonitors.
pub fn enumerate_monitors() -> Result<Vec<Monitor>> {
    let mut out: Vec<Monitor> = Vec::new();
    let ptr: *mut Vec<Monitor> = &mut out;
    unsafe {
        let callback: windows::Win32::Graphics::Gdi::MONITORENUMPROC =
            Some(monitor_callback);
        let lparam = LPARAM(ptr as isize);
        windows::Win32::Graphics::Gdi::EnumDisplayMonitors(None, None, callback, lparam);
    }
    Ok(out)
}

unsafe extern "system" fn monitor_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors: &mut Vec<Monitor> =
        unsafe { &mut *(lparam.0 as *mut Vec<Monitor>) };
    let monitor_rect = unsafe { *rect };

    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT::default(),
        rcWork: RECT::default(),
        dwFlags: 0,
    };
    let got = unsafe { GetMonitorInfoW(hmonitor, &mut info) }.as_bool();
    let work = if got { info.rcWork } else { monitor_rect };
    let is_primary = got && (info.dwFlags & MONITORINFOF_PRIMARY) != 0;

    monitors.push(Monitor {
        rect: monitor_rect,
        work,
        is_primary,
    });
    true.into()
}

/// Calculate grid placements for all tiling windows across monitors.
pub fn calculate_layout(
    windows: &[ManagedWindow],
    floating: &Floating,
    rules: &Rules,
    monitors: &[Monitor],
) -> Vec<Placement> {
    let mut placements: Vec<Placement> = Vec::new();

    // Group tiling windows by monitor (nearest monitor to window rect center).
    let mut per_monitor: Vec<Vec<&ManagedWindow>> = monitors.iter().map(|_| Vec::new()).collect();

    for w in windows {
        if floating.classify(w.hwnd, &w.class, &w.title, rules) {
            continue;
        }
        if w.is_fullscreen(monitors) {
            continue; // don't tile exclusive fullscreen games
        }
        let idx = nearest_monitor(&w.rect, monitors);
        if idx < per_monitor.len() {
            per_monitor[idx].push(w);
        }
    }

    for (m, wins) in monitors.iter().zip(per_monitor.iter()) {
        placements.extend(grid_layout(m, wins));
    }

    placements
}

/// Grid-tile `windows` across monitor `m`'s work area.
fn grid_layout(m: &Monitor, wins: &[&ManagedWindow]) -> Vec<Placement> {
    let mut out: Vec<Placement> = Vec::new();
    let n = wins.len();
    if n == 0 {
        return out;
    }

    let cols = (n as f64).sqrt().ceil() as i32;
    let rows = (n as i32 + cols - 1) / cols;

    let cell_w = (m.work.right - m.work.left) / cols;
    let cell_h = (m.work.bottom - m.work.top) / rows;

    for (i, w) in wins.iter().enumerate() {
        let col = (i as i32) % cols;
        let row = (i as i32) / cols;
        let rect = RECT {
            left: m.work.left + col * cell_w,
            top: m.work.top + row * cell_h,
            right: m.work.left + (col + 1) * cell_w,
            bottom: m.work.top + (row + 1) * cell_h,
        };
        out.push(Placement {
            hwnd: (*w).hwnd,
            rect,
        });
    }
    out
}

/// Index of the monitor closest to the center of `rect`.
fn nearest_monitor(rect: &RECT, monitors: &[Monitor]) -> usize {
    let cx = (rect.left + rect.right) / 2;
    let cy = (rect.top + rect.bottom) / 2;
    monitors
        .iter()
        .enumerate()
        .min_by_key(|(_, m)| {
            let mcx = (m.rect.left + m.rect.right) / 2;
            let mcy = (m.rect.top + m.rect.bottom) / 2;
            (cx - mcx).abs() + (cy - mcy).abs()
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Apply placements with a single DeferWindowPos batch.
pub fn apply_layout(placements: &[Placement]) -> Result<()> {
    if placements.is_empty() {
        return Ok(());
    }

    let hdwp = unsafe { BeginDeferWindowPos(placements.len() as i32) }?;
    let mut last_err = Ok(());

    for p in placements {
        let flags = SWP_SHOWWINDOW | SWP_FRAMECHANGED;
        match unsafe {
            DeferWindowPos(
                hdwp,
                p.hwnd,
                Some(HWND_TOP),
                p.rect.left,
                p.rect.top,
                p.rect.right - p.rect.left,
                p.rect.bottom - p.rect.top,
                flags,
            )
        } {
            Ok(_) => {}
            Err(e) => {
                last_err = Err(e);
            }
        }
    }

    if let Err(e) = unsafe { EndDeferWindowPos(hdwp) } {
        return Err(e);
    }
    last_err
}

/// Fallback: apply a single window placement with SetWindowPos.
pub fn apply_single(hwnd: HWND, rect: &RECT) -> Result<()> {
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED | SWP_NOZORDER,
        )
    }
}
