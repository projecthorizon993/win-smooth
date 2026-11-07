//! Floating mode.
//!
//! Tracks which windows should float (user-placed or rule-matched) and leaves
//! them alone — the tiler never moves or resizes a floating window.
//!
//! shortcut: floating state lives only in memory, not persisted across restarts;
//! upgrade when a config file is added.

use std::collections::HashSet;

use crate::window_manager::rules::{RuleAction, Rules};
use windows::Win32::Foundation::HWND;

/// Set of floating windows, keyed by the raw HWND pointer as `usize`
/// (`HWND` is `pub struct HWND(pub *mut c_void)` and does not implement
/// `Hash`/`Eq`, so it cannot be a `HashSet` key directly).
#[derive(Debug, Default, Clone)]
pub struct Floating {
    floating: HashSet<usize>,
}

impl Floating {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a window as floating (never tile it).
    pub fn insert(&mut self, hwnd: HWND) {
        self.floating.insert(hwnd.0 as usize);
    }

    /// Remove a window from floating (it becomes eligible for tiling).
    pub fn remove(&mut self, hwnd: HWND) {
        self.floating.remove(&(hwnd.0 as usize));
    }

    /// Returns `true` if the window is floating.
    pub fn contains(&self, hwnd: HWND) -> bool {
        self.floating.contains(&(hwnd.0 as usize))
    }

    /// Number of floating windows.
    pub fn len(&self) -> usize {
        self.floating.len()
    }
    pub fn is_empty(&self) -> bool {
        self.floating.is_empty()
    }

    /// Classify a window: float if it matches a float rule, or is already
    /// marked floating. Tile otherwise.
    pub fn classify(&self, hwnd: HWND, class: &str, title: &str, rules: &Rules) -> bool {
        if self.contains(hwnd) {
            return true;
        }
        matches!(
            rules.action_for(class, title),
            Some(RuleAction::Float)
        )
    }
}
