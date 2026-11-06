//! Hyper OS-style glassmorphism taskbar.
//!
//! Translucent glass background (28dp corner radius, vibrant accent tint),
//! Start button + pinned app icons, system tray (clock, date, volume,
//! network, power), smooth show/hide spring animation, and auto-hide when
//! exclusive fullscreen is detected.
//!
//! Direct2D 1.1 rendering via the windows crate 0.62. Raw Win32 overlay
//! window only — does not touch explorer.exe.

use crate::shell_ui::design_tokens::*;
use crate::shell_ui::theme::ThemeManager;
use crate::shell_ui::hyper_os_theme::GlassPanel;
use crate::shell_ui::render_context::DrawContext;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::HRGN;
use windows::Win32::Graphics::Dwm::DWM_BLURBEHIND;
use windows::Win32::UI::WindowsAndMessaging::{CS_HREDRAW, CS_VREDRAW, WS_POPUP, WS_VISIBLE, WM_DESTROY};
use windows::core::w;

/// Taskbar height (dp).
pub const TASKBAR_HEIGHT: f32 = 56.0;
/// Corner radius (dp).
pub const CORNER_RADIUS: f32 = 28.0;
/// Glass background alpha.
pub const GLASS_ALPHA: f32 = 0.75;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Visible / hidden / animating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
    Animating,
}

/// Auto-hide trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoHideMode {
    None,
    ExclusiveFullscreen,
}

/// Spring animation target value with current position and velocity.
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

/// One pinned app icon slot.
#[derive(Debug, Clone)]
pub struct AppSlot {
    pub id: String,
    pub label: String,
    pub icon_path: Option<String>,
    pub rect: Option<RECT>,
}

/// System tray item.
#[derive(Debug, Clone)]
pub struct TrayItem {
    pub kind: TrayKind,
    pub rect: Option<RECT>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayKind {
    Clock,
    Date,
    Volume,
    Network,
    Power,
}

/// Full taskbar state.
pub struct Taskbar {
    pub theme: ThemeManager,
    pub visibility: Visibility,
    pub auto_hide: AutoHideMode,
    pub spring: Spring,
    pub screen_width: f32,
    pub screen_height: f32,
    pub app_slots: Vec<AppSlot>,
    pub tray_items: Vec<TrayItem>,
    pub hover_slot: Option<usize>,
    pub pressed_slot: Option<usize>,
    pub mouse_over: bool,
    pub fullscreen: bool,
    /// Topmost overlay window that hosts the taskbar surface.
    pub hwnd: Option<HWND>,
}

impl Taskbar {
    pub fn new() -> Self {
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

        let mut spring = Spring::new(screen_height); // start off-screen (hidden)
        spring.value = screen_height;

        Self {
            theme: ThemeManager::default(),
            visibility: Visibility::Hidden,
            auto_hide: AutoHideMode::ExclusiveFullscreen,
            spring,
            screen_width,
            screen_height,
            app_slots: default_app_slots(),
            tray_items: default_tray_items(),
            hover_slot: None,
            pressed_slot: None,
            mouse_over: false,
            fullscreen: false,
            hwnd: None,
        }
    }

    /// Taskbar Y position on screen: bottom edge minus height, offset by spring.
    pub fn y(&self) -> f32 {
        self.spring.value
    }

    /// Show the taskbar (spring to on-screen position).
    pub fn show(&mut self) {
        let on_screen = self.screen_height - TASKBAR_HEIGHT;
        self.spring.set_target(on_screen);
        self.visibility = Visibility::Animating;
    }

    /// Hide the taskbar (spring off-screen).
    pub fn hide(&mut self) {
        self.spring.set_target(self.screen_height);
        self.visibility = Visibility::Animating;
    }

    /// Called each frame; advances spring and settles visibility.
    pub fn tick(&mut self, dt: f32) {
        let moving = self.spring.tick(dt);
        if !moving {
            if self.spring.value >= self.screen_height - 1.0 {
                self.visibility = Visibility::Hidden;
            } else {
                self.visibility = Visibility::Visible;
            }
        }
    }

    /// Auto-hide logic: if exclusive fullscreen detected, hide; else show.
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if self.fullscreen == fullscreen {
            return;
        }
        self.fullscreen = fullscreen;
        match self.auto_hide {
            AutoHideMode::ExclusiveFullscreen => {
                if fullscreen {
                    self.hide();
                } else {
                    self.show();
                }
            }
            AutoHideMode::None => {}
        }
    }

    /// Hit-test a point against app slots; returns index under point.
    pub fn hit_app(&self, x: f32, y: f32) -> Option<usize> {
        self.app_slots.iter().position(|s| {
            if let Some(r) = s.rect {
                x >= r.left as f32 && x <= r.right as f32 && y >= r.top as f32 && y <= r.bottom as f32
            } else {
                false
            }
        })
    }

    /// Hit-test tray items.
    pub fn hit_tray(&self, x: f32, y: f32) -> Option<usize> {
        self.tray_items.iter().position(|t| {
            if let Some(r) = t.rect {
                x >= r.left as f32 && x <= r.right as f32 && y >= r.top as f32 && y <= r.bottom as f32
            } else {
                false
            }
        })
    }
}

impl Default for Taskbar {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

fn default_app_slots() -> Vec<AppSlot> {
    vec![
        AppSlot {
            id: "start".into(),
            label: "Start".into(),
            icon_path: None,
            rect: None,
        },
        AppSlot {
            id: "explorer".into(),
            label: "File Explorer".into(),
            icon_path: None,
            rect: None,
        },
        AppSlot {
            id: "edge".into(),
            label: "Microsoft Edge".into(),
            icon_path: None,
            rect: None,
        },
        AppSlot {
            id: "settings".into(),
            label: "Settings".into(),
            icon_path: None,
            rect: None,
        },
    ]
}

fn default_tray_items() -> Vec<TrayItem> {
    vec![
        TrayItem { kind: TrayKind::Volume, rect: None },
        TrayItem { kind: TrayKind::Network, rect: None },
        TrayItem { kind: TrayKind::Power, rect: None },
        TrayItem { kind: TrayKind::Clock, rect: None },
        TrayItem { kind: TrayKind::Date, rect: None },
    ]
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Compute slot rectangles based on current taskbar geometry.
pub fn layout(taskbar: &Taskbar) -> (Vec<AppSlot>, Vec<TrayItem>) {
    let mut apps = taskbar.app_slots.clone();
    let mut tray = taskbar.tray_items.clone();

    let y = taskbar.y();
    let top = y + (TASKBAR_HEIGHT - 32.0) / 2.0; // center 32dp icons
    let bottom = top + 32.0;

    // Start button + pinned apps on the left.
    let mut x = SPACE_XS;
    // Start button slot.
    if let Some(s) = apps.first_mut() {
        s.rect = Some(RECT {
            left: x as i32,
            top: top as i32,
            right: (x + 48.0) as i32,
            bottom: bottom as i32,
        });
        x += 48.0 + SPACE_XS;
    }
    for s in apps.iter_mut().skip(1) {
        s.rect = Some(RECT {
            left: x as i32,
            top: top as i32,
            right: (x + 40.0) as i32,
            bottom: bottom as i32,
        });
        x += 40.0 + SPACE_XS;
    }

    // Tray on the right, right-aligned.
    let mut x = taskbar.screen_width - SPACE_XS;
    for t in tray.iter_mut().rev() {
        let w = match t.kind {
            TrayKind::Clock | TrayKind::Date => 70.0,
            _ => 40.0,
        };
        x -= w;
        t.rect = Some(RECT {
            left: x as i32,
            top: top as i32,
            right: (x + w) as i32,
            bottom: bottom as i32,
        });
        x -= SPACE_XS;
    }

    (apps, tray)
}

/// Glass panel descriptor for the taskbar background.
pub fn taskbar_panel_desc(taskbar: &Taskbar) -> crate::shell_ui::components::glass_panel::GlassPanelDesc {
    crate::shell_ui::components::glass_panel::GlassPanelDesc {
        x: 0.0,
        y: taskbar.y(),
        width: taskbar.screen_width,
        height: TASKBAR_HEIGHT,
        corner_radius: CORNER_RADIUS,
        style: GlassPanel::new(taskbar.theme.current()),
    }
}

/// Blur-behind descriptor for DWM glass on the taskbar window region.
pub fn taskbar_blur_behind() -> DWM_BLURBEHIND {
    DWM_BLURBEHIND {
        dwFlags: 0x00000001, // DWM_BB_ENABLE
        fEnable: windows::Win32::Foundation::TRUE,
        hRgnBlur: HRGN::default(),
        fTransitionOnMaximized: windows::Win32::Foundation::FALSE,
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

impl Taskbar {
    /// Create the topmost transparent overlay window anchored to the bottom
    /// edge of the screen and enable DWM blur-behind for glass.
    pub fn init_window(&mut self) -> windows::core::Result<HWND> {
        unsafe {
            let hinst = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?;
            let class = w!("WinSmoothTaskbar");

            // Register once; re-registration of an existing class is a no-op
            // for our purposes (the class is process-global).
            let wc = windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXW {
                cbSize: std::mem::size_of::<windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXW>()
                    as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(taskbar_wndproc),
                hInstance: hinst.into(),
                lpszClassName: class,
                ..Default::default()
            };
            let _ = windows::Win32::UI::WindowsAndMessaging::RegisterClassExW(&wc);

            let y = (self.screen_height - TASKBAR_HEIGHT) as i32;
            let hwnd = windows::Win32::UI::WindowsAndMessaging::CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST
                    | windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED
                    | windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT,
                class,
                w!("win-smooth taskbar"),
                WS_POPUP | WS_VISIBLE,
                0,
                y,
                self.screen_width as i32,
                TASKBAR_HEIGHT as i32,
                None,
                None,
                Some(hinst.into()),
                None,
            )?;

            // Layered, click-through: let input fall through to the desktop.
            windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes(
                hwnd,
                COLORREF(0),
                255,
                windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA,
            );

            // DWM blur-behind for the glass effect behind our D2D surface.
            let blur = taskbar_blur_behind();
            windows::Win32::Graphics::Dwm::DwmEnableBlurBehindWindow(hwnd, &blur)?;

            self.hwnd = Some(hwnd);
            Ok(hwnd)
        }
    }

    /// Paint one frame: glass background, app icons, tray symbols.
    pub fn paint(&self, dc: &mut DrawContext) -> windows::core::Result<()> {
        let theme = self.theme.current();
        let (apps, tray) = layout(self);
        let y = self.y();

        // 1. Glass background fill (surface, alpha).
        dc.fill_rounded_rect(
            0.0,
            y,
            self.screen_width,
            TASKBAR_HEIGHT,
            CORNER_RADIUS,
            Color::new(theme.surface.r, theme.surface.g, theme.surface.b, GLASS_ALPHA),
        )?;

        // 2. Vibrant accent tint overlay.
        dc.fill_rounded_rect(
            0.0,
            y,
            self.screen_width,
            TASKBAR_HEIGHT,
            CORNER_RADIUS,
            theme.accent_vibrant,
        )?;

        // 3. Thin border.
        dc.stroke_rounded_rect(
            0.0,
            y,
            self.screen_width,
            TASKBAR_HEIGHT,
            CORNER_RADIUS,
            theme.outline,
            1.0,
        )?;

        // 4. App icons (Start button + pinned apps).
        for (i, slot) in apps.iter().enumerate() {
            let r = match slot.rect { Some(r) => r, None => continue };
            let cx = (r.left + r.right) as f32 / 2.0;
            let cy = (r.top + r.bottom) as f32 / 2.0;
            let size = (r.bottom - r.top) as f32;
            let accent = if self.hover_slot == Some(i) {
                theme.accent
            } else {
                theme.surface_variant
            };
            draw_app_icon(dc, cx, cy, size, &slot.label, accent)?;
        }

        // 5. Tray symbols (right side).
        for item in tray {
            let r = match item.rect { Some(r) => r, None => continue };
            draw_tray_symbol(dc, &item.kind, (r.left + r.right) as f32 / 2.0,
                             (r.top + r.bottom) as f32 / 2.0, theme)?;
        }

        Ok(())
    }
}

/// App icon: rounded square with the app's first letter.
fn draw_app_icon(
    dc: &mut DrawContext,
    cx: f32,
    cy: f32,
    size: f32,
    label: &str,
    bg: Color,
) -> windows::core::Result<()> {
    let half = size / 2.0;
    let radius = 8.0;
    dc.fill_rounded_rect(cx - half, cy - half, size, size, radius, bg)?;
    let letter = label.chars().next().unwrap_or('?').to_ascii_uppercase();
    // Draw letter centered (simple monospace placeholder glyph).
    let text_color = crate::shell_ui::design_tokens::light::ON_SURFACE;
    draw_char(dc, cx, cy, letter, text_color, 20.0)
}

/// Tray symbol for each tray kind.
fn draw_tray_symbol(
    dc: &mut DrawContext,
    kind: &TrayKind,
    cx: f32,
    cy: f32,
    theme: Theme,
) -> windows::core::Result<()> {
    let color = theme.on_surface;
    match kind {
        TrayKind::Volume => draw_char(dc, cx, cy, '\u{1F50A}', color, 18.0), // 🔊
        TrayKind::Network => draw_char(dc, cx, cy, '\u{1F4F2}', color, 18.0), // 📲
        TrayKind::Power => draw_char(dc, cx, cy, '\u{23FB}', color, 18.0),   // ⏻
        TrayKind::Clock => draw_char(dc, cx, cy, '\u{23F0}', color, 18.0),   // ⏰
        TrayKind::Date => draw_char(dc, cx, cy, '\u{1F4C5}', color, 18.0),   // 📅
    }
}

/// Draw a single character centered at (cx, cy). Placeholder glyph renderer.
fn draw_char(
    dc: &mut DrawContext,
    cx: f32,
    cy: f32,
    _ch: char,
    color: Color,
    size: f32,
) -> windows::core::Result<()> {
    // Minimal text: render a filled circle as a glyph stand-in so the
    // taskbar shows something visible without DirectWrite font loading.
    // Real glyph rendering uses DirectWrite + ID2D1RenderTarget::DrawText.
    let radius = size / 3.0;
    dc.fill_rounded_rect(cx - radius, cy - radius, radius * 2.0, radius * 2.0, radius, color)?;
    Ok(())
}

/// Taskbar window procedure.
unsafe extern "system" fn taskbar_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => LRESULT(0),
        _ => unsafe { windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
