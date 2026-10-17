//! Reopens the window at the size it was closed at, and maximized if it was.
//!
//! Sizes are stored in logical pixels and restored through the same window's
//! scale factor, so fractional display scales can't make the window grow on
//! each reopen. Restored sizes are clamped to the current screen. Position is
//! not saved because Wayland does not let an application place its window.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{LogicalSize, Manager, Runtime, WebviewWindow, WindowEvent};

const FILE: &str = "window.json";
/// Written by tauri-plugin-window-state, which this module replaces.
const OLD_FILE: &str = ".window-state.json";
/// The window's minimum size, as in `tauri.conf.json`.
const MIN: (f64, f64) = (360.0, 520.0);
/// The configured size, used if there is no resize event before closing.
const INITIAL: (f64, f64) = (1100.0, 740.0);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    /// Logical pixels, of the window when it isn't maximized.
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}

/// The last size the window had while not maximized, kept up to date as it is
/// resized so that closing it maximized still remembers its normal size.
#[derive(Default)]
pub struct Tracker(Mutex<Option<Saved>>);

fn path<R: Runtime>(window: &WebviewWindow<R>) -> Option<PathBuf> {
    window.app_handle().path().app_config_dir().ok()
}

/// `saved` fitted between the minimum size and `screen` (logical pixels).
pub fn fit(saved: Saved, screen: Option<(f64, f64)>) -> Saved {
    let (max_w, max_h) = screen.unwrap_or((3840.0, 2160.0));
    let clamp = |v: f64, lo: f64, hi: f64| {
        if v.is_finite() {
            v.clamp(lo, hi.max(lo))
        } else {
            lo
        }
    };
    Saved {
        width: clamp(saved.width, MIN.0, max_w),
        height: clamp(saved.height, MIN.1, max_h),
        ..saved
    }
}

/// Applies the saved size, then shows the window (it starts hidden so the
/// resize isn't seen).
pub fn restore<R: Runtime>(window: &WebviewWindow<R>) {
    let mut initial = Saved {
        width: INITIAL.0,
        height: INITIAL.1,
        maximized: false,
    };
    if let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) {
        let size = size.to_logical::<f64>(scale);
        initial.width = size.width;
        initial.height = size.height;
    }
    *window.state::<Tracker>().0.lock().unwrap() = Some(initial);

    if let Some(dir) = path(window) {
        // Old plugin state contains physical dimensions, which could expand
        // dramatically after a scale-factor change. Start from the new format.
        drop(std::fs::remove_file(dir.join(OLD_FILE)));
        let saved = std::fs::read(dir.join(FILE))
            .ok()
            .and_then(|b| serde_json::from_slice::<Saved>(&b).ok());
        if let Some(saved) = saved {
            let monitor = window
                .current_monitor()
                .ok()
                .flatten()
                .or_else(|| window.primary_monitor().ok().flatten());
            let screen = monitor.map(|m| {
                let size = m.size().to_logical::<f64>(m.scale_factor());
                (size.width, size.height)
            });
            let fitted = fit(saved, screen);
            drop(window.set_size(LogicalSize::new(fitted.width, fitted.height)));
            if fitted.maximized {
                drop(window.maximize());
            }
            *window.state::<Tracker>().0.lock().unwrap() = Some(Saved {
                maximized: false,
                ..fitted
            });
        }
    }
    drop(window.show());
}

/// Follows resizes, and saves on close.
pub fn on_event<R: Runtime>(window: &tauri::Window<R>, event: &WindowEvent) {
    let Some(webview) = window.app_handle().get_webview_window(window.label()) else {
        return;
    };
    match event {
        WindowEvent::Resized(_) => {
            if webview.is_maximized().unwrap_or(false) || webview.is_minimized().unwrap_or(false) {
                return;
            }
            if let (Ok(size), Ok(scale)) = (webview.inner_size(), webview.scale_factor()) {
                let size = size.to_logical::<f64>(scale);
                *webview.state::<Tracker>().0.lock().unwrap() = Some(Saved {
                    width: size.width,
                    height: size.height,
                    maximized: false,
                });
            }
        }
        WindowEvent::CloseRequested { .. } => {
            let Some(normal) = *webview.state::<Tracker>().0.lock().unwrap() else {
                return;
            };
            let saved = Saved {
                maximized: webview.is_maximized().unwrap_or(false),
                ..normal
            };
            if let Some(dir) = path(&webview) {
                if std::fs::create_dir_all(&dir).is_ok() {
                    if let Ok(data) = serde_json::to_vec_pretty(&saved) {
                        drop(std::fs::write(dir.join(FILE), data));
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(width: f64, height: f64) -> Saved {
        Saved {
            width,
            height,
            maximized: false,
        }
    }

    #[test]
    fn a_runaway_size_is_brought_back_to_the_screen() {
        let fitted = fit(
            Saved {
                width: 37232.0,
                height: 20512.0,
                maximized: true,
            },
            Some((2327.0, 1309.0)),
        );
        assert_eq!((fitted.width, fitted.height), (2327.0, 1309.0));
        assert!(fitted.maximized);
    }

    #[test]
    fn a_normal_size_is_kept_as_it_was() {
        assert_eq!(
            fit(saved(1100.0, 740.0), Some((2327.0, 1309.0))),
            saved(1100.0, 740.0)
        );
    }

    #[test]
    fn nothing_goes_below_the_minimum_or_through_a_broken_number() {
        assert_eq!(
            fit(saved(10.0, 10.0), Some((1920.0, 1080.0))),
            saved(360.0, 520.0)
        );
        assert_eq!(
            fit(saved(f64::NAN, f64::INFINITY), None),
            saved(360.0, 520.0)
        );
    }

    #[test]
    fn without_a_screen_it_still_has_a_ceiling() {
        assert_eq!(fit(saved(9000.0, 9000.0), None), saved(3840.0, 2160.0));
    }
}
