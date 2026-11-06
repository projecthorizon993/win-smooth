//! Hyper OS-inspired design system tokens for win-smooth shell.
//! Corner radius 28dp, glassmorphism, vibrant accents, card-based layout.

use windows::Win32::Foundation::COLORREF;

/// Corner radius in device-independent pixels (Hyper OS signature: 28dp)
pub const CORNER_RADIUS: f32 = 28.0;

/// Base spacing scale (8dp grid)
pub const SPACE_XS: f32 = 8.0;
pub const SPACE_SM: f32 = 16.0;
pub const SPACE_MD: f32 = 24.0;
pub const SPACE_LG: f32 = 32.0;
pub const SPACE_XL: f32 = 48.0;

/// Panel elevation shadow blur
pub const SHADOW_BLUR: f32 = 20.0;
pub const SHADOW_OFFSET_Y: f32 = 8.0;

/// Glassmorphism: translucent background alpha (0.0-1.0)
pub const GLASS_ALPHA: f32 = 0.75;
/// Glass border alpha
pub const GLASS_BORDER_ALPHA: f32 = 0.15;
/// Glass tint strength from accent color
pub const GLASS_TINT_STRENGTH: f32 = 0.10;

/// Animation durations (milliseconds) — spring-based, no easing curves
pub const ANIM_FAST: u32 = 200;
pub const ANIM_NORMAL: u32 = 300;
pub const ANIM_SLOW: u32 = 450;

/// Taskbar height
pub const TASKBAR_HEIGHT: f32 = 56.0;
/// Taskbar icon size
pub const TASKBAR_ICON_SIZE: f32 = 24.0;

/// App card dimensions for launcher grid
pub const APP_CARD_WIDTH: f32 = 100.0;
pub const APP_CARD_HEIGHT: f32 = 116.0;
pub const APP_CARD_ICON_SIZE: f32 = 44.0;

// ---------------------------------------------------------------------------
// Color palette — Hyper OS vibrant but restrained
// ---------------------------------------------------------------------------

/// sRGB color with alpha
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Convert to Windows COLORREF (ignores alpha)
    pub fn to_colorref(self) -> COLORREF {
        COLORREF(
            (self.r * 255.0) as u32
                | ((self.g * 255.0) as u32) << 8
                | ((self.b * 255.0) as u32) << 16,
        )
    }
}

/// Light theme colors (Hyper OS light)
pub mod light {
    use super::Color;

    pub const BG: Color = Color::new(0.96, 0.96, 0.97, 1.0);
    pub const SURFACE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
    pub const SURFACE_VARIANT: Color = Color::new(0.92, 0.92, 0.94, 1.0);
    pub const ON_SURFACE: Color = Color::new(0.12, 0.12, 0.14, 1.0);
    pub const ON_SURFACE_VARIANT: Color = Color::new(0.45, 0.45, 0.48, 1.0);
    pub const OUTLINE: Color = Color::new(0.78, 0.78, 0.80, 1.0);

    /// Vibrant accent — Hyper OS style blue-violet
    pub const ACCENT: Color = Color::new(0.30, 0.55, 1.0, 1.0);
    pub const ACCENT_CONTAINER: Color = Color::new(0.90, 0.95, 1.0, 1.0);
    pub const ACCENT_VIBRANT: Color = Color::new(0.40, 0.65, 1.0, 0.15);

    pub const SUCCESS: Color = Color::new(0.20, 0.70, 0.35, 1.0);
    pub const WARNING: Color = Color::new(1.0, 0.75, 0.15, 1.0);
    pub const ERROR: Color = Color::new(1.0, 0.25, 0.25, 1.0);
    pub const INFO: Color = Color::new(0.25, 0.60, 0.95, 1.0);
}

/// Dark theme colors (Hyper OS dark)
pub mod dark {
    use super::Color;

    pub const BG: Color = Color::new(0.06, 0.06, 0.07, 1.0);
    pub const SURFACE: Color = Color::new(0.10, 0.10, 0.11, 1.0);
    pub const SURFACE_VARIANT: Color = Color::new(0.15, 0.15, 0.17, 1.0);
    pub const ON_SURFACE: Color = Color::new(0.95, 0.95, 0.97, 1.0);
    pub const ON_SURFACE_VARIANT: Color = Color::new(0.70, 0.70, 0.73, 1.0);
    pub const OUTLINE: Color = Color::new(0.28, 0.28, 0.30, 1.0);

    pub const ACCENT: Color = Color::new(0.45, 0.70, 1.0, 1.0);
    pub const ACCENT_CONTAINER: Color = Color::new(0.15, 0.20, 0.35, 1.0);
    pub const ACCENT_VIBRANT: Color = Color::new(0.55, 0.80, 1.0, 0.12);

    pub const SUCCESS: Color = Color::new(0.30, 0.80, 0.45, 1.0);
    pub const WARNING: Color = Color::new(1.0, 0.85, 0.25, 1.0);
    pub const ERROR: Color = Color::new(1.0, 0.35, 0.35, 1.0);
    pub const INFO: Color = Color::new(0.35, 0.70, 1.0, 1.0);
}

// ---------------------------------------------------------------------------
// Theme context
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub bg: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub accent: Color,
    pub accent_container: Color,
    pub accent_vibrant: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg: light::BG,
            surface: light::SURFACE,
            surface_variant: light::SURFACE_VARIANT,
            on_surface: light::ON_SURFACE,
            on_surface_variant: light::ON_SURFACE_VARIANT,
            outline: light::OUTLINE,
            accent: light::ACCENT,
            accent_container: light::ACCENT_CONTAINER,
            accent_vibrant: light::ACCENT_VIBRANT,
            success: light::SUCCESS,
            warning: light::WARNING,
            error: light::ERROR,
            info: light::INFO,
        }
    }

    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg: dark::BG,
            surface: dark::SURFACE,
            surface_variant: dark::SURFACE_VARIANT,
            on_surface: dark::ON_SURFACE,
            on_surface_variant: dark::ON_SURFACE_VARIANT,
            outline: dark::OUTLINE,
            accent: dark::ACCENT,
            accent_container: dark::ACCENT_CONTAINER,
            accent_vibrant: dark::ACCENT_VIBRANT,
            success: dark::SUCCESS,
            warning: dark::WARNING,
            error: dark::ERROR,
            info: dark::INFO,
        }
    }

    /// Detect system accent color from Windows (placeholder — query via DwmGetColorizationColor later)
    pub fn system_accent() -> Color {
        // Hyper OS borrows the user's Windows accent; default to vibrant blue-violet
        light::ACCENT
    }
}
