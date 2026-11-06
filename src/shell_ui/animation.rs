//! Animation integration: connects spring physics to UI properties.
//! Taskbar show/hide, panel slide, card press.

use crate::shell_ui::spring::AnimationManager;

/// Named animations used across the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimName {
    TaskbarOpacity,
    TaskbarTranslateY,
    LauncherOpacity,
    LauncherTranslateY,
    ControlCenterTranslateY,
    CardScale,
}

impl AnimName {
    pub fn as_str(self) -> &'static str {
        match self {
            AnimName::TaskbarOpacity => "taskbar_opacity",
            AnimName::TaskbarTranslateY => "taskbar_translate_y",
            AnimName::LauncherOpacity => "launcher_opacity",
            AnimName::LauncherTranslateY => "launcher_translate_y",
            AnimName::ControlCenterTranslateY => "control_center_translate_y",
            AnimName::CardScale => "card_scale",
        }
    }
}

/// Starts all default animations at zero/hidden state.
pub fn init_defaults(mgr: &mut AnimationManager) {
    mgr.add(AnimName::TaskbarOpacity.as_str(), 0.0);
    mgr.add(AnimName::TaskbarTranslateY.as_str(), 50.0);
    mgr.add(AnimName::LauncherOpacity.as_str(), 0.0);
    mgr.add(AnimName::LauncherTranslateY.as_str(), 50.0);
    mgr.add(AnimName::ControlCenterTranslateY.as_str(), 50.0);
    mgr.add(AnimName::CardScale.as_str(), 0.9);
}
