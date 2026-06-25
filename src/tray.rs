//! System tray (menu bar) integration. Ported from `src/main/tray.js`.

use crate::constants::TRAY_ICON_BYTES;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Commands the tray menu can issue back to the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ToggleTimer,
    TriggerBreak,
    OpenSettings,
    Quit,
}

const ID_TOGGLE: &str = "toggle";
const ID_BREAK: &str = "trigger-break";
const ID_SETTINGS: &str = "settings";
const ID_QUIT: &str = "quit";

pub struct Tray {
    // Held to keep the tray icon alive for the lifetime of the app.
    _icon: TrayIcon,
    // Items whose label/state changes over time are kept so we can update them
    // *in place*. Rebuilding/replacing the whole menu while it is open would
    // dismiss it on macOS, so we never call `set_menu` after construction.
    status_item: MenuItem,
    toggle_item: MenuItem,
}

impl Tray {
    pub fn new(
        status: &str,
        toggle_label: &str,
        toggle_enabled: bool,
        tooltip: &str,
    ) -> Option<Self> {
        let status_item = MenuItem::with_id("status", status, false, None);
        let toggle_item = MenuItem::with_id(ID_TOGGLE, toggle_label, toggle_enabled, None);

        let menu = Menu::new();
        let _ = menu.append(&status_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&toggle_item);
        let _ = menu.append(&MenuItem::with_id(
            ID_BREAK,
            "🪳  立即开始休息 (召唤蟑螂)",
            true,
            None,
        ));
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id(ID_SETTINGS, "⚙  设置…", true, None));
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id(ID_QUIT, "❌  退出", true, None));

        let mut builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip);
        if let Some(icon) = load_icon() {
            builder = builder.with_icon(icon);
        }

        match builder.build() {
            Ok(icon) => Some(Self {
                _icon: icon,
                status_item,
                toggle_item,
            }),
            Err(e) => {
                eprintln!("Failed to create tray icon: {e}");
                None
            }
        }
    }

    /// Update the dynamic labels/tooltip in place (safe to call while the menu is open).
    pub fn refresh(&self, status: &str, toggle_label: &str, toggle_enabled: bool, tooltip: &str) {
        self.status_item.set_text(status);
        self.toggle_item.set_text(toggle_label);
        self.toggle_item.set_enabled(toggle_enabled);
        let _ = self._icon.set_tooltip(Some(tooltip));
    }
}

fn load_icon() -> Option<Icon> {
    let img = image::load_from_memory(TRAY_ICON_BYTES).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).ok()
}

/// Drain any pending tray menu click into a [`TrayCommand`].
pub fn poll_command() -> Option<TrayCommand> {
    let event = MenuEvent::receiver().try_recv().ok()?;
    match event.id.0.as_str() {
        ID_TOGGLE => Some(TrayCommand::ToggleTimer),
        ID_BREAK => Some(TrayCommand::TriggerBreak),
        ID_SETTINGS => Some(TrayCommand::OpenSettings),
        ID_QUIT => Some(TrayCommand::Quit),
        _ => None,
    }
}
