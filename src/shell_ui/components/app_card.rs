//! App card component: rounded card with app icon and label (Hyper OS launcher style).

use crate::shell_ui::design_tokens::*;
use crate::shell_ui::hyper_os_theme::AppCardStyle;

/// Describes an app card for the launcher grid.
#[derive(Debug, Clone)]
pub struct AppCardDesc {
    pub x: f32,
    pub y: f32,
    pub icon_path: Option<String>,
    pub label: String,
    pub style: AppCardStyle,
}

impl AppCardDesc {
    pub fn new(x: f32, y: f32, label: String) -> Self {
        Self {
            x,
            y,
            icon_path: None,
            label,
            style: AppCardStyle::new(crate::shell_ui::theme::ThemeManager::default().current()),
        }
    }
}
