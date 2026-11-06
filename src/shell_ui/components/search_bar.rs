//! Search bar component for app launcher.

use crate::shell_ui::design_tokens::*;

#[derive(Debug, Clone)]
pub struct SearchBarDesc {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: String,
    pub focused: bool,
}

impl Default for SearchBarDesc {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 48.0,
            text: String::new(),
            focused: false,
        }
    }
}
