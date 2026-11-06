//! Glassmorphism panel component: translucent rounded rectangle with vibrant tint.

use crate::shell_ui::design_tokens::*;
use crate::shell_ui::hyper_os_theme::GlassPanel;

/// Describes a glass panel to be rendered by Direct2D.
#[derive(Debug, Clone, Copy)]
pub struct GlassPanelDesc {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub corner_radius: f32,
    pub style: GlassPanel,
}

impl Default for GlassPanelDesc {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 400.0,
            corner_radius: CORNER_RADIUS,
            style: GlassPanel::new(crate::shell_ui::theme::ThemeManager::default().current()),
        }
    }
}

impl GlassPanelDesc {
    pub fn taskbar(width: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height: TASKBAR_HEIGHT,
            corner_radius: CORNER_RADIUS,
            style: GlassPanel::new(crate::shell_ui::theme::ThemeManager::default().current()),
        }
    }
}
