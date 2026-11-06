//! Toggle switch component for control center.

use crate::shell_ui::design_tokens::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToggleState {
    Off,
    On,
}

#[derive(Debug, Clone, Copy)]
pub struct ToggleDesc {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub state: ToggleState,
    pub accent: Color,
}

impl Default for ToggleDesc {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 48.0,
            height: 28.0,
            state: ToggleState::Off,
            accent: crate::shell_ui::design_tokens::light::ACCENT,
        }
    }
}
