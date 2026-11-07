//! Window manager: tiling + floating for win-smooth.
//!
//! Runs with DWM (never replaces it). Enumerates top-level windows via
//! EnumWindows, classifies each as floating or tiling (per-app rules), skips
//! exclusive fullscreen games, grids tile the rest per monitor, and applies
//! all placements in one DeferWindowPos batch.
//!
//! shortcut: single-threaded layout pass per run() call; callers drive the
//! loop. Upgrade to a background thread + change-driven re-layout when
//! responsiveness needs it.

pub mod floating;
pub mod rules;
pub mod tiling;

use std::time::Duration;

use crate::window_manager::floating::Floating;
use crate::window_manager::rules::{AppRule, default_rules, RuleAction, Rules, WorkspacePins};
use crate::window_manager::tiling::{apply_layout, calculate_layout, enumerate_monitors,
    enumerate_windows, ManagedWindow, Monitor, Placement};
use windows::Win32::Foundation::HWND;
use windows::core::Result;

/// Snapshot of the full windowing state at one moment.
#[derive(Debug, Default)]
pub struct WindowState {
    pub monitors: Vec<Monitor>,
    pub windows: Vec<ManagedWindow>,
}

/// The window manager. Holds rules, floating set, and last-known state.
pub struct WindowManager {
    pub rules: Rules,
    pub floating: Floating,
    pub state: WindowState,
    /// workspace pin for each managed HWND (runtime only)
    pub workspace_pins: WorkspacePins,
}

impl WindowManager {
    pub fn new() -> Result<Self> {
        let monitors = enumerate_monitors()?;
        let windows = enumerate_windows()?;
        Ok(Self {
            rules: default_rules(),
            floating: Floating::new(),
            state: WindowState { monitors, windows },
            workspace_pins: WorkspacePins::new(),
        })
    }

    /// Refresh the window + monitor enumeration (call whenever a window is
    /// created, destroyed, or moved).
    pub fn refresh(&mut self) -> Result<()> {
        self.state.monitors = enumerate_monitors()?;
        self.state.windows = enumerate_windows()?;
        Ok(())
    }

    /// One layout pass: re-enumerate, classify, tile float/tiling, apply.
    /// Returns the placements applied.
    pub fn layout_once(&mut self) -> Result<Vec<Placement>> {
        self.refresh()?;
        let placements =
            calculate_layout(&self.state.windows, &self.floating, &self.rules, &self.state.monitors);
        apply_layout(&placements)?;
        Ok(placements)
    }

    /// Mark a window as floating (user-placed or rule-driven).
    pub fn float_window(&mut self, hwnd: HWND) {
        self.floating.insert(hwnd);
    }

    /// Mark a window as tiling-eligible.
    pub fn tile_window(&mut self, hwnd: HWND) {
        self.floating.remove(hwnd);
    }

    /// Classify + record a single window's desired mode without applying.
    pub fn classify(&mut self, hwnd: HWND, class: &str, title: &str) -> bool {
        let is_float = self
            .floating
            .classify(hwnd, class, title, &self.rules);
        if is_float {
            self.floating.insert(hwnd);
        } else {
            self.floating.remove(hwnd);
        }
        if let Some(ws) = self.rules.workspace_for(class, title) {
            self.workspace_pins.insert(hwnd.0 as isize, ws);
        }
        is_float
    }

    /// Look up a window's last known rect, if tracked.
    pub fn rect_for(&self, hwnd: HWND) -> Option<windows::Win32::Foundation::RECT> {
        self.state
            .windows
            .iter()
            .find(|w| w.hwnd == hwnd)
            .map(|w| w.rect)
    }

    /// Run the layout loop forever: re-enumerate, tile, apply, sleep.
    /// Intended to be spawned on a background thread.
    pub fn run(mut self, interval: Duration) -> ! {
        loop {
            let _ = self.layout_once();
            std::thread::sleep(interval);
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new().expect("window manager initial enumeration")
    }
}

/// Register a built-in per-app rule.
pub fn rule_float_class(class: &str) -> AppRule {
    AppRule {
        class: Some(class.to_string()),
        title: None,
        action: RuleAction::Float,
        workspace: None,
    }
}
