//! Theme manager: light/dark switching, system accent detection, theme change events.

use crate::shell_ui::design_tokens::{Theme, ThemeMode};

/// Global theme state. Single instance for the shell.
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current: Theme,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current: Theme::system(),
        }
    }

    pub fn current(&self) -> Theme {
        self.current
    }

    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.current = match mode {
            ThemeMode::Light => Theme::light(),
            ThemeMode::Dark => Theme::dark(),
        };
    }

    pub fn toggle(&mut self) {
        match self.current.mode {
            ThemeMode::Light => self.set_mode(ThemeMode::Dark),
            ThemeMode::Dark => self.set_mode(ThemeMode::Light),
        }
    }

    /// Detect system preference via Windows registry / DwmGetColorizationColor.
    /// Placeholder: defaults to light theme.
    pub fn system() -> Theme {
        // TODO: Query HKCU\Software\Microsoft\Windows\DWM\ColorizationColor
        // and AppsUseLightTheme registry value for real system detection.
        Theme::light()
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}
