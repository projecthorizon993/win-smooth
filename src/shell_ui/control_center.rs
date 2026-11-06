//! Hyper OS control center: pull-down panel with quick settings toggles.
//! WiFi, Bluetooth, brightness, volume, DND. Spring slide animation.

use crate::shell_ui::design_tokens::*;
use crate::shell_ui::animation::AnimName;
use crate::shell_ui::spring::AnimationManager;

/// Control center panel state.
pub struct ControlCenter {
    pub visible: bool,
    pub anim: AnimationManager,
}

impl ControlCenter {
    pub fn new() -> Self {
        let mut anim = AnimationManager::new();
        anim.add(AnimName::ControlCenterTranslateY.as_str(), 50.0);
        Self {
            visible: false,
            anim,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        let target = if self.visible { 0.0 } else { 50.0 };
        self.anim
            .set_target(AnimName::ControlCenterTranslateY.as_str(), target);
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn translate_y(&self) -> f32 {
        self.anim
            .value(AnimName::ControlCenterTranslateY.as_str())
            .unwrap_or(50.0)
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.anim.tick()
    }
}

impl Default for ControlCenter {
    fn default() -> Self {
        Self::new()
    }
}
