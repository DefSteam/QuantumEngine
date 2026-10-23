//! Connects `quantumenginectl` to the `quantumengined` service over
//! D-Bus, starting it if it is not already running. Like the window and the
//! tray icon, this CLI never opens the headset itself — see
//! `quantumengine-dbus` for the shared contract.

use quantumengine_dbus::HeadsetProxyBlocking;
use std::time::{Duration, Instant};

/// How long to wait for a `quantumengined` that is starting (ours or one
/// someone else just started) to claim the bus name and answer.
const SERVICE_START_TIMEOUT: Duration = Duration::from_secs(10);

/// The longest any one call may go unanswered. Without it, a call the
/// service never answers would leave this command, and whatever runs it
/// (the Plasma widget), waiting forever.
const CALL_TIMEOUT: Duration = Duration::from_secs(20);

/// Connects to the running service, or starts one and waits for it.
pub fn connect() -> Result<HeadsetProxyBlocking<'static>, String> {
    let conn = zbus::blocking::connection::Builder::session()
        .and_then(|b| b.method_timeout(CALL_TIMEOUT).build())
        .map_err(|e| format!("cannot reach the session bus: {e}"))?;
    let bus = zbus::blocking::fdo::DBusProxy::new(&conn)
        .map_err(|e| format!("cannot reach the session bus: {e}"))?;
    let name = || quantumengine_dbus::BUS_NAME.try_into().expect("a valid bus name");
    if !bus.name_has_owner(name()).unwrap_or(false) {
        start_service()?;
        let deadline = Instant::now() + SERVICE_START_TIMEOUT;
        while !bus.name_has_owner(name()).unwrap_or(false) {
            if Instant::now() >= deadline {
                return Err("quantumengined did not start in time".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    // No property cache: a one-shot CLI invocation reads each property at
    // most once anyway, so there is nothing for a cache to save.
    let proxy = HeadsetProxyBlocking::builder(&conn)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .map_err(|e| format!("cannot reach the headset service: {e}"))?;
    wait_until_serving(&proxy)?;
    Ok(proxy)
}

/// A service that has only just started owns its name a moment before it
/// answers on it, and calls in between fail with "unknown object". Waits
/// that out; anything else is a real error.
fn wait_until_serving(proxy: &HeadsetProxyBlocking<'static>) -> Result<(), String> {
    let deadline = Instant::now() + SERVICE_START_TIMEOUT;
    loop {
        match proxy.version() {
            Ok(_) => return Ok(()),
            Err(e) if still_starting(&e) && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(e) if still_starting(&e) => return Err("quantumengined did not start answering in time".into()),
            Err(e) => return Err(format!("cannot reach the headset service: {e}")),
        }
    }
}

/// Whether `e` means the service is up but hasn't registered its interface yet.
fn still_starting(e: &zbus::Error) -> bool {
    match e {
        zbus::Error::MethodError(name, _, _) => {
            let name = name.as_str();
            name.ends_with(".UnknownObject")
                || name.ends_with(".UnknownInterface")
                || name.ends_with(".UnknownMethod")
                || name.ends_with(".UnknownProperty")
                || name.ends_with(".ServiceUnknown")
        }
        zbus::Error::FDO(e) => matches!(
            **e,
            zbus::fdo::Error::UnknownObject(_)
                | zbus::fdo::Error::UnknownInterface(_)
                | zbus::fdo::Error::UnknownMethod(_)
                | zbus::fdo::Error::UnknownProperty(_)
                | zbus::fdo::Error::ServiceUnknown(_)
        ),
        _ => false,
    }
}

/// Starts `quantumengined`, found next to this executable — a dev
/// build and the packaged install both put both binaries in the same
/// directory.
///
/// Its stdio is detached (not inherited) rather than left to default: the
/// service outlives this one-shot CLI invocation, so if it inherited, say, a
/// pipe this command's own output was captured into, that pipe would never
/// see EOF and whatever is reading it would hang forever waiting for a
/// process that has no reason to exit.
fn start_service() -> Result<(), String> {
    let mut exe = std::env::current_exe().map_err(|e| format!("cannot find my own executable: {e}"))?;
    exe.set_file_name("quantumengined");
    std::process::Command::new(&exe)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot start {}: {e}", exe.display()))
}
