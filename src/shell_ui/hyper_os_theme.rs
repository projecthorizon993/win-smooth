//! Hyper OS-specific styling helpers: glass panels, vibrant accent tinting,
//! card shadows, and rounded-surface drawing primitives.

use crate::shell_ui::design_tokens::*;

/// Hyper OS glass panel background descriptor.
#[derive(Debug, Clone, Copy)]
pub struct GlassPanel {
    pub base: Color,
    pub border: Color,
    pub tint: Color,
}

impl GlassPanel {
    pub fn new(theme: Theme) -> Self {
        Self {
            base: theme.surface,
            border: theme.outline,
            tint: theme.accent_vibrant,
        }
    }
}

/// Hyper OS app card: rounded rectangle with vibrant icon background.
#[derive(Debug, Clone, Copy)]
pub struct AppCardStyle {
    pub card_bg: Color,
    pub card_border: Color,
    pub icon_bg: Color,
    pub icon_radius: f32,
}

impl AppCardStyle {
    pub fn new(theme: Theme) -> Self {
        Self {
            card_bg: theme.surface,
            card_border: theme.outline,
            icon_bg: theme.accent_container,
            icon_radius: 16.0,
        }
    }
}

/// Draw a Hyper OS glass panel outline + fill (to be implemented in Direct2D).
pub struct GlassPainter;

impl GlassPainter {
    /// Returns the layered drawing order for a glass panel:
    /// 1. Base fill (surface color, alpha)
    /// 2. Vibrant tint overlay (accent_vibrant, low alpha)
    /// 3. Border stroke (outline, thin)
    pub fn layers(theme: Theme) -> Vec<GlassLayer> {
        vec![
            GlassLayer::Fill { color: theme.surface },
            GlassLayer::Tint { color: theme.accent_vibrant },
            GlassLayer::Border { color: theme.outline },
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GlassLayer {
    Fill { color: Color },
    Tint { color: Color },
    Border { color: Color },
}
