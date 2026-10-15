use std::sync::{Arc, OnceLock};

/// Tells the D-Bus service and the tray icon that something they show has
/// changed. Handed out before they exist and filled in once they do, so the
/// EQ and update threads can be started first.
#[derive(Clone, Default)]
pub struct Notifier(Arc<OnceLock<Box<dyn Fn() + Send + Sync>>>);

impl Notifier {
    pub fn set(&self, f: impl Fn() + Send + Sync + 'static) {
        let _ = self.0.set(Box::new(f));
    }

    pub fn notify(&self) {
        if let Some(f) = self.0.get() {
            f();
        }
    }
}
