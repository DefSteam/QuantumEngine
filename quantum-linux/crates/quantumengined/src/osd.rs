//! KDE Plasma's OSD for the headset's physical Game/Chat dial.
//!
//! Plasma exposes a text OSD over the session bus. It is intentionally
//! best-effort: other desktops, or a disabled Plasma OSD, simply show nothing.

use zbus::blocking::{Connection, Proxy};

const PLASMASHELL: &str = "org.kde.plasmashell";
const PATH: &str = "/org/kde/osdService";
const INTERFACE: &str = "org.kde.osdService";
const ICON: &str = "audio-headphones";

pub struct DialOsd {
    connection: Option<Connection>,
}

impl DialOsd {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection: Some(connection),
        }
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self { connection: None }
    }

    /// Shows the dial offset from the center position. The dial is 0 (all
    /// Chat) through 16 (all Game), with 8 balanced.
    pub fn show(&self, dial: u8) {
        let Some(connection) = &self.connection else {
            return;
        };
        let dial = dial.min(16);
        let label = match dial.cmp(&8) {
            std::cmp::Ordering::Greater => format!("Game +{}", dial - 8),
            std::cmp::Ordering::Less => format!("Chat +{}", 8 - dial),
            std::cmp::Ordering::Equal => "Game = Chat".to_owned(),
        };
        let Ok(proxy) = Proxy::new(connection, PLASMASHELL, PATH, INTERFACE) else {
            return;
        };
        // No reply is needed. In particular, don't let a desktop shell that is
        // shutting down stall the thread that reads headset events.
        drop(proxy.call_noreply("showText", &(ICON, label.as_str())));
    }
}
