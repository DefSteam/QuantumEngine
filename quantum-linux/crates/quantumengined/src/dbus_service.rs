//! The D-Bus service `quantumengined` runs. See `quantumengine-dbus`
//! for the interface contract this implements.
//!
//! [`HeadsetIface`] never touches the headset directly: property getters
//! read the cache the [`device`] worker thread keeps up to date, and the
//! `Set*` methods send a [`device::Command`] down the same channel the
//! window would, then block on its reply. That keeps every caller — this
//! service's own D-Bus clients, however many there are — funnelled through
//! the one thread that is allowed to talk to the dongle.

use crate::device::{self, Link, Shared};
use crate::eq::{self, profiles::Correction};
use crate::surround;
use crate::update;
use jbl_quantum::{Anc, Colour, Effect, Sidetone, Zone, ZoneLighting, MAX_SLOTS};
use quantumengine_dbus::ZoneLightingArg;
use std::sync::mpsc::{self, Sender};
use zbus::fdo;
use zbus::interface;

pub struct HeadsetIface {
    shared: Shared,
    commands: Sender<device::Command>,
    eq: eq::Handle,
    surround: surround::Handle,
    updater: update::Handle,
}

#[interface(name = "org.quantumengine.Headset1")]
impl HeadsetIface {
    #[zbus(property)]
    fn link(&self) -> String {
        link_str(&self.shared.lock().unwrap().link)
    }

    #[zbus(property)]
    fn link_detail(&self) -> String {
        match &self.shared.lock().unwrap().link {
            Link::NoAccess(msg) => msg.clone(),
            _ => String::new(),
        }
    }

    #[zbus(property)]
    fn battery(&self) -> u8 {
        self.shared.lock().unwrap().battery.unwrap_or(0)
    }

    #[zbus(property)]
    fn anc(&self) -> String {
        self.shared.lock().unwrap().anc.map_or("off", Anc::as_str).to_string()
    }

    #[zbus(property)]
    fn sidetone(&self) -> String {
        self.shared.lock().unwrap().sidetone.map_or("off", Sidetone::as_str).to_string()
    }

    #[zbus(property)]
    fn mic_active(&self) -> bool {
        self.shared.lock().unwrap().mic_active.unwrap_or(false)
    }

    #[zbus(property)]
    fn dial(&self) -> u8 {
        self.shared.lock().unwrap().dial.unwrap_or(8)
    }

    #[zbus(property)]
    fn lights_on(&self) -> bool {
        self.shared.lock().unwrap().lights_on.unwrap_or(false)
    }

    #[zbus(property)]
    fn device_name(&self) -> String {
        self.shared.lock().unwrap().device_name.clone()
    }

    #[zbus(property)]
    fn serial(&self) -> String {
        self.shared.lock().unwrap().serial.clone()
    }

    #[zbus(property)]
    fn firmware(&self) -> Vec<String> {
        self.shared.lock().unwrap().firmware.clone()
    }

    #[zbus(property)]
    fn busy(&self) -> bool {
        self.shared.lock().unwrap().pending.is_some()
    }

    #[zbus(property)]
    fn last_error(&self) -> String {
        self.shared.lock().unwrap().error.clone().unwrap_or_default()
    }

    fn set_anc(&self, mode: &str) -> fdo::Result<String> {
        let mode = parse_anc(mode)?;
        let (reply, rx) = mpsc::channel();
        self.dispatch(device::Command::Anc(mode, reply))?;
        recv(rx).map(|m| m.as_str().to_string())
    }

    fn set_sidetone(&self, level: &str) -> fdo::Result<String> {
        let level = parse_sidetone(level)?;
        let (reply, rx) = mpsc::channel();
        self.dispatch(device::Command::Sidetone(level, reply))?;
        recv(rx).map(|l| l.as_str().to_string())
    }

    fn set_lights(&self, on: bool) -> fdo::Result<bool> {
        let (reply, rx) = mpsc::channel();
        self.dispatch(device::Command::Lights(on, reply))?;
        recv(rx)
    }

    fn set_lighting(&self, zones: Vec<ZoneLightingArg>) -> fdo::Result<()> {
        let zones = zones.into_iter().map(parse_zone_lighting).collect::<fdo::Result<Vec<_>>>()?;
        let (reply, rx) = mpsc::channel();
        self.dispatch(device::Command::Lighting(zones, reply))?;
        recv(rx)
    }

    #[zbus(property)]
    fn eq_enabled(&self) -> bool {
        self.eq.state().enabled
    }

    #[zbus(property)]
    fn eq_profile(&self) -> String {
        self.eq.state().profile.to_string()
    }

    #[zbus(property)]
    fn eq_profiles(&self) -> Vec<String> {
        eq::profiles::PROFILES.iter().map(|p| p.name.to_string()).collect()
    }

    #[zbus(property)]
    fn eq_correction(&self) -> String {
        self.eq.state().correction.as_str().to_string()
    }

    #[zbus(property)]
    fn eq_status(&self) -> String {
        self.eq.state().status
    }

    fn set_eq_enabled(&self, enabled: bool) -> fdo::Result<bool> {
        self.eq.call(|r| eq::Command::Enable(enabled, r)).map_err(fdo::Error::Failed)
    }

    fn set_eq_profile(&self, name: &str) -> fdo::Result<String> {
        if eq::profiles::find(name).is_none() {
            return Err(fdo::Error::InvalidArgs(format!("unknown EQ profile '{name}'")));
        }
        let name = name.to_string();
        self.eq.call(|r| eq::Command::Profile(name, r)).map(str::to_string).map_err(fdo::Error::Failed)
    }

    fn eq_response(&self, profile: &str, correction: &str) -> fdo::Result<Vec<f64>> {
        let p = eq::profiles::find(profile)
            .ok_or_else(|| fdo::Error::InvalidArgs(format!("unknown EQ profile '{profile}'")))?;
        let c = parse_correction(correction)?;
        let freqs = (0..quantumengine_dbus::EQ_RESPONSE_POINTS).map(quantumengine_dbus::eq_response_freq);
        Ok(eq::response(c, p, freqs))
    }

    #[zbus(property)]
    fn surround_enabled(&self) -> bool {
        self.surround.state().enabled
    }

    #[zbus(property)]
    fn surround_status(&self) -> String {
        self.surround.state().status
    }

    fn set_surround_enabled(&self, enabled: bool) -> fdo::Result<bool> {
        self.surround.call(|r| surround::Command::Enable(enabled, r)).map_err(fdo::Error::Failed)
    }

    fn set_eq_correction(&self, correction: &str) -> fdo::Result<String> {
        let c = parse_correction(correction)?;
        self.eq
            .call(|r| eq::Command::Correction(c, r))
            .map(|c| c.as_str().to_string())
            .map_err(fdo::Error::Failed)
    }

    #[zbus(property)]
    fn version(&self) -> String {
        update::VERSION.to_string()
    }

    #[zbus(property)]
    fn update_version(&self) -> String {
        self.updater.state().available.map(|r| r.version).unwrap_or_default()
    }

    #[zbus(property)]
    fn update_url(&self) -> String {
        self.updater.state().available.map(|r| r.url).unwrap_or_default()
    }

    #[zbus(property)]
    fn update_can_install(&self) -> bool {
        self.updater.state().can_install()
    }

    #[zbus(property)]
    fn update_status(&self) -> String {
        self.updater.state().status
    }

    #[zbus(property)]
    fn update_busy(&self) -> bool {
        self.updater.state().busy
    }

    #[zbus(property)]
    fn update_checks(&self) -> bool {
        self.updater.state().checks_enabled
    }

    fn check_for_updates(&self) -> fdo::Result<String> {
        self.updater
            .call(update::Command::Check)
            .map(Option::unwrap_or_default)
            .map_err(fdo::Error::Failed)
    }

    fn install_update(&self) -> fdo::Result<()> {
        self.updater.call(update::Command::Install).map_err(fdo::Error::Failed)
    }

    fn set_update_checks(&self, enabled: bool) -> fdo::Result<bool> {
        self.updater.call(|r| update::Command::SetChecks(enabled, r)).map_err(fdo::Error::Failed)
    }
}

impl HeadsetIface {
    fn dispatch(&self, cmd: device::Command) -> fdo::Result<()> {
        self.commands
            .send(cmd)
            .map_err(|_| fdo::Error::Failed("the headset worker has stopped".into()))
    }
}

/// Wait for a command's outcome and turn it into the D-Bus error `Set*`
/// methods return on failure.
fn recv<T>(rx: mpsc::Receiver<Result<T, jbl_quantum::Error>>) -> fdo::Result<T> {
    match rx.recv() {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(fdo::Error::Failed(e.to_string())),
        Err(_) => Err(fdo::Error::Failed("no reply from the headset worker".into())),
    }
}

fn parse_anc(s: &str) -> fdo::Result<Anc> {
    match s {
        "off" => Ok(Anc::Off),
        "on" => Ok(Anc::On),
        "talkthru" => Ok(Anc::TalkThru),
        other => Err(fdo::Error::InvalidArgs(format!(
            "unknown ANC mode '{other}' (want off, on or talkthru)"
        ))),
    }
}

fn parse_sidetone(s: &str) -> fdo::Result<Sidetone> {
    match s {
        "off" => Ok(Sidetone::Off),
        "low" => Ok(Sidetone::Low),
        "mid" => Ok(Sidetone::Mid),
        "high" => Ok(Sidetone::High),
        other => Err(fdo::Error::InvalidArgs(format!(
            "unknown sidetone level '{other}' (want off, low, mid or high)"
        ))),
    }
}

fn parse_zone_lighting(arg: ZoneLightingArg) -> fdo::Result<ZoneLighting> {
    let zone = match arg.zone.as_str() {
        "logo" => Zone::Logo,
        "ring" => Zone::Ring,
        other => {
            return Err(fdo::Error::InvalidArgs(format!("unknown zone '{other}' (want logo or ring)")))
        }
    };
    if arg.colours.is_empty() || arg.colours.len() > MAX_SLOTS {
        return Err(fdo::Error::InvalidArgs(format!(
            "palette must have 1 to {MAX_SLOTS} colours, got {}",
            arg.colours.len()
        )));
    }
    if arg.brightness > 100 {
        return Err(fdo::Error::InvalidArgs(format!("brightness must be 0-100, got {}", arg.brightness)));
    }
    Ok(ZoneLighting {
        zone,
        brightness: arg.brightness,
        palette: arg.colours.iter().map(|&(r, g, b)| Colour::new(r, g, b)).collect(),
        effect: Effect(arg.effect),
    })
}

fn link_str(link: &Link) -> String {
    match link {
        Link::NoDongle => "no_dongle",
        Link::NoAccess(_) => "no_access",
        Link::HeadsetOff => "headset_off",
        Link::Ready => "ready",
    }
    .to_string()
}

/// Claims [`quantumengine_dbus::BUS_NAME`] on the session bus.
///
/// Returns `None` when another service already owns the name. The claim is
/// made without replacement on purpose: zbus's default flags would let a
/// second instance take the name over and leave the first running as a
/// duplicate tray icon that still holds the dongle open. The caller must then
/// exit before it starts anything else — a tray icon, the EQ filter, or the
/// headset worker.
///
/// The connection must be kept alive for as long as the service runs;
/// dropping it releases the name.
pub fn claim() -> zbus::Result<Option<zbus::blocking::Connection>> {
    let connection = zbus::blocking::Connection::session()?;
    match connection.request_name_with_flags(
        quantumengine_dbus::BUS_NAME,
        fdo::RequestNameFlags::DoNotQueue.into(),
    ) {
        Ok(fdo::RequestNameReply::PrimaryOwner | fdo::RequestNameReply::AlreadyOwner) => Ok(Some(connection)),
        // zbus reports a taken name as an error rather than an `Exists` reply.
        Ok(_) | Err(zbus::Error::NameTaken) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Serves the interface at [`quantumengine_dbus::OBJECT_PATH`] on a
/// connection from [`claim`], reading from `shared`, `eq` and `updater` and
/// sending to their command channels — all set up by the caller, which owns
/// the threads that drain them (see `main::run_tray`).
///
/// Returns a callback the caller must invoke whenever anything the interface
/// exposes changes. It emits a `PropertiesChanged` for every property — a
/// broad brush rather than diffing field by field, but changes only happen on
/// user action, a physical control or the updater, so the traffic is small.
pub fn serve(
    connection: &zbus::blocking::Connection,
    shared: Shared,
    commands: Sender<device::Command>,
    eq: eq::Handle,
    surround: surround::Handle,
    updater: update::Handle,
) -> zbus::Result<impl Fn() + Send + Sync + use<>> {
    connection
        .object_server()
        .at(quantumengine_dbus::OBJECT_PATH, HeadsetIface { shared, commands, eq, surround, updater })?;

    let iface_ref = connection
        .object_server()
        .interface::<_, HeadsetIface>(quantumengine_dbus::OBJECT_PATH)?;

    let notify_changed = move || {
        let iface = iface_ref.get();
        let emitter = iface_ref.signal_emitter();
        // Best-effort: a failure here means a client missed one update, not
        // that the service is broken, so log-and-continue rather than panic.
        for result in [
            zbus::block_on(iface.link_changed(emitter)),
            zbus::block_on(iface.link_detail_changed(emitter)),
            zbus::block_on(iface.battery_changed(emitter)),
            zbus::block_on(iface.anc_changed(emitter)),
            zbus::block_on(iface.sidetone_changed(emitter)),
            zbus::block_on(iface.mic_active_changed(emitter)),
            zbus::block_on(iface.dial_changed(emitter)),
            zbus::block_on(iface.lights_on_changed(emitter)),
            zbus::block_on(iface.device_name_changed(emitter)),
            zbus::block_on(iface.serial_changed(emitter)),
            zbus::block_on(iface.firmware_changed(emitter)),
            zbus::block_on(iface.busy_changed(emitter)),
            zbus::block_on(iface.last_error_changed(emitter)),
            zbus::block_on(iface.eq_enabled_changed(emitter)),
            zbus::block_on(iface.eq_profile_changed(emitter)),
            zbus::block_on(iface.eq_correction_changed(emitter)),
            zbus::block_on(iface.eq_status_changed(emitter)),
            zbus::block_on(iface.surround_enabled_changed(emitter)),
            zbus::block_on(iface.surround_status_changed(emitter)),
            zbus::block_on(iface.update_version_changed(emitter)),
            zbus::block_on(iface.update_url_changed(emitter)),
            zbus::block_on(iface.update_can_install_changed(emitter)),
            zbus::block_on(iface.update_status_changed(emitter)),
            zbus::block_on(iface.update_busy_changed(emitter)),
            zbus::block_on(iface.update_checks_changed(emitter)),
        ] {
            if let Err(e) = result {
                eprintln!("quantumengined: failed to emit a property change: {e}");
            }
        }
    };

    Ok(notify_changed)
}

fn parse_correction(s: &str) -> fdo::Result<Correction> {
    Correction::parse(s)
        .ok_or_else(|| fdo::Error::InvalidArgs(format!("unknown correction '{s}' (want off, stereo or surround)")))
}
