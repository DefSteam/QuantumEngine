//! The commands the UI calls. Each is one D-Bus call to `quantumengined`,
//! run off the main thread: the headset setters wait for the headset to
//! confirm, which can take a couple of seconds.

use crate::dto::{EffectDto, InfoDto, LightingDto, StateDto};
use crate::lighting;
use crate::service::Link;
use jbl_quantum::{Effect, MAX_SLOTS};
use quantumengine_dbus::{HeadsetProxyBlocking, ZoneLightingArg};
use std::sync::Arc;
use tauri::State;

type Shared<'a> = State<'a, Arc<Link>>;
type Result<T> = std::result::Result<T, String>;

/// Runs `call` on a proxy, on a blocking thread.
async fn with_proxy<T: Send + 'static>(
    link: &Arc<Link>,
    call: impl FnOnce(&HeadsetProxyBlocking<'static>) -> zbus::Result<T> + Send + 'static,
) -> Result<T> {
    let link = Arc::clone(link);
    tauri::async_runtime::spawn_blocking(move || {
        let proxy = link.proxy()?;
        call(&proxy).map_err(|e| error_text(&e))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The message of a D-Bus error, without the error name zbus puts first.
fn error_text(e: &zbus::Error) -> String {
    match e {
        zbus::Error::MethodError(_, Some(msg), _) => msg.clone(),
        other => other.to_string(),
    }
}

/// The latest state; after this, changes arrive as `StateEvent`s.
#[tauri::command]
#[specta::specta]
pub fn state(link: Shared<'_>) -> StateDto {
    link.state()
}

#[tauri::command]
#[specta::specta]
pub fn info() -> InfoDto {
    InfoDto {
        version: env!("CARGO_PKG_VERSION").into(),
        effects: Effect::KNOWN.iter().map(|&(e, name)| EffectDto { id: e.0, label: effect_label(name) }).collect(),
        max_colours: MAX_SLOTS as u32,
        eq_frequencies: (0..quantumengine_dbus::EQ_RESPONSE_POINTS).map(quantumengine_dbus::eq_response_freq).collect(),
    }
}

/// `"off"`, `"on"` or `"talkthru"`; returns the confirmed mode.
#[tauri::command]
#[specta::specta]
pub async fn set_anc(link: Shared<'_>, mode: String) -> Result<String> {
    with_proxy(&link, move |p| p.set_anc(&mode)).await
}

/// `"off"`, `"low"`, `"mid"` or `"high"`; returns the confirmed level.
#[tauri::command]
#[specta::specta]
pub async fn set_sidetone(link: Shared<'_>, level: String) -> Result<String> {
    with_proxy(&link, move |p| p.set_sidetone(&level)).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_lights(link: Shared<'_>, on: bool) -> Result<bool> {
    with_proxy(&link, move |p| p.set_lights(on)).await
}

/// The lighting last applied from this window.
#[tauri::command]
#[specta::specta]
pub fn lighting() -> LightingDto {
    lighting::load()
}

/// Sends both zones to the headset (which switches the lights on) and
/// remembers them.
#[tauri::command]
#[specta::specta]
pub async fn apply_lighting(link: Shared<'_>, lighting: LightingDto) -> Result<()> {
    let lighting = lighting::validate(lighting)?;
    let zones = [("logo", &lighting.logo), ("ring", &lighting.ring)]
        .into_iter()
        .map(|(name, z)| {
            Ok(ZoneLightingArg {
                zone: name.into(),
                brightness: z.brightness,
                colours: z.colours.iter().map(|c| lighting::parse_hex(c)).collect::<Result<_>>()?,
                effect: z.effect,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // Remembered first: even if the headset is slow to confirm, this is what
    // was asked for.
    lighting::save(&lighting)?;
    with_proxy(&link, move |p| p.set_lighting(zones)).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_eq_enabled(link: Shared<'_>, enabled: bool) -> Result<bool> {
    with_proxy(&link, move |p| p.set_eq_enabled(enabled)).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_eq_profile(link: Shared<'_>, name: String) -> Result<String> {
    with_proxy(&link, move |p| p.set_eq_profile(&name)).await
}

/// `"off"`, `"stereo"` or `"surround"`.
#[tauri::command]
#[specta::specta]
pub async fn set_eq_correction(link: Shared<'_>, correction: String) -> Result<String> {
    with_proxy(&link, move |p| p.set_eq_correction(&correction)).await
}

/// Virtual 7.1 surround; on also makes it the default output.
#[tauri::command]
#[specta::specta]
pub async fn set_surround_enabled(link: Shared<'_>, enabled: bool) -> Result<bool> {
    with_proxy(&link, move |p| p.set_surround_enabled(enabled)).await
}

/// The curve of `profile` under `correction`, in dB at each of
/// `InfoDto::eq_frequencies`.
#[tauri::command]
#[specta::specta]
pub async fn eq_response(link: Shared<'_>, profile: String, correction: String) -> Result<Vec<f64>> {
    with_proxy(&link, move |p| p.eq_response(&profile, &correction)).await
}

/// Returns the newer version, or empty when up to date.
#[tauri::command]
#[specta::specta]
pub async fn check_for_updates(link: Shared<'_>) -> Result<String> {
    with_proxy(&link, |p| p.check_for_updates()).await
}

/// Starts installing; progress shows in the state's `update.status`.
#[tauri::command]
#[specta::specta]
pub async fn install_update(link: Shared<'_>) -> Result<()> {
    with_proxy(&link, |p| p.install_update()).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_update_checks(link: Shared<'_>, enabled: bool) -> Result<bool> {
    with_proxy(&link, move |p| p.set_update_checks(enabled)).await
}

/// Replaces an older running service with the installed one.
#[tauri::command]
#[specta::specta]
pub async fn restart_service(link: Shared<'_>) -> Result<()> {
    let link = Arc::clone(&link);
    tauri::async_runtime::spawn_blocking(move || crate::service::restart(&link))
        .await
        .map_err(|e| e.to_string())?
}

/// Opens the available release's page in the browser. The URL comes from
/// the service, never from the page, and must be this project's.
#[tauri::command]
#[specta::specta]
pub async fn open_release_page(link: Shared<'_>) -> Result<()> {
    let url = with_proxy(&link, |p| p.update_url()).await?;
    let ours = concat!(env!("CARGO_PKG_REPOSITORY"), "/");
    if !url.starts_with(ours) {
        return Err("There's no release page to open.".into());
    }
    open_url(&url)
}

/// Opens this project's page (the About section's link).
#[tauri::command]
#[specta::specta]
pub fn open_project_page() -> Result<()> {
    open_url(env!("CARGO_PKG_REPOSITORY"))
}

fn open_url(url: &str) -> Result<()> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| format!("Cannot open the browser: {e}"))
}

/// `chase-strobe-logo` -> `Chase strobe logo`.
fn effect_label(name: &str) -> String {
    let words = name.replace('-', " ");
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_labels_read_as_words() {
        assert_eq!(effect_label("chase-strobe-logo"), "Chase strobe logo");
        assert_eq!(effect_label("fade"), "Fade");
    }
}
