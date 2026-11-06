//! Notification card component for notification center.

use crate::shell_ui::design_tokens::*;

#[derive(Debug, Clone)]
pub struct NotificationCardDesc {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub dismissed: bool,
    pub stack_index: usize,
}

impl Default for NotificationCardDesc {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 360.0,
            height: 96.0,
            app_name: String::new(),
            title: String::new(),
            body: String::new(),
            dismissed: false,
            stack_index: 0,
        }
    }
}
