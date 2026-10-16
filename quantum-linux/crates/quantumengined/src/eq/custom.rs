//! Profiles the user makes or imports, kept in the service's settings next to
//! the built-in ones.
//!
//! Imports understand the two formats people actually share curves in:
//! * Equalizer APO's parametric text, which is also what AutoEQ publishes as
//!   `ParametricEQ.txt` for thousands of headphones;
//! * EasyEffects presets (JSON), from their equalizer.

use super::biquad::{Band, FilterType};
use super::profiles;
use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_1_SQRT_2;

/// The filter has `pipewire::SLOTS` biquads: one is kept for the volume
/// turn-down and up to five go to JBL's headset tuning.
pub const MAX_BANDS: usize = super::pipewire::SLOTS - 1 - 5;
pub const MAX_NAME: usize = 40;
const FREQ_RANGE: (f64, f64) = (10.0, 20_000.0);
const GAIN_RANGE: (f64, f64) = (-30.0, 30.0);
const Q_RANGE: (f64, f64) = (0.05, 20.0);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomProfile {
    pub name: String,
    pub bands: Vec<Band>,
}

/// Checks a profile name: not empty, not too long, and not taken by a
/// built-in profile or another custom one (`except` is the profile being
/// renamed, which may keep its own name). Returns it trimmed.
pub fn check_name(name: &str, custom: &[CustomProfile], except: Option<&str>) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the profile a name.".into());
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!("Profile names can be up to {MAX_NAME} characters."));
    }
    if profiles::normalise(name).is_empty() {
        return Err("Profile names need at least one letter or digit.".into());
    }
    if profiles::find(name).is_some() {
        return Err(format!("“{name}” is a built-in profile; pick another name."));
    }
    let key = profiles::normalise(name);
    let taken = custom.iter().any(|p| {
        profiles::normalise(&p.name) == key && except.is_none_or(|e| profiles::normalise(e) != key)
    });
    if taken {
        return Err(format!("There's already a profile called “{name}”."));
    }
    Ok(name.to_string())
}

/// Checks that every band is one the filter can play.
pub fn check_bands(bands: &[Band]) -> Result<(), String> {
    if bands.len() > MAX_BANDS {
        return Err(format!("A profile can have up to {MAX_BANDS} bands; this has {}.", bands.len()));
    }
    for (i, b) in bands.iter().enumerate() {
        let n = i + 1;
        if !(FREQ_RANGE.0..=FREQ_RANGE.1).contains(&b.freq) || !b.freq.is_finite() {
            return Err(format!("Band {n}: the frequency must be between 10 Hz and 20 kHz."));
        }
        if !(GAIN_RANGE.0..=GAIN_RANGE.1).contains(&b.gain_db) || !b.gain_db.is_finite() {
            return Err(format!("Band {n}: the gain must be between -30 and +30 dB."));
        }
        if !(Q_RANGE.0..=Q_RANGE.1).contains(&b.q) || !b.q.is_finite() {
            return Err(format!("Band {n}: Q must be between 0.05 and 20."));
        }
    }
    Ok(())
}

/// A name for an imported profile that isn't taken: `base`, else `base 2`,
/// `base 3` and so on.
pub fn unique_name(base: &str, custom: &[CustomProfile]) -> String {
    let base: String = base.trim().chars().take(MAX_NAME - 3).collect();
    let base = if profiles::normalise(&base).is_empty() { "Imported".to_string() } else { base };
    if check_name(&base, custom, None).is_ok() {
        return base;
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| check_name(name, custom, None).is_ok())
        .expect("some suffix is free")
}

/// Reads a curve in Equalizer APO / AutoEQ text or EasyEffects JSON.
pub fn parse_import(text: &str) -> Result<Vec<Band>, String> {
    let bands = if text.trim_start().starts_with('{') { parse_easyeffects(text)? } else { parse_eqapo(text)? };
    if bands.is_empty() {
        return Err("No filters found. Use an Equalizer APO or AutoEQ ParametricEQ.txt, or an EasyEffects preset.".into());
    }
    check_bands(&bands)?;
    Ok(bands)
}

/// Equalizer APO's text: one `Filter: ON PK Fc 100 Hz Gain -2 dB Q 1.41`
/// per line. `Preamp` is ignored: the service works out its own turn-down.
fn parse_eqapo(text: &str) -> Result<Vec<Band>, String> {
    let mut bands = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        let Some((key, rest)) = line.split_once(':') else { continue };
        let key = key.trim().to_ascii_lowercase();
        if key.starts_with("graphiceq") {
            return Err("GraphicEQ files aren't supported; use the ParametricEQ version of the curve.".into());
        }
        if !key.starts_with("filter") {
            continue; // Preamp, Channel, Include, comments...
        }
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.first().is_some_and(|t| t.eq_ignore_ascii_case("off")) {
            continue;
        }
        let tokens = if tokens.first().is_some_and(|t| t.eq_ignore_ascii_case("on")) { &tokens[1..] } else { &tokens[..] };
        let Some(kind) = tokens.first() else { continue };
        let kind = match kind.to_ascii_uppercase().as_str() {
            "PK" | "PEQ" | "MODAL" => FilterType::Bell,
            "LS" | "LSC" | "LSQ" => FilterType::LowShelf,
            "HS" | "HSC" | "HSQ" => FilterType::HighShelf,
            "HP" | "HPQ" => FilterType::HighPass,
            "LP" | "LPQ" => FilterType::LowPass,
            "NO" | "NONE" => continue,
            other => return Err(format!("Line {}: filter type {other} isn't supported.", i + 1)),
        };
        let value = |name: &str| -> Option<f64> {
            let pos = tokens.iter().position(|t| t.eq_ignore_ascii_case(name))?;
            tokens.get(pos + 1)?.parse().ok()
        };
        let freq = value("Fc").ok_or_else(|| format!("Line {}: no frequency (Fc).", i + 1))?;
        let gain_db = if kind.has_gain() { value("Gain").unwrap_or(0.0) } else { 0.0 };
        let q = match (value("Q"), bandwidth(tokens)) {
            (Some(q), _) => q,
            (None, Some(octaves)) => q_from_octaves(octaves),
            (None, None) => FRAC_1_SQRT_2,
        };
        bands.push(Band { kind, freq, gain_db, q });
    }
    Ok(bands)
}

/// `BW Oct 1.5`: a bandwidth in octaves, as Equalizer APO writes it.
fn bandwidth(tokens: &[&str]) -> Option<f64> {
    let pos = tokens.iter().position(|t| t.eq_ignore_ascii_case("BW"))?;
    let next = tokens.get(pos + 1)?;
    let n = if next.eq_ignore_ascii_case("oct") { tokens.get(pos + 2)? } else { next };
    n.parse().ok()
}

fn q_from_octaves(n: f64) -> f64 {
    let p = 2f64.powf(n);
    p.sqrt() / (p - 1.0)
}

/// An EasyEffects preset: the left channel of its output equalizer.
fn parse_easyeffects(text: &str) -> Result<Vec<Band>, String> {
    let json: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("Not a valid EasyEffects preset: {e}"))?;
    let eq = json["output"]
        .as_object()
        .and_then(|o| o.iter().find(|(k, _)| k.starts_with("equalizer")).map(|(_, v)| v))
        .ok_or("This EasyEffects preset has no output equalizer.")?;
    let channel = &eq["left"];
    let count = eq["num-bands"].as_u64().unwrap_or(0) as usize;
    let mut bands = Vec::new();
    for i in 0..count {
        let b = &channel[format!("band{i}")];
        if b["mute"].as_bool() == Some(true) {
            continue;
        }
        let kind = match b["type"].as_str().unwrap_or("Bell") {
            "Bell" => FilterType::Bell,
            "Lo-shelf" => FilterType::LowShelf,
            "Hi-shelf" => FilterType::HighShelf,
            "Hi-pass" => FilterType::HighPass,
            "Lo-pass" => FilterType::LowPass,
            "Off" => continue,
            other => return Err(format!("Band {}: EasyEffects filter type “{other}” isn't supported.", i + 1)),
        };
        let num = |k: &str| b[k].as_f64().ok_or_else(|| format!("Band {}: no {k}.", i + 1));
        bands.push(Band {
            kind,
            freq: num("frequency")?,
            gain_db: if kind.has_gain() { num("gain")? } else { 0.0 },
            q: num("q").unwrap_or(FRAC_1_SQRT_2),
        });
    }
    Ok(bands)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTOEQ: &str = "Preamp: -6.4 dB
Filter 1: ON LSC Fc 105 Hz Gain 6.6 dB Q 0.70
Filter 2: ON PK Fc 2930 Hz Gain 3.3 dB Q 2.70
Filter 3: OFF PK Fc 5000 Hz Gain 9 dB Q 1
Filter 4: ON HSC Fc 10000 Hz Gain -1.9 dB Q 0.70
Filter 5: ON HP Fc 20 Hz
Filter: ON PK Fc 800 Hz Gain -2 dB BW Oct 1
";

    #[test]
    fn reads_autoeq_and_equalizer_apo_text() {
        let bands = parse_import(AUTOEQ).unwrap();
        assert_eq!(bands.len(), 5);
        assert_eq!(bands[0], Band { kind: FilterType::LowShelf, freq: 105.0, gain_db: 6.6, q: 0.70 });
        assert_eq!(bands[1].kind, FilterType::Bell);
        assert_eq!(bands[2].kind, FilterType::HighShelf);
        assert_eq!(bands[3], Band { kind: FilterType::HighPass, freq: 20.0, gain_db: 0.0, q: FRAC_1_SQRT_2 });
        assert!((bands[4].q - std::f64::consts::SQRT_2).abs() < 1e-9, "1 octave is Q √2");
    }

    #[test]
    fn reads_easyeffects_presets() {
        let preset = r#"{"output":{"blocklist":[],"equalizer#0":{"num-bands":3,"left":{
            "band0":{"frequency":60.0,"gain":4.0,"q":0.7,"type":"Lo-shelf","mute":false},
            "band1":{"frequency":1000.0,"gain":-3.5,"q":2.0,"type":"Bell","mute":false},
            "band2":{"frequency":4000.0,"gain":9.0,"q":1.0,"type":"Bell","mute":true}}}}}"#;
        let bands = parse_import(preset).unwrap();
        assert_eq!(bands, vec![
            Band { kind: FilterType::LowShelf, freq: 60.0, gain_db: 4.0, q: 0.7 },
            Band { kind: FilterType::Bell, freq: 1000.0, gain_db: -3.5, q: 2.0 },
        ]);
    }

    #[test]
    fn rejects_what_the_filter_cannot_play() {
        assert!(parse_import("GraphicEQ: 20 -1; 30 -2").is_err());
        assert!(parse_import("Filter 1: ON XYZ Fc 100 Hz").is_err());
        assert!(parse_import("Preamp: -3 dB").is_err());
        assert!(check_bands(&[Band::bell(5.0, 0.0, 1.0)]).is_err());
        assert!(check_bands(&vec![Band::bell(100.0, 1.0, 1.0); MAX_BANDS + 1]).is_err());
    }

    #[test]
    fn the_contract_states_the_same_band_limit() {
        assert_eq!(MAX_BANDS, quantumengine_dbus::EQ_MAX_BANDS);
    }

    #[test]
    fn names_must_be_free() {
        let mine = vec![CustomProfile { name: "My Curve".into(), bands: vec![] }];
        assert!(check_name("  ", &mine, None).is_err());
        assert!(check_name("bass boost", &mine, None).is_err(), "built-in");
        assert!(check_name("my-curve", &mine, None).is_err(), "taken");
        assert_eq!(check_name("my curve", &mine, Some("My Curve")).unwrap(), "my curve", "renaming itself");
        assert_eq!(unique_name("My Curve", &mine), "My Curve 2");
        assert_eq!(unique_name("Fresh", &mine), "Fresh");
    }
}
