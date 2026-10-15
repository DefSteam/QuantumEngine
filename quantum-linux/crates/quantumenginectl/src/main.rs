//! Command-line control for JBL Quantum 810 Wireless headsets.
//!
//! Every command except `devices` (a plain hidraw enumeration, with no
//! device access) goes through the `quantumengined` service over
//! D-Bus, starting it if it is not already running — this CLI never opens
//! the headset itself. See `quantumengine-dbus` for the shared contract.

mod dbus;

use jbl_quantum::{Colour, Effect, Headset, Zone, MAX_SLOTS};
use quantumengine_dbus::{HeadsetProxyBlocking, ZoneLightingArg};
use std::process::ExitCode;
use std::time::Duration;

const USAGE: &str = "\
quantumenginectl — control a JBL Quantum 810 Wireless headset

USAGE:
    quantumenginectl [--json] <COMMAND>

COMMANDS:
    status                      everything at once (default)
    battery                     battery percentage
    anc [off|on|talkthru]       show or set noise cancelling
    sidetone [off|low|mid|high] show or set microphone monitoring
    mic                         whether the microphone is live
    dial                        game/chat dial position, 0-16
    lights [on|off]             show or set the lights
    rgb <zone> <effect> <colour>...
                                configure lighting and switch it on
    watch                       stream state changes until interrupted
    devices                     list detected dongles
    eq [on|off|<profile>]       show or set the equalizer
    eq list                     list the EQ profiles
    eq tuning [off|stereo|surround]
                                JBL's headset tuning under the profile
    surround [on|off]           show or set virtual 7.1 surround
    update                      check GitHub for a newer release
    update install              install it (asks for your password)
    update checks [on|off]      show or set the daily update check

    rgb zones:    logo, ring, both
    rgb effects:  fade, instant, chase, strobe, chase-strobe-logo,
                  chase-flash-logo, fast, chase-blink-logo, rainbow-logo,
                  spectrum
    rgb colours:  1-5 hex colours, e.g. '#ff0000'
    rgb options:  --brightness N   (0-100, default 100)

OPTIONS:
    --json      machine-readable output
    -h, --help  this help
";

/// How often `watch` re-reads state looking for a change.
const WATCH_POLL_EVERY: Duration = Duration::from_millis(150);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut json = false;
    let mut rest: Vec<String> = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ => rest.push(arg),
        }
    }

    match run(&rest, json) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("quantumenginectl: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String], json: bool) -> Result<(), String> {
    let (command, params) = args
        .split_first()
        .map_or(("status", &[] as &[String]), |(c, p)| (c.as_str(), p));

    if command == "devices" {
        let nodes = Headset::discover().map_err(|e| e.to_string())?;
        if json {
            let entries: Vec<String> = nodes
                .iter()
                .map(|n| {
                    format!(
                        "{{\"path\":\"{}\",\"name\":\"{}\"}}",
                        n.path.display(),
                        escape(&n.name)
                    )
                })
                .collect();
            println!("[{}]", entries.join(","));
        } else if nodes.is_empty() {
            println!("no JBL Quantum 810 dongle found");
        } else {
            for n in &nodes {
                println!("{}  {}", n.path.display(), n.name);
            }
        }
        return Ok(());
    }

    let proxy = dbus::connect()?;

    match command {
        "status" => cmd_status(&proxy, json)?,
        "battery" => {
            require_ready(&proxy)?;
            let pct = proxy.battery().map_err(dbus_err)?;
            if json {
                println!("{{\"battery_percent\":{pct}}}");
            } else {
                println!("{pct}%");
            }
        }
        "anc" => match params.first() {
            None => {
                let mode = proxy.anc().map_err(dbus_err)?;
                emit(json, "anc", &mode);
            }
            Some(value) => {
                parse_anc(value)?;
                let mode = proxy.set_anc(value).map_err(dbus_err)?;
                emit(json, "anc", &mode);
            }
        },
        "sidetone" => match params.first() {
            None => {
                let level = proxy.sidetone().map_err(dbus_err)?;
                emit(json, "sidetone", &level);
            }
            Some(value) => {
                parse_sidetone(value)?;
                let level = proxy.set_sidetone(value).map_err(dbus_err)?;
                emit(json, "sidetone", &level);
            }
        },
        "mic" => {
            let active = proxy.mic_active().map_err(dbus_err)?;
            if json {
                println!("{{\"mic_active\":{active}}}");
            } else {
                println!("{}", if active { "active" } else { "inactive" });
            }
        }
        "dial" => {
            let pos = proxy.dial().map_err(dbus_err)?;
            if json {
                println!("{{\"dial\":{pos},\"label\":\"{}\"}}", dial_label(pos));
            } else {
                println!("{pos} ({})", dial_label(pos));
            }
        }
        "lights" => match params.first() {
            None => {
                let on = proxy.lights_on().map_err(dbus_err)?;
                emit(json, "lights", if on { "on" } else { "off" });
            }
            Some(value) => {
                let on = parse_on_off(value)?;
                let on = proxy.set_lights(on).map_err(dbus_err)?;
                emit(json, "lights", if on { "on" } else { "off" });
            }
        },
        "rgb" => {
            require_ready(&proxy)?;
            cmd_rgb(&proxy, params)?;
            emit(json, "lighting", "applied");
        }
        "watch" => cmd_watch(&proxy, json)?,
        "eq" => cmd_eq(&proxy, params, json)?,
        "surround" => cmd_surround(&proxy, params, json)?,
        "update" => cmd_update(&proxy, params, json)?,
        other => return Err(format!("unknown command '{other}' (try --help)")),
    }
    Ok(())
}

/// Fails with the same wording `Headset::open_first` used to give when
/// there was no live link, instead of silently showing default values.
fn require_ready(proxy: &HeadsetProxyBlocking) -> Result<(), String> {
    match proxy.link().map_err(dbus_err)?.as_str() {
        "ready" => Ok(()),
        "no_dongle" => Err(jbl_quantum::Error::NotFound.to_string()),
        "no_access" => Err(proxy.link_detail().unwrap_or_default()),
        _ => Err(jbl_quantum::Error::NotConnected.to_string()),
    }
}

fn cmd_status(proxy: &HeadsetProxyBlocking, json: bool) -> Result<(), String> {
    let link = proxy.link().map_err(dbus_err)?;
    let battery = proxy.battery().map_err(dbus_err)?;
    let anc = proxy.anc().map_err(dbus_err)?;
    let sidetone = proxy.sidetone().map_err(dbus_err)?;
    let mic_active = proxy.mic_active().map_err(dbus_err)?;
    let dial = proxy.dial().map_err(dbus_err)?;
    let lights_on = proxy.lights_on().map_err(dbus_err)?;
    let device_name = proxy.device_name().map_err(dbus_err)?;
    let serial = proxy.serial().map_err(dbus_err)?;
    let firmware = proxy.firmware().map_err(dbus_err)?;
    let eq = Eq::read(proxy)?;
    let surround = Surround::read(proxy)?;
    let update = UpdateInfo::read(proxy)?;

    if json {
        let fw: Vec<String> = firmware.iter().map(|f| format!("\"{f}\"")).collect();
        println!(
            "{{\"link\":\"{link}\",\"battery_percent\":{battery},\"anc\":\"{anc}\",\"sidetone\":\"{sidetone}\",\
             \"mic_active\":{mic_active},\"dial\":{dial},\"lights_on\":{lights_on},\"device_name\":\"{}\",\"serial\":\"{}\",\"firmware\":[{}],\
             \"eq\":{},\"surround\":{},\"update\":{}}}",
            escape(&device_name),
            escape(&serial),
            fw.join(","),
            eq.json(),
            surround.json(),
            update.json()
        );
    } else {
        if link != "ready" {
            println!("headset:   {}", link_note(&link));
            println!("           values below are defaults until it reconnects");
        }
        println!("battery:   {battery}%");
        println!("anc:       {anc}");
        println!("sidetone:  {sidetone}");
        println!("mic:       {}", if mic_active { "active" } else { "inactive" });
        println!("dial:      {dial} ({})", dial_label(dial));
        println!("lights:    {}", if lights_on { "on" } else { "off" });
        println!("paired to: {device_name}");
        println!("serial:    {serial}");
        println!("firmware:  {}", firmware.join(", "));
        println!("eq:        {}", eq.summary());
        println!("surround:  {}", surround.summary());
        if !update.available.is_empty() {
            println!("update:    {} available (you have {})", update.available, update.version);
        }
    }
    Ok(())
}

struct Eq {
    enabled: bool,
    profile: String,
    profiles: Vec<String>,
    tuning: String,
    status: String,
}

impl Eq {
    fn read(proxy: &HeadsetProxyBlocking) -> Result<Self, String> {
        Ok(Self {
            enabled: proxy.eq_enabled().map_err(dbus_err)?,
            profile: proxy.eq_profile().map_err(dbus_err)?,
            profiles: proxy.eq_profiles().map_err(dbus_err)?,
            tuning: proxy.eq_correction().map_err(dbus_err)?,
            status: proxy.eq_status().map_err(dbus_err)?,
        })
    }

    fn summary(&self) -> String {
        let state = if self.enabled { "on" } else { "off" };
        let mut text = format!("{state}, {} (headset tuning: {})", self.profile, self.tuning);
        if !self.status.is_empty() {
            text.push_str(&format!(" — {}", self.status));
        }
        text
    }

    fn json(&self) -> String {
        let profiles: Vec<String> = self.profiles.iter().map(|p| format!("\"{}\"", escape(p))).collect();
        format!(
            "{{\"enabled\":{},\"profile\":\"{}\",\"tuning\":\"{}\",\"status\":\"{}\",\"profiles\":[{}]}}",
            self.enabled,
            escape(&self.profile),
            escape(&self.tuning),
            escape(&self.status),
            profiles.join(",")
        )
    }
}

struct Surround {
    enabled: bool,
    status: String,
}

impl Surround {
    fn read(proxy: &HeadsetProxyBlocking) -> Result<Self, String> {
        Ok(Self {
            enabled: proxy.surround_enabled().map_err(dbus_err)?,
            status: proxy.surround_status().map_err(dbus_err)?,
        })
    }

    fn summary(&self) -> String {
        let mut text = if self.enabled { "on (virtual 7.1)" } else { "off" }.to_string();
        if !self.status.is_empty() {
            text.push_str(&format!(" — {}", self.status));
        }
        text
    }

    fn json(&self) -> String {
        format!("{{\"enabled\":{},\"status\":\"{}\"}}", self.enabled, escape(&self.status))
    }
}

fn cmd_surround(proxy: &HeadsetProxyBlocking, params: &[String], json: bool) -> Result<(), String> {
    match params.first().map(String::as_str) {
        None => {}
        Some("on") => drop(proxy.set_surround_enabled(true).map_err(dbus_err)?),
        Some("off") => drop(proxy.set_surround_enabled(false).map_err(dbus_err)?),
        Some(other) => return Err(format!("surround takes on or off, not '{other}'")),
    }
    let surround = Surround::read(proxy)?;
    if json {
        println!("{}", surround.json());
    } else {
        println!("{}", surround.summary());
    }
    Ok(())
}

struct UpdateInfo {
    version: String,
    available: String,
    url: String,
    can_install: bool,
    busy: bool,
    status: String,
}

impl UpdateInfo {
    fn read(proxy: &HeadsetProxyBlocking) -> Result<Self, String> {
        Ok(Self {
            version: proxy.version().map_err(dbus_err)?,
            available: proxy.update_version().map_err(dbus_err)?,
            url: proxy.update_url().map_err(dbus_err)?,
            can_install: proxy.update_can_install().map_err(dbus_err)?,
            busy: proxy.update_busy().map_err(dbus_err)?,
            status: proxy.update_status().map_err(dbus_err)?,
        })
    }

    fn json(&self) -> String {
        format!(
            "{{\"version\":\"{}\",\"available\":\"{}\",\"url\":\"{}\",\"can_install\":{},\"busy\":{},\"status\":\"{}\"}}",
            escape(&self.version),
            escape(&self.available),
            escape(&self.url),
            self.can_install,
            self.busy,
            escape(&self.status)
        )
    }
}

fn cmd_eq(proxy: &HeadsetProxyBlocking, params: &[String], json: bool) -> Result<(), String> {
    match params.first().map(String::as_str) {
        None => {}
        Some("on") => drop(proxy.set_eq_enabled(true).map_err(dbus_err)?),
        Some("off") => drop(proxy.set_eq_enabled(false).map_err(dbus_err)?),
        Some("list") => {
            let eq = Eq::read(proxy)?;
            if json {
                let names: Vec<String> = eq.profiles.iter().map(|p| format!("\"{}\"", escape(p))).collect();
                println!("[{}]", names.join(","));
            } else {
                for p in &eq.profiles {
                    println!("{} {p}", if *p == eq.profile { "*" } else { " " });
                }
            }
            return Ok(());
        }
        Some("tuning") => match params.get(1) {
            None => {}
            Some(v) => drop(proxy.set_eq_correction(v).map_err(dbus_err)?),
        },
        // Profile names have spaces, so take everything after `eq`.
        Some(_) => drop(proxy.set_eq_profile(&params.join(" ")).map_err(dbus_err)?),
    }
    let eq = Eq::read(proxy)?;
    if json {
        println!("{}", eq.json());
    } else {
        println!("{}", eq.summary());
        if !eq.enabled && !params.is_empty() && !matches!(params[0].as_str(), "off") {
            println!("(the EQ is off; `quantumenginectl eq on` applies it)");
        }
    }
    Ok(())
}

fn cmd_update(proxy: &HeadsetProxyBlocking, params: &[String], json: bool) -> Result<(), String> {
    match params.first().map(String::as_str) {
        None | Some("check") => {
            proxy.check_for_updates().map_err(dbus_err)?;
            let u = UpdateInfo::read(proxy)?;
            if json {
                println!("{}", u.json());
            } else if u.available.is_empty() {
                println!("QuantumEngine {} is up to date", u.version);
            } else {
                println!("QuantumEngine {} is available (you have {})", u.available, u.version);
                println!("{}", u.url);
                if u.can_install {
                    println!("Install it with: quantumenginectl update install");
                } else {
                    println!("This copy wasn't installed by pacman; update it the way you installed it.");
                }
            }
            Ok(())
        }
        Some("install") => cmd_update_install(proxy),
        Some("checks") => {
            match params.get(1).map(String::as_str) {
                None => {}
                Some(v) => drop(proxy.set_update_checks(parse_on_off(v)?).map_err(dbus_err)?),
            }
            let on = proxy.update_checks().map_err(dbus_err)?;
            emit(json, "update_checks", if on { "on" } else { "off" });
            Ok(())
        }
        Some(other) => Err(format!("unknown update command '{other}' (check, install, checks)")),
    }
}

/// Starts the install and follows it until the service restarts with the new
/// version or reports a failure.
fn cmd_update_install(proxy: &HeadsetProxyBlocking) -> Result<(), String> {
    let available = proxy.check_for_updates().map_err(dbus_err)?;
    if available.is_empty() {
        println!("QuantumEngine {} is up to date", proxy.version().map_err(dbus_err)?);
        return Ok(());
    }
    proxy.install_update().map_err(dbus_err)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(15 * 60);
    let mut last = String::new();
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(500));
        let (status, busy, version) = match (proxy.update_status(), proxy.update_busy(), proxy.version()) {
            (Ok(s), Ok(b), Ok(v)) => (s, b, v),
            // The old service has exited and the new one isn't up yet.
            _ => continue,
        };
        if version == available {
            println!("QuantumEngine {available} is installed and running.");
            return Ok(());
        }
        if status != last && !status.is_empty() {
            println!("{status}");
            last = status.clone();
        }
        if !busy && status.starts_with("Update failed") {
            return Err(status);
        }
    }
    Err("the update is taking too long; check `quantumenginectl update`".into())
}

/// The message of a D-Bus error, without the error name zbus puts in front.
fn dbus_err(e: zbus::Error) -> String {
    match e {
        zbus::Error::MethodError(_, Some(msg), _) => msg,
        other => other.to_string(),
    }
}

fn link_note(link: &str) -> &'static str {
    match link {
        "no_dongle" => "no dongle plugged in",
        "no_access" => "cannot access the dongle",
        _ => "not connected (dongle present, headset off or out of range)",
    }
}

fn cmd_rgb(proxy: &HeadsetProxyBlocking, params: &[String]) -> Result<(), String> {
    let mut brightness: u8 = 100;
    let mut positional: Vec<&str> = Vec::new();
    let mut it = params.iter();
    while let Some(arg) = it.next() {
        if arg == "--brightness" {
            let value = it.next().ok_or("--brightness needs a value 0-100")?;
            brightness = value.parse().map_err(|_| format!("invalid brightness '{value}'"))?;
        } else {
            positional.push(arg);
        }
    }

    let [zone_name, effect_name, colours @ ..] = positional.as_slice() else {
        return Err("usage: quantumenginectl rgb <zone> <effect> <colour>... (try --help)".into());
    };

    let zones: &[Zone] = match *zone_name {
        "logo" => &[Zone::Logo],
        "ring" => &[Zone::Ring],
        "both" => &Zone::ALL,
        other => return Err(format!("unknown zone '{other}' (logo, ring, both)")),
    };

    let effect = Effect::from_name(effect_name).ok_or_else(|| {
        let names: Vec<&str> = Effect::KNOWN.iter().map(|(_, n)| *n).collect();
        format!("unknown effect '{effect_name}' (one of: {})", names.join(", "))
    })?;

    if colours.is_empty() {
        return Err("give at least one colour, e.g. '#ff0000'".into());
    }
    if colours.len() > MAX_SLOTS {
        return Err(format!("at most {MAX_SLOTS} colours, got {}", colours.len()));
    }

    let palette: Vec<Colour> = colours
        .iter()
        .map(|c| Colour::parse_hex(c).ok_or_else(|| format!("invalid colour '{c}' (expected #rrggbb)")))
        .collect::<Result<_, _>>()?;

    let args: Vec<ZoneLightingArg> = zones
        .iter()
        .map(|&zone| ZoneLightingArg {
            zone: match zone {
                Zone::Logo => "logo".to_string(),
                Zone::Ring => "ring".to_string(),
            },
            brightness,
            colours: palette.iter().map(|c| (c.r, c.g, c.b)).collect(),
            effect: effect.0,
        })
        .collect();

    proxy.set_lighting(args).map_err(dbus_err)
}

/// State `watch` compares between polls, since the service exposes changes
/// as properties rather than raw device events — see `quantumengine-dbus`.
/// A physical Bluetooth reconnect or an unrecognised report, which the old
/// direct-hardware `watch` could show, has no property here and so is not
/// visible over D-Bus.
#[derive(Clone, PartialEq)]
struct WatchState {
    link: String,
    battery: u8,
    anc: String,
    sidetone: String,
    mic_active: bool,
    dial: u8,
    lights_on: bool,
}

impl WatchState {
    fn read(proxy: &HeadsetProxyBlocking) -> Result<Self, String> {
        Ok(Self {
            link: proxy.link().map_err(dbus_err)?,
            battery: proxy.battery().map_err(dbus_err)?,
            anc: proxy.anc().map_err(dbus_err)?,
            sidetone: proxy.sidetone().map_err(dbus_err)?,
            mic_active: proxy.mic_active().map_err(dbus_err)?,
            dial: proxy.dial().map_err(dbus_err)?,
            lights_on: proxy.lights_on().map_err(dbus_err)?,
        })
    }
}

fn cmd_watch(proxy: &HeadsetProxyBlocking, json: bool) -> Result<(), String> {
    let mut prev = WatchState::read(proxy)?;
    loop {
        std::thread::sleep(WATCH_POLL_EVERY);
        let cur = WatchState::read(proxy)?;
        if cur.link != prev.link {
            print_watch_event(json, "link", &format!("\"{}\"", cur.link), &format!("link: {}", cur.link));
        }
        if cur.battery != prev.battery {
            print_watch_event(json, "battery", &cur.battery.to_string(), &format!("battery: {}%", cur.battery));
        }
        if cur.anc != prev.anc {
            print_watch_event(json, "anc", &format!("\"{}\"", cur.anc), &format!("anc: {}", cur.anc));
        }
        if cur.sidetone != prev.sidetone {
            print_watch_event(json, "sidetone", &format!("\"{}\"", cur.sidetone), &format!("sidetone: {}", cur.sidetone));
        }
        if cur.mic_active != prev.mic_active {
            let text = if cur.mic_active { "active" } else { "inactive" };
            print_watch_event(json, "mic_active", &cur.mic_active.to_string(), &format!("mic: {text}"));
        }
        if cur.dial != prev.dial {
            print_watch_event(json, "dial", &cur.dial.to_string(), &format!("dial: {} ({})", cur.dial, dial_label(cur.dial)));
        }
        if cur.lights_on != prev.lights_on {
            let text = if cur.lights_on { "on" } else { "off" };
            print_watch_event(json, "lights", &cur.lights_on.to_string(), &format!("lights: {text}"));
        }
        prev = cur;
    }
}

fn print_watch_event(json: bool, key: &str, json_value: &str, text: &str) {
    if json {
        println!("{{\"event\":\"{key}\",\"value\":{json_value}}}");
    } else {
        println!("{}  {text}", clock());
    }
}

fn emit(json: bool, key: &str, value: &str) {
    if json {
        println!("{{\"{key}\":\"{value}\"}}");
    } else {
        println!("{value}");
    }
}

fn dial_label(pos: u8) -> &'static str {
    match pos {
        0 => "full chat",
        1..=7 => "chat",
        8 => "centre",
        9..=15 => "game",
        _ => "full game",
    }
}

/// Local wall-clock time as HH:MM:SS, without pulling in a date crate.
fn clock() -> String {
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&now, &mut tm) };
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Validates an ANC value locally, so a typo fails fast without waiting on
/// the service. The service parses it again regardless (see
/// `dbus_service::parse_anc`), since it must trust no other caller either.
fn parse_anc(value: &str) -> Result<(), String> {
    match value {
        "off" | "on" => Ok(()),
        "talkthru" => Ok(()),
        other => Err(format!("unknown anc mode '{other}' (off, on, talkthru)")),
    }
}

fn parse_sidetone(value: &str) -> Result<(), String> {
    match value {
        "off" | "low" | "mid" | "high" => Ok(()),
        other => Err(format!("unknown sidetone level '{other}' (off, low, mid, high)")),
    }
}

fn parse_on_off(value: &str) -> Result<bool, String> {
    match value {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(format!("expected on or off, got '{other}'")),
    }
}
