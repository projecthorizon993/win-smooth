//! Hyper OS notification center: stacked notification cards.
//! App icon + timestamp + content, dismiss swipe, clear all.

use crate::shell_ui::design_tokens::*;

/// A single notification.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u64,
    pub app_name: String,
    pub app_icon: Option<Vec<u8>>,
    pub title: String,
    pub body: String,
    pub time: chrono::DateTime<chrono::Local>,
    pub dismissed: bool,
}

impl Notification {
    pub fn new(id: u64, app_name: String, title: String, body: String) -> Self {
        Self {
            id,
            app_name,
            app_icon: None,
            title,
            body,
            time: chrono::Local::now(),
            dismissed: false,
        }
    }
}

/// Notification center: holds the queue and renders stacked cards.
pub struct NotificationCenter {
    pub notifications: Vec<Notification>,
    pub visible: bool,
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            visible: false,
        }
    }

    pub fn push(&mut self, notif: Notification) {
        self.notifications.insert(0, notif);
    }

    pub fn dismiss(&mut self, id: u64) {
        if let Some(n) = self.notifications.iter_mut().find(|n| n.id == id) {
            n.dismissed = true;
        }
        self.notifications.retain(|n| !n.dismissed);
    }

    pub fn clear_all(&mut self) {
        self.notifications.clear();
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for NotificationCenter {
    fn default() -> Self {
        Self::new()
    }
}
