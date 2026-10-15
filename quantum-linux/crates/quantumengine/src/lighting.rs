//! The lighting the user last set up. The headset can't report its lighting,
//! so the window remembers what it last applied, in
//! `$XDG_CONFIG_HOME/quantumengine/lighting.json`.

use crate::dto::{LightingDto, ZoneDto};
use jbl_quantum::{Effect, MAX_SLOTS};
use std::path::PathBuf;

impl Default for LightingDto {
    fn default() -> Self {
        let zone = ZoneDto { brightness: 100, colours: vec!["#8a3cff".into()], effect: Effect::FADE.0 };
        Self { logo: zone.clone(), ring: zone }
    }
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("quantumengine").join("lighting.json"))
}

pub fn load() -> LightingDto {
    path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| serde_json::from_slice::<LightingDto>(&bytes).ok())
        .and_then(|l| validate(l).ok())
        .unwrap_or_default()
}

pub fn save(lighting: &LightingDto) -> Result<(), String> {
    let path = path().ok_or("No home directory to save lighting in.")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Cannot save lighting: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(lighting).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| format!("Cannot save lighting: {e}"))
}

/// Checks what the UI sent before it reaches the headset or the disk.
pub fn validate(lighting: LightingDto) -> Result<LightingDto, String> {
    for zone in [&lighting.logo, &lighting.ring] {
        if zone.brightness > 100 {
            return Err("Brightness goes up to 100%.".into());
        }
        if zone.colours.is_empty() || zone.colours.len() > MAX_SLOTS {
            return Err(format!("Each zone takes 1 to {MAX_SLOTS} colours."));
        }
        for c in &zone.colours {
            parse_hex(c)?;
        }
    }
    Ok(lighting)
}

pub fn parse_hex(colour: &str) -> Result<(u8, u8, u8), String> {
    let hex = colour.strip_prefix('#').unwrap_or(colour);
    let bad = || format!("'{colour}' isn't a #rrggbb colour.");
    if hex.len() != 6 || !hex.is_ascii() {
        return Err(bad());
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| bad());
    Ok((byte(0)?, byte(2)?, byte(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colours_parse() {
        assert_eq!(parse_hex("#ff5a00"), Ok((0xff, 0x5a, 0x00)));
        assert_eq!(parse_hex("8A3CFF"), Ok((0x8a, 0x3c, 0xff)));
        assert!(parse_hex("#fff").is_err());
        assert!(parse_hex("#gg0000").is_err());
    }

    #[test]
    fn zones_are_checked() {
        let mut l = LightingDto::default();
        assert!(validate(l.clone()).is_ok());
        l.ring.colours = vec!["#000000".into(); MAX_SLOTS + 1];
        assert!(validate(l.clone()).is_err());
        l.ring.colours.clear();
        assert!(validate(l).is_err());
    }
}
