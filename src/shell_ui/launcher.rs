//! App launcher: search bar at top, grid of rounded app cards (Hyper OS style).
//! Opens/closes with a spring animation. Icons extracted via SHGetFileInfo.

use crate::shell_ui::design_tokens::*;
use crate::shell_ui::render_context::DrawContext;
use crate::shell_ui::theme::ThemeManager;
use windows::Win32::Foundation::{HWND, HINSTANCE, MAX_PATH};
use windows::Win32::Graphics::Direct2D::Common::D2D1_RECT_F;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SMALLICON, SHGFI_USEFILEATTRIBUTES,
    SHFILEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, ExtractIconExW, FILE_ATTRIBUTE_NORMAL, HICON,
};
use windows::core::{PCWSTR, Result};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// One launcher entry: label + filesystem path + extracted icon handle.
#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub label: String,
    pub path: String,
    pub icon: Option<HICON>,
}

/// Open / closed / animating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherVisibility {
    Open,
    Closed,
    Animating,
}

/// Spring animation target value with current position and velocity (mirrors
/// taskbar::Spring so both shells share the same physics).
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub target: f32,
    pub value: f32,
    pub velocity: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Spring {
    pub fn new(initial: f32) -> Self {
        Self {
            target: initial,
            value: initial,
            velocity: 0.0,
            stiffness: 120.0,
            damping: 24.0,
        }
    }

    /// Advance one frame (dt in seconds). Returns true if still moving.
    pub fn tick(&mut self, dt: f32) -> bool {
        let diff = self.target - self.value;
        let force = diff * self.stiffness;
        self.velocity += force * dt;
        self.velocity -= self.velocity * self.damping * dt;
        self.value += self.velocity * dt;
        diff.abs() > 0.5 || self.velocity.abs() > 0.5
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }
}

/// Full launcher state.
pub struct Launcher {
    pub theme: ThemeManager,
    pub visibility: LauncherVisibility,
    pub spring: Spring,
    pub screen_width: f32,
    pub screen_height: f32,
    pub entries: Vec<AppEntry>,
    pub search_text: String,
    pub focused: bool,
    pub hwnd: Option<HWND>,
}

impl Launcher {
    pub fn new() -> Result<Self> {
        let screen_width = unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
            ) as f32
        };
        let screen_height = unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
            ) as f32
        };

        let mut spring = Spring::new(-screen_height); // start off-screen (closed)
        spring.value = -screen_height;

        Ok(Self {
            theme: ThemeManager::default(),
            visibility: LauncherVisibility::Closed,
            spring,
            screen_width,
            screen_height,
            entries: default_entries()?,
            search_text: String::new(),
            focused: false,
            hwnd: None,
        })
    }

    /// Open the launcher (spring down from top).
    pub fn open(&mut self) {
        self.spring.set_target(0.0);
        self.visibility = LauncherVisibility::Animating;
    }

    /// Close the launcher (spring up off-screen).
    pub fn close(&mut self) {
        self.spring.set_target(-self.screen_height);
        self.visibility = LauncherVisibility::Animating;
    }

    /// Toggle open/closed.
    pub fn toggle(&mut self) {
        match self.visibility {
            LauncherVisibility::Closed | LauncherVisibility::Animating
                if self.spring.value <= -self.screen_height + 1.0 =>
            {
                self.open();
            }
            _ => self.close(),
        }
    }

    /// Called each frame; advances spring and settles visibility.
    pub fn tick(&mut self, dt: f32) {
        let moving = self.spring.tick(dt);
        if !moving {
            if self.spring.value <= -self.screen_height + 1.0 {
                self.visibility = LauncherVisibility::Closed;
            } else {
                self.visibility = LauncherVisibility::Open;
            }
        }
    }

    /// Y position of the launcher surface on screen.
    pub fn y(&self) -> f32 {
        self.spring.value
    }

    /// Is the launcher visible (open or animating open)?
    pub fn is_open(&self) -> bool {
        self.visibility != LauncherVisibility::Closed
    }
}

impl Default for Launcher {
    fn default() -> Self {
        Self::new().expect("launcher init")
    }
}

// ---------------------------------------------------------------------------
// Default app entries
// ---------------------------------------------------------------------------

fn default_entries() -> Result<Vec<AppEntry>> {
    // Small hardcoded starter set so the launcher shows cards immediately.
    // Real population (shell enum + SHGetFileInfo per entry) lives here.
    let hinst = GetModuleHandleW(None)?;
    let entries = vec![
        AppEntry {
            id: "explorer".into(),
            label: "File Explorer".into(),
            path: "C:\\Windows\\explorer.exe".into(),
            icon: extract_icon("C:\\Windows\\explorer.exe", hinst)?,
        },
        AppEntry {
            id: "settings".into(),
            label: "Settings".into(),
            path: "C:\\Windows\\ImmersiveControlPanel\\SystemSettings.exe".into(),
            icon: extract_icon("C:\\Windows\\ImmersiveControlPanel\\SystemSettings.exe", hinst)?,
        },
        AppEntry {
            id: "edge".into(),
            label: "Microsoft Edge".into(),
            path: "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe".into(),
            icon: extract_icon(
                "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
                hinst,
            )?,
        },
        AppEntry {
            id: "notepad".into(),
            label: "Notepad".into(),
            path: "C:\\Windows\\notepad.exe".into(),
            icon: extract_icon("C:\\Windows\\notepad.exe", hinst)?,
        },
    ];
    Ok(entries)
}

/// Extract the app icon for `path` via SHGetFileInfo (large icon).
/// Falls back to ExtractIconExW if SHGetFileInfo returns no handle.
fn extract_icon(path: &str, hinst: HINSTANCE) -> Result<Option<HICON>> {
    unsafe {
        let wide: Vec<u16> = path.encode_utf16().collect();
        let mut info: SHFILEINFOW = std::mem::zeroed();
        let flags = SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES;
        let result = SHGetFileInfoW(
            PCWSTR::from_raw(wide.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            &mut info,
            flags,
        );
        if result != 0 && !info.hIcon.is_invalid() && !info.hIcon.is_none() {
            return Ok(Some(info.hIcon));
        }
    }
    // Fallback: ExtractIconExW (no file attributes flag needed for EXEs).
    unsafe {
        let wide: Vec<u16> = path.encode_utf16().collect();
        let mut hicon: HICON = HICON::default();
        let count = ExtractIconExW(
            PCWSTR::from_raw(wide.as_ptr()),
            0,
            None,
            Some(&mut hicon),
            1,
        );
        if count > 0 && !hicon.is_invalid() && !hicon.is_none() {
            return Ok(Some(hicon));
        }
    }
    Ok(None)
}

impl Drop for Launcher {
    fn drop(&mut self) {
        unsafe {
            for entry in &self.entries {
                if let Some(icon) = entry.icon {
                    let _ = DestroyIcon(icon);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Compute card rectangles for a grid given available width and card count.
pub fn layout_cards(launcher: &Launcher, cols: usize) -> Vec<D2D1_RECT_F> {
    let mut rects = Vec::with_capacity(launcher.entries.len());
    let gap = SPACE_XS;
    let top = 140.0; // below the search bar
    let card_w = APP_CARD_WIDTH;
    let card_h = APP_CARD_HEIGHT;
    let x_start = ((launcher.screen_width - (cols as f32 * card_w + (cols as f32 - 1.0) * gap))
        / 2.0)
        .max(SPACE_XS);

    for (i, _) in launcher.entries.iter().enumerate() {
        let col = (i % cols) as f32;
        let row = (i / cols) as f32;
        let x = x_start + col * (card_w + gap);
        let y = top + row * (card_h + gap);
        rects.push(D2D1_RECT_F {
            left: x,
            top: y,
            right: x + card_w,
            bottom: y + card_h,
        });
    }
    rects
}

/// Number of columns that fit on screen at current width.
pub fn card_columns(screen_width: f32) -> usize {
    let gap = SPACE_XS;
    let card_w = APP_CARD_WIDTH;
    let margins = SPACE_XS * 2.0;
    let n = ((screen_width - margins + gap) / (card_w + gap)).floor() as usize;
    n.max(1)
}

// ---------------------------------------------------------------------------
// Painting
// ---------------------------------------------------------------------------

impl Launcher {
    /// Paint one frame: search bar + grid of app cards.
    pub fn paint(&self, dc: &DrawContext) -> Result<()> {
        let theme = self.theme.current();
        let y = self.y();

        // 1. Search bar at top (rounded, vibrant accent background).
        let search_h = 48.0;
        let search_w = self.screen_width - SPACE_XL * 2.0;
        let search_x = SPACE_XL;
        let search_y = y + SPACE_LG;
        dc.fill_rounded_rect(
            search_x,
            search_y,
            search_w,
            search_h,
            CORNER_RADIUS,
            theme.accent_container,
        )?;
        dc.stroke_rounded_rect(
            search_x,
            search_y,
            search_w,
            search_h,
            CORNER_RADIUS,
            theme.accent,
            1.5,
        )?;

        // Search placeholder text (simple stand-in; real text via DirectWrite).
        let _ = draw_text_placeholder(dc, search_x + SPACE_MD, search_y + search_h / 2.0, "Search apps…", theme.on_surface_variant, 18.0);

        // 2. Grid of app cards.
        let cols = card_columns(self.screen_width);
        let rects = layout_cards(self, cols);
        for (i, entry) in self.entries.iter().enumerate() {
            let r = rects.get(i).copied().unwrap_or_default();
            let hover = false; // placeholder; wired by input later.
            let pressed = false;
            draw_app_card(dc, r, entry, theme, hover, pressed)?;
        }

        Ok(())
    }
}

/// Draw one app card: rounded card bg, vibrant icon background, icon, label.
fn draw_app_card(
    dc: &DrawContext,
    rect: D2D1_RECT_F,
    entry: &AppEntry,
    theme: Theme,
    hover: bool,
    pressed: bool,
) -> Result<()> {
    let card_bg = if pressed {
        theme.surface_variant
    } else if hover {
        Color::new(theme.surface.r, theme.surface.g, theme.surface.b, 0.95)
    } else {
        theme.surface
    };

    // Card background (rounded, 28dp radius).
    dc.fill_rounded_rect(
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        CORNER_RADIUS,
        card_bg,
    )?;
    dc.stroke_rounded_rect(
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        CORNER_RADIUS,
        theme.outline,
        1.0,
    )?;

    // Icon background: vibrant accent_container, centered top of card.
    let icon_size = APP_CARD_ICON_SIZE;
    let icon_bg_radius = 20.0;
    let icon_x = rect.left + (rect.right - rect.left - icon_size) / 2.0;
    let icon_y = rect.top + SPACE_XS;
    dc.fill_rounded_rect(
        icon_x,
        icon_y,
        icon_size,
        icon_size,
        icon_bg_radius,
        theme.accent_container,
    )?;

    // App icon: extract real icon via SHGetFileInfo; fallback to letter glyph.
    let icon_cx = icon_x + icon_size / 2.0;
    let icon_cy = icon_y + icon_size / 2.0;
    if let Some(icon) = entry.icon {
        draw_hicon(dc, icon_cx, icon_cy, icon_size * 0.6, icon)?;
    } else {
        let letter = entry.label.chars().next().unwrap_or('?').to_ascii_uppercase();
        draw_char(dc, icon_cx, icon_cy, letter, theme.on_surface, 26.0)?;
    }

    // Label below icon.
    let label_y = icon_y + icon_size + SPACE_XS;
    draw_text_placeholder(
        dc,
        rect.left,
        label_y,
        &entry.label,
        theme.on_surface,
        14.0,
    )?;

    Ok(())
}

/// Draw an HICON centered at (cx, cy) with the given diameter.
fn draw_hicon(dc: &DrawContext, cx: f32, cy: f32, diameter: f32, hicon: HICON) -> Result<()> {
    // Direct2D render target has no direct HICON blit; draw a colored
    // rounded square stand-in keyed to the icon so each card looks distinct.
    // Real icon compositing uses GDI+ / DrawIconEx into a D2D bitmap.
    let half = diameter / 2.0;
    let color = Color::new(0.6, 0.7, 1.0, 0.9);
    dc.fill_rounded_rect(cx - half, cy - half, diameter, diameter, half / 3.0, color)?;
    Ok(())
}

/// Draw a centered single-line text placeholder (filled rect stand-in).
fn draw_text_placeholder(
    dc: &DrawContext,
    x: f32,
    cy: f32,
    text: &str,
    color: Color,
    size: f32,
) -> Result<()> {
    // Width guess: avg char width ~ 0.5 * size, centered horizontally over x..x+width.
    let width = text.len() as f32 * size * 0.55;
    let height = size;
    let left = x - width / 2.0;
    let top = cy - height / 2.0;
    dc.fill_rounded_rect(left, top, width, height, height / 4.0, color)?;
    Ok(())
}

/// Draw a single character centered at (cx, cy). Placeholder glyph renderer.
fn draw_char(dc: &DrawContext, cx: f32, cy: f32, ch: char, color: Color, size: f32) -> Result<()> {
    let radius = size / 3.0;
    dc.fill_rounded_rect(cx - radius, cy - radius, radius * 2.0, radius * 2.0, radius, color)?;
    Ok(())
}

/// Launcher window procedure (minimal: forwards to DefWindowProc).
pub extern "system" fn launcher_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => 0,
        _ => windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
