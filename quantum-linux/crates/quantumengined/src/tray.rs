//! The system tray icon. Reads [`device::Snapshot`] and sends
//! [`device::Command`]s, exactly like the window — it never touches the
//! headset itself.
//!
//! ksni only re-renders the icon, tooltip and menu when told to, via
//! [`ksni::blocking::Handle::update`]. Nothing here needs the tray's own
//! state changed for that (everything it renders lives in [`device::Shared`]
//! already), so [`refresh`] passes an empty closure — its only job is to
//! prompt ksni to call the render methods below again and diff the result.

use crate::device::{self, Link, Shared, Snapshot};
use crate::eq::{self, profiles::Correction};
use crate::surround;
use crate::update;
use jbl_quantum::Anc;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, RadioGroup, RadioItem, StandardItem, SubMenu};
use ksni::MenuItem;
use std::cell::Cell;
use std::sync::mpsc::Sender;
use std::sync::LazyLock;

/// Battery percentage at or below which the tray asks for attention and
/// notifies once.
const LOW_BATTERY: u8 = 15;

const ICON_PNG: &[u8] = include_bytes!("../icon.png");

static ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
    let img = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png)
        .expect("bundled icon.png is a valid PNG")
        .into_rgba8();
    let (width, height) = (img.width() as i32, img.height() as i32);
    let mut data = img.into_vec();
    for pixel in data.as_chunks_mut::<4>().0 {
        pixel.rotate_right(1); // RGBA -> ARGB, as ksni::Icon wants
    }
    ksni::Icon { width, height, data }
});

pub struct QuantumTray {
    shared: Shared,
    commands: Sender<device::Command>,
    eq: eq::Handle,
    surround: surround::Handle,
    updater: update::Handle,
    /// Whether the last-seen battery level already triggered a
    /// notification, so a level that stays low doesn't repeat it.
    battery_notified: Cell<bool>,
}

impl QuantumTray {
    fn snap(&self) -> Snapshot {
        self.shared.lock().unwrap().clone()
    }

    fn send(&self, cmd: device::Command) {
        drop(self.commands.send(cmd));
    }
}

impl ksni::Tray for QuantumTray {
    fn id(&self) -> String {
        "quantumengine".into()
    }

    fn title(&self) -> String {
        "QuantumEngine".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ICON.clone()]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_window();
    }

    /// `NeedsAttention` makes most trays visually emphasise the icon
    /// (Plasma, for instance, keeps it out of the "hidden" overflow area).
    /// Also where the one-shot low-battery notification is decided: this is
    /// called exactly once per real change, since [`refresh`] is the only
    /// thing that ever asks ksni to re-render.
    fn status(&self) -> ksni::Status {
        let snap = self.snap();
        let low = is_low_battery(&snap);
        // Records the current state and returns what was previously
        // recorded, in one step: notify only on the transition into low
        // battery, and let a later drop notify again once it's cleared.
        let already_notified = self.battery_notified.replace(low);
        if low && !already_notified {
            notify_low_battery(snap.battery.unwrap_or(0));
        }
        if low { ksni::Status::NeedsAttention } else { ksni::Status::Active }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let snap = self.snap();
        ksni::ToolTip { title: tooltip_text(&snap), ..Default::default() }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let snap = self.snap();
        let mut items = update_items(&self.updater.state());
        items.extend([
            SubMenu {
                label: "Noise control".into(),
                submenu: vec![anc_radio_group(&snap)],
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Sidetone".into(),
                submenu: vec![sidetone_radio_group(&snap)],
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "Lights".into(),
                checked: snap.lights_on.unwrap_or(false),
                activate: Box::new(|this: &mut Self| {
                    let now = this.shared.lock().unwrap().lights_on.unwrap_or(false);
                    this.send(device::Command::Lights(!now, device::no_reply()));
                }),
                ..Default::default()
            }
            .into(),
            eq_menu(&self.eq.state()),
            CheckmarkItem {
                label: "Virtual surround 7.1".into(),
                checked: self.surround.state().enabled,
                activate: Box::new(|this: &mut Self| {
                    let on = !this.surround.state().enabled;
                    this.surround.send(surround::Command::Enable(on, surround::no_reply()));
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Open window".into(),
                activate: Box::new(|_: &mut Self| open_window()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_: &mut Self| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]);
        items
    }
}

fn eq_menu(state: &eq::State) -> MenuItem<QuantumTray> {
    let names = state.profile_names();
    let label = if state.enabled { format!("Equalizer: {}", state.profile) } else { "Equalizer: off".into() };
    let mut submenu = vec![
        CheckmarkItem {
            label: "Enabled".into(),
            checked: state.enabled,
            activate: Box::new(|this: &mut QuantumTray| {
                let on = !this.eq.state().enabled;
                this.eq.send(eq::Command::Enable(on, eq::no_reply()));
            }),
            ..Default::default()
        }
        .into(),
    ];
    if !state.status.is_empty() {
        submenu.push(StandardItem { label: state.status.clone(), enabled: false, ..Default::default() }.into());
    }
    submenu.push(MenuItem::Separator);
    submenu.push(
        RadioGroup {
            selected: names.iter().position(|p| *p == state.profile).unwrap_or(0),
            select: Box::new(|this: &mut QuantumTray, index| {
                if let Some(name) = this.eq.state().profile_names().get(index) {
                    this.eq.send(eq::Command::Profile(name.clone(), eq::no_reply()));
                }
            }),
            options: names.iter().map(|p| RadioItem { label: p.clone(), ..Default::default() }).collect(),
        }
        .into(),
    );
    submenu.push(MenuItem::Separator);
    submenu.push(
        SubMenu {
            label: "Headset tuning".into(),
            submenu: vec![RadioGroup {
                selected: Correction::ALL.iter().position(|&c| c == state.correction).unwrap_or(0),
                select: Box::new(|this: &mut QuantumTray, index| {
                    this.eq.send(eq::Command::Correction(Correction::ALL[index], eq::no_reply()));
                }),
                options: Correction::ALL
                    .iter()
                    .map(|c| RadioItem { label: correction_label(*c).into(), ..Default::default() })
                    .collect(),
            }
            .into()],
            ..Default::default()
        }
        .into(),
    );
    SubMenu { label, submenu, ..Default::default() }.into()
}

pub fn correction_label(c: Correction) -> &'static str {
    match c {
        Correction::Off => "Off",
        Correction::Stereo => "JBL stereo",
        Correction::Surround => "JBL surround",
    }
}

/// An "Install update" entry (or a link to the release, or what the updater
/// is busy with) at the top of the menu, only while there is something to say.
fn update_items(state: &update::State) -> Vec<MenuItem<QuantumTray>> {
    let item: MenuItem<QuantumTray> = match &state.available {
        _ if state.busy && !state.status.is_empty() => {
            StandardItem { label: state.status.clone(), enabled: false, ..Default::default() }.into()
        }
        Some(r) if state.can_install() => StandardItem {
            label: format!("Install update {}", r.version),
            icon_name: "system-software-update".into(),
            activate: Box::new(|this: &mut QuantumTray| this.updater.send(update::Command::Install(update::no_reply()))),
            ..Default::default()
        }
        .into(),
        Some(r) => {
            let url = r.url.clone();
            StandardItem {
                label: format!("Update {} available…", r.version),
                icon_name: "system-software-update".into(),
                activate: Box::new(move |_: &mut QuantumTray| update::open_url(&url)),
                ..Default::default()
            }
            .into()
        }
        None => return Vec::new(),
    };
    vec![item, MenuItem::Separator]
}

fn anc_radio_group(snap: &Snapshot) -> MenuItem<QuantumTray> {
    const MODES: [Anc; 3] = [Anc::Off, Anc::On, Anc::TalkThru];
    RadioGroup {
        selected: MODES.iter().position(|&m| Some(m) == snap.anc).unwrap_or(0),
        select: Box::new(|this: &mut QuantumTray, index| {
            this.send(device::Command::Anc(MODES[index], device::no_reply()));
        }),
        options: vec![
            RadioItem { label: "Off".into(), ..Default::default() },
            RadioItem { label: "ANC".into(), ..Default::default() },
            RadioItem { label: "TalkThru".into(), ..Default::default() },
        ],
    }
    .into()
}

fn sidetone_radio_group(snap: &Snapshot) -> MenuItem<QuantumTray> {
    use jbl_quantum::Sidetone as St;
    const LEVELS: [St; 4] = [St::Off, St::Low, St::Mid, St::High];
    RadioGroup {
        selected: LEVELS.iter().position(|&l| Some(l) == snap.sidetone).unwrap_or(0),
        select: Box::new(|this: &mut QuantumTray, index| {
            this.send(device::Command::Sidetone(LEVELS[index], device::no_reply()));
        }),
        options: vec![
            RadioItem { label: "Off".into(), ..Default::default() },
            RadioItem { label: "Low".into(), ..Default::default() },
            RadioItem { label: "Mid".into(), ..Default::default() },
            RadioItem { label: "High".into(), ..Default::default() },
        ],
    }
    .into()
}

fn is_low_battery(snap: &Snapshot) -> bool {
    snap.link == Link::Ready && matches!(snap.battery, Some(pct) if pct <= LOW_BATTERY)
}

fn tooltip_text(snap: &Snapshot) -> String {
    match (&snap.link, snap.battery, snap.anc) {
        (Link::Ready, Some(pct), Some(anc)) => format!("Battery {pct}% · ANC {}", anc.as_str()),
        (Link::Ready, ..) => "QuantumEngine".into(),
        (Link::HeadsetOff, ..) => "Headset off or out of range".into(),
        (Link::NoAccess(_), ..) => "Cannot access the dongle".into(),
        (Link::Unverified, ..) => "Unverified headset model".into(),
        (Link::NoDongle, ..) => "No dongle plugged in".into(),
    }
}

/// Starts the `quantumengine` window: the one next to this executable when
/// there is one (a development build in `target/`), else whichever is on
/// `PATH`. The window only talks to this service over D-Bus, and is
/// single-instance, so a second click just raises it.
fn open_window() {
    let sibling = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name("quantumengine"))
        .filter(|p| p.is_file());
    let program = sibling.unwrap_or_else(|| "quantumengine".into());
    let result = std::process::Command::new(program)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    if let Err(e) = result {
        eprintln!("quantumengined: failed to open a window: {e}");
    }
}

fn notify_low_battery(pct: u8) {
    let result = notify_rust::Notification::new()
        .summary("QuantumEngine")
        .body(&format!("Headset battery at {pct}%"))
        .icon("quantumengine")
        .urgency(notify_rust::Urgency::Critical)
        .show();
    if let Err(e) = result {
        eprintln!("quantumengined: failed to show the low-battery notification: {e}");
    }
}

/// Spawns the tray icon and returns a handle. Call `handle.update(|_| {})`
/// whenever `shared` changes, to prompt ksni to re-render.
pub fn spawn(
    shared: Shared,
    commands: Sender<device::Command>,
    eq: eq::Handle,
    surround: surround::Handle,
    updater: update::Handle,
) -> Result<Handle<QuantumTray>, ksni::Error> {
    // Wait for a tray host instead of failing when there isn't one yet (the
    // service can start before Plasma's at login) or at all.
    QuantumTray { shared, commands, eq, surround, updater, battery_notified: Cell::new(false) }
        .assume_sni_available(true)
        .spawn()
}

/// A closure that makes ksni re-render the icon, tooltip and menu.
pub fn refresh(handle: Handle<QuantumTray>) -> impl Fn() + Send + Sync {
    move || {
        handle.update(|_| {});
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jbl_quantum::Anc;

    fn ready(battery: Option<u8>, anc: Option<Anc>) -> Snapshot {
        Snapshot { link: Link::Ready, battery, anc, ..Snapshot::default() }
    }

    #[test]
    fn low_battery_only_while_ready() {
        assert!(is_low_battery(&ready(Some(15), None)));
        assert!(is_low_battery(&ready(Some(0), None)));
        assert!(!is_low_battery(&ready(Some(16), None)));
        assert!(!is_low_battery(&ready(None, None)));
        assert!(!is_low_battery(&Snapshot { link: Link::HeadsetOff, battery: Some(5), ..Snapshot::default() }));
    }

    #[test]
    fn tooltip_reports_battery_and_anc_once_ready() {
        assert_eq!(tooltip_text(&ready(Some(80), Some(Anc::On))), "Battery 80% · ANC on");
        assert_eq!(tooltip_text(&Snapshot::default()), "No dongle plugged in");
        assert_eq!(tooltip_text(&Snapshot { link: Link::HeadsetOff, ..Snapshot::default() }), "Headset off or out of range");
    }

    #[test]
    fn anc_radio_group_selects_the_current_mode() {
        let MenuItem::RadioGroup(group) = anc_radio_group(&ready(None, Some(Anc::TalkThru))) else {
            panic!("expected a radio group");
        };
        assert_eq!(group.selected, 2);
    }

    #[test]
    fn sidetone_radio_group_defaults_to_off_when_unknown() {
        let MenuItem::RadioGroup(group) = sidetone_radio_group(&ready(None, None)) else {
            panic!("expected a radio group");
        };
        assert_eq!(group.selected, 0);
    }
}
