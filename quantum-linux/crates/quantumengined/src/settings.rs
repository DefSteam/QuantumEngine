//! The service's own settings, in `$XDG_CONFIG_HOME/quantumengine/settings.json`.
//!
//! Only `quantumengined` reads or writes this file. Everything else
//! changes settings through it over D-Bus, so there is one writer and nothing
//! to merge.

use crate::eq::custom::CustomProfile;
use crate::eq::profiles::Correction;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub eq: EqSettings,
    pub surround: SurroundSettings,
    pub updates: UpdateSettings,
    /// What the user chose for each unverified model, by `vendor:product`.
    pub unverified: BTreeMap<String, UnverifiedChoice>,
}

/// Opt-in for one unverified model. Everything is off until the user turns it on.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UnverifiedChoice {
    pub in_use: bool,
    /// Any of `anc`, `sidetone`, `lights`.
    pub allowed: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SurroundSettings {
    pub enabled: bool,
    /// Slider percentage; the applied linear gain is twice this value (150 means 3x).
    pub output_gain: u16,
    #[serde(default = "default_surround_profile_name")]
    pub profile: String,
    #[serde(default = "default_surround_profiles")]
    pub profiles: BTreeMap<String, SurroundProfile>,
}

impl Default for SurroundSettings {
    fn default() -> Self {
        Self { enabled: false, output_gain: 150, profile: "Default".into(), profiles: default_surround_profiles() }
    }
}

/// User-adjustable rendering values applied to the KEMAR-derived filters.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct SurroundProfile {
    /// HRTF interaural difference strength, percent; 100 is the measured value.
    pub spatial_cues: u8,
    /// Early reflection pressure per bounce, percent.
    pub reflections: u8,
    /// Late reverberation energy relative to direct sound, percent.
    pub ambience: u8,
    /// Late reverberation decay time in milliseconds.
    pub decay_ms: u16,
}

impl Default for SurroundProfile {
    fn default() -> Self {
        Self { spatial_cues: 100, reflections: 45, ambience: 10, decay_ms: 280 }
    }
}

fn default_surround_profiles() -> BTreeMap<String, SurroundProfile> {
    [
        ("Default".into(), SurroundProfile::default()),
        ("Wide".into(), SurroundProfile { spatial_cues: 125, reflections: 45, ambience: 15, decay_ms: 320 }),
        ("Room".into(), SurroundProfile { spatial_cues: 105, reflections: 60, ambience: 22, decay_ms: 420 }),
        ("Clear".into(), SurroundProfile { spatial_cues: 110, reflections: 25, ambience: 3, decay_ms: 180 }),
    ].into_iter().collect()
}

fn default_surround_profile_name() -> String {
    "Default".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EqSettings {
    pub enabled: bool,
    pub profile: String,
    pub correction: Correction,
    pub custom: Vec<CustomProfile>,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self { enabled: false, profile: "Flat".into(), correction: Correction::default(), custom: Vec::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateSettings {
    /// Whether to ask GitHub once a day for a newer release.
    pub check: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self { check: true }
    }
}

/// Serialises read-modify-write cycles between the EQ, surround and update
/// threads.
static LOCK: Mutex<()> = Mutex::new(());

fn path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("quantumengine").join("settings.json")
}

pub fn load() -> Settings {
    let _guard = LOCK.lock().unwrap();
    read()
}

fn read() -> Settings {
    let path = path();
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            eprintln!("quantumengined: ignoring {}: {e}", path.display());
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

/// Applies `change` to the stored settings and saves them.
pub fn update(change: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
    let _guard = LOCK.lock().unwrap();
    let mut settings = read();
    change(&mut settings);
    let path = path();
    let dir = path.parent().expect("settings path has a parent");
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let json = serde_json::to_vec_pretty(&settings).expect("settings serialise");
    // Write then rename, so a crash mid-write never leaves a truncated file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("cannot save {}: {e}", path.display()))?;
    Ok(settings)
}
