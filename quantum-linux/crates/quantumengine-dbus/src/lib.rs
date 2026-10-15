//! The D-Bus contract between `quantumengined` (the only process that
//! ever talks to the headset) and every other QuantumEngine client — the
//! window, `quantumenginectl`, and the Plasma widget.
//!
//! Defining the interface once, here, means the server and its clients
//! cannot drift apart: the `#[proxy]` macro below generates [`HeadsetProxy`]
//! (async) and [`HeadsetProxyBlocking`] straight from these method
//! signatures, so a client only ever needs `use quantumengine_dbus::*` — it
//! never hand-writes a matching call.
//!
//! # Contract
//!
//! Properties are cheap: the service answers them from an in-memory cache
//! kept fresh by its own worker thread, never by asking the headset while a
//! client waits. Read [`HeadsetProxyBlocking::link`] first — the other
//! properties hold stale or default values until it reads `"ready"`.
//!
//! Enum-shaped values are plain strings rather than a custom D-Bus type, so
//! they need no extra plumbing from QML or shell scripts:
//! * `Link`: `"no_dongle"` | `"no_access"` | `"headset_off"` | `"ready"`.
//! * `Anc`: `"off"` | `"on"` | `"talkthru"`.
//! * `Sidetone`: `"off"` | `"low"` | `"mid"` | `"high"`.
//!
//! The EQ, surround and update members don't involve the headset at all; they are on
//! the same object so a client needs only one proxy.
//!
//! The headset `Set*` methods block until the headset confirms the change (as
//! `jbl_quantum::Headset`'s own `set_*_confirmed` calls do) and fail with
//! `org.freedesktop.DBus.Error.Failed` if it doesn't. Only one command runs
//! at a time — the service serialises every caller through the same worker
//! thread that owns the headset, the same as a single in-process client
//! would.

use serde::{Deserialize, Serialize};
use zbus::proxy;
use zbus::zvariant::Type;

/// The well-known bus name `quantumengined` claims on the session bus.
pub const BUS_NAME: &str = "org.quantumengine.Headset1";
/// The single object every call and property lives on.
pub const OBJECT_PATH: &str = "/org/quantumengine/Headset1";

/// How many points [`HeadsetProxy::eq_response`] returns.
pub const EQ_RESPONSE_POINTS: usize = 64;

/// The frequency, in Hz, of point `i` of [`HeadsetProxy::eq_response`]: the
/// points are spaced evenly on a log scale from 20 Hz to 20 kHz, both
/// included.
pub fn eq_response_freq(i: usize) -> f64 {
    let t = i as f64 / (EQ_RESPONSE_POINTS - 1) as f64;
    20.0 * 1000f64.powf(t)
}

/// One lighting zone, for [`HeadsetProxy::set_lighting`]. Mirrors
/// `jbl_quantum::ZoneLighting`, but this crate never depends on that crate —
/// the service converts each side.
#[derive(Debug, Clone, PartialEq, Eq, Type, Serialize, Deserialize)]
pub struct ZoneLightingArg {
    /// `"logo"` or `"ring"`.
    pub zone: String,
    /// 0-100.
    pub brightness: u8,
    /// 1-5 `(r, g, b)` triples; the device animates through them.
    pub colours: Vec<(u8, u8, u8)>,
    /// The raw effect byte — see `jbl_quantum::protocol::Effect`'s named
    /// constants for the ones with known behaviour.
    pub effect: u8,
}

#[proxy(
    interface = "org.quantumengine.Headset1",
    default_service = "org.quantumengine.Headset1",
    default_path = "/org/quantumengine/Headset1",
    gen_blocking = true
)]
pub trait Headset {
    /// `"no_dongle"` | `"no_access"` | `"headset_off"` | `"ready"`. The other
    /// properties are only meaningful once this reads `"ready"`.
    #[zbus(property)]
    fn link(&self) -> zbus::Result<String>;

    /// The access error's message when [`link`](Self::link) is
    /// `"no_access"` (typically a missing udev rule); empty otherwise.
    #[zbus(property)]
    fn link_detail(&self) -> zbus::Result<String>;

    /// Battery percentage, 0-100.
    #[zbus(property)]
    fn battery(&self) -> zbus::Result<u8>;

    /// `"off"` | `"on"` | `"talkthru"`.
    #[zbus(property)]
    fn anc(&self) -> zbus::Result<String>;

    /// `"off"` | `"low"` | `"mid"` | `"high"`.
    #[zbus(property)]
    fn sidetone(&self) -> zbus::Result<String>;

    /// Whether the microphone is live: boom down and not muted.
    #[zbus(property)]
    fn mic_active(&self) -> zbus::Result<bool>;

    /// Game/chat dial position, 0 (full chat) to 16 (full game).
    #[zbus(property)]
    fn dial(&self) -> zbus::Result<u8>;

    #[zbus(property)]
    fn lights_on(&self) -> zbus::Result<bool>;

    /// The name the headset has for this host, e.g. `PC-myhost #1`.
    #[zbus(property)]
    fn device_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn serial(&self) -> zbus::Result<String>;

    /// Four `major.minor.patch` triplets.
    #[zbus(property)]
    fn firmware(&self) -> zbus::Result<Vec<String>>;

    /// Whether a `Set*` call is in progress. Other `Set*` calls still queue
    /// and wait rather than fail — this is only for showing a spinner.
    #[zbus(property)]
    fn busy(&self) -> zbus::Result<bool>;

    /// The last failed command's message, or empty. Cleared by the next
    /// command that succeeds.
    #[zbus(property)]
    fn last_error(&self) -> zbus::Result<String>;

    /// Sets noise control and waits for the headset to confirm it. `mode` is
    /// `"off"`, `"on"` or `"talkthru"`. Returns the confirmed mode.
    fn set_anc(&self, mode: &str) -> zbus::Result<String>;

    /// Sets sidetone and waits for it to read back. `level` is `"off"`,
    /// `"low"`, `"mid"` or `"high"`. Returns the confirmed level.
    fn set_sidetone(&self, level: &str) -> zbus::Result<String>;

    /// Switches the lights and waits for the headset to confirm it. Returns
    /// the confirmed state.
    fn set_lights(&self, on: bool) -> zbus::Result<bool>;

    /// Configures lighting for one or more zones and switches the lights on,
    /// as `jbl_quantum::Headset::set_lighting_zones` does — including the
    /// off/settle/on cycle that takes a few hundred milliseconds.
    fn set_lighting(&self, zones: Vec<ZoneLightingArg>) -> zbus::Result<()>;

    // Equalizer. Applied on the PC to the headset's game output by a PipeWire
    // filter; works whether or not the headset is connected right now.

    #[zbus(property)]
    fn eq_enabled(&self) -> zbus::Result<bool>;

    /// The sound profile's name, one of [`eq_profiles`](Self::eq_profiles).
    #[zbus(property)]
    fn eq_profile(&self) -> zbus::Result<String>;

    /// Every built-in profile name, in display order, `"Flat"` first.
    #[zbus(property)]
    fn eq_profiles(&self) -> zbus::Result<Vec<String>>;

    /// JBL's headset correction under the profile: `"off"` | `"stereo"` |
    /// `"surround"`.
    #[zbus(property)]
    fn eq_correction(&self) -> zbus::Result<String>;

    /// Why the EQ isn't working, or empty when it is (or is off).
    #[zbus(property)]
    fn eq_status(&self) -> zbus::Result<String>;

    fn set_eq_enabled(&self, enabled: bool) -> zbus::Result<bool>;

    /// Selects a profile by name, ignoring case and punctuation. Returns the
    /// profile's display name.
    fn set_eq_profile(&self, name: &str) -> zbus::Result<String>;

    fn set_eq_correction(&self, correction: &str) -> zbus::Result<String>;

    /// The gain, in dB, that `profile` under `correction` applies at each of
    /// the [`EQ_RESPONSE_POINTS`] frequencies [`eq_response_freq`] gives, for
    /// drawing the curve. It is the curve's shape: the overall turn-down that
    /// keeps a boosting curve from clipping is left out.
    fn eq_response(&self, profile: &str, correction: &str) -> zbus::Result<Vec<f64>>;

    // Virtual 7.1 surround: a PipeWire sink that renders eight channels
    // binaurally for the headset. Turning it on makes it the default output.

    #[zbus(property)]
    fn surround_enabled(&self) -> zbus::Result<bool>;

    /// Why surround isn't working (e.g. the dongle is unplugged), or empty.
    #[zbus(property)]
    fn surround_status(&self) -> zbus::Result<String>;

    fn set_surround_enabled(&self, enabled: bool) -> zbus::Result<bool>;

    // Updates from GitHub releases.

    /// The running service's version, e.g. `0.3.0`.
    #[zbus(property)]
    fn version(&self) -> zbus::Result<String>;

    /// A newer release's version, or empty.
    #[zbus(property)]
    fn update_version(&self) -> zbus::Result<String>;

    /// The newer release's web page, or empty.
    #[zbus(property)]
    fn update_url(&self) -> zbus::Result<String>;

    /// Whether [`install_update`](Self::install_update) can install it: this
    /// copy came from the pacman package and the release has one to install.
    #[zbus(property)]
    fn update_can_install(&self) -> zbus::Result<bool>;

    /// What the updater is doing or last failed at, for showing as-is.
    #[zbus(property)]
    fn update_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn update_busy(&self) -> zbus::Result<bool>;

    /// Whether the service checks for a new release once a day.
    #[zbus(property)]
    fn update_checks(&self) -> zbus::Result<bool>;

    /// Checks now. Returns the newer version, or empty when up to date.
    fn check_for_updates(&self) -> zbus::Result<String>;

    /// Starts installing the available update and returns at once; watch
    /// `UpdateStatus`. Shows a password prompt. On success the service
    /// restarts itself, so clients briefly lose it and reconnect.
    fn install_update(&self) -> zbus::Result<()>;

    fn set_update_checks(&self, enabled: bool) -> zbus::Result<bool>;
}
