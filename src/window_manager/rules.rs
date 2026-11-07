//! Per-app window rules.
//!
//! Match a window by class name and/or title (substring, case-insensitive) and
//! assign an action: float or tile. Optionally pin to a named workspace.
//! Rules are evaluated in order; the first matching rule wins.
//!
//! shortcut: substring match only, no regex/wildcard; upgrade when users ask for
//! pattern matching.

use std::collections::HashMap;

/// Action to take for a matching window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    /// Leave the window floating (never tile it).
    Float,
    /// Tile the window (default for unmatched windows).
    Tile,
}

/// A single per-app rule.
#[derive(Debug, Clone)]
pub struct AppRule {
    /// Substring matched against the window class name (case-insensitive).
    /// `None` matches any class.
    pub class: Option<String>,
    /// Substring matched against the window title (case-insensitive).
    /// `None` matches any title.
    pub title: Option<String>,
    /// Action to apply when the rule matches.
    pub action: RuleAction,
    /// Optional workspace name to pin matching windows to.
    pub workspace: Option<String>,
}

impl AppRule {
    /// Returns `true` if this rule matches the given class/title.
    pub fn matches(&self, class: &str, title: &str) -> bool {
        let class_ok = self
            .class
            .as_ref()
            .map(|p| class.to_lowercase().contains(&p.to_lowercase()))
            .unwrap_or(true);
        let title_ok = self
            .title
            .as_ref()
            .map(|p| title.to_lowercase().contains(&p.to_lowercase()))
            .unwrap_or(true);
        class_ok && title_ok
    }
}

/// Collection of app rules evaluated in order.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    rules: Vec<AppRule>,
}

impl Rules {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, rule: AppRule) {
        self.rules.push(rule);
    }

    /// Evaluate rules against a window; returns the action of the first match,
    /// or `None` if no rule matches (caller decides default).
    pub fn action_for(&self, class: &str, title: &str) -> Option<RuleAction> {
        self.rules
            .iter()
            .find(|r| r.matches(class, title))
            .map(|r| r.action)
    }

    /// Returns the workspace pin for the first matching rule, if any.
    pub fn workspace_for(&self, class: &str, title: &str) -> Option<String> {
        self.rules
            .iter()
            .find(|r| r.matches(class, title))
            .and_then(|r| r.workspace.clone())
    }
}

/// Default built-in rules: common dialogs/tools float, everything else tiles.
pub fn default_rules() -> Rules {
    let mut rules = Rules::new();
    // File pickers, property sheets, calculators etc. tend to be small dialogs.
    rules.add(AppRule {
        class: Some("#32770".into()), // Win32 dialog class
        title: None,
        action: RuleAction::Float,
        workspace: None,
    });
    rules.add(AppRule {
        class: Some("Shell_TrayWnd".into()),
        title: None,
        action: RuleAction::Float,
        workspace: None,
    });
    rules
}

/// Workspace name pinned per HWND (runtime state, not config).
pub type WorkspacePins = HashMap<isize, String>;
