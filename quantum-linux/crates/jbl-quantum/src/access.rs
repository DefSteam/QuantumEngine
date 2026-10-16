//! Giving the current user access to one device, for a model that has no udev
//! rule of its own.
//!
//! The packaged rule (`udev/70-jbl-quantum.rules`) covers only the verified
//! product. For another model the user asks for access explicitly, one
//! `vendor:product` at a time, and approves it with their password; nothing
//! gets broader access than the one ID they named.

use std::io::Write;
use std::process::{Command, Stdio};

/// Parses `0ecb:203e` into `(0x0ecb, 0x203e)`. Exactly two 4-digit hex numbers.
pub fn parse_id(id: &str) -> Option<(u16, u16)> {
    let (vid, pid) = id.split_once(':')?;
    let hex4 = |s: &str| (s.len() == 4 && s.bytes().all(|b| b.is_ascii_hexdigit())).then(|| u16::from_str_radix(s, 16).ok()).flatten();
    Some((hex4(vid)?, hex4(pid)?))
}

/// Where the rule for `vid:pid` is written.
pub fn rule_path(vid: u16, pid: u16) -> String {
    format!("/etc/udev/rules.d/71-quantumengine-{vid:04x}-{pid:04x}.rules")
}

/// The rule's text. Only the two validated numbers go into it.
pub fn rule(vid: u16, pid: u16) -> String {
    format!(
        "# Added by QuantumEngine at the user's request, for one unverified device.\n\
         KERNEL==\"hidraw*\", ATTRS{{idVendor}}==\"{vid:04x}\", ATTRS{{idProduct}}==\"{pid:04x}\", TAG+=\"uaccess\"\n"
    )
}

/// Installs the rule through `pkexec` (asking for the user's password) and
/// reloads udev so it applies to the device already plugged in.
pub fn grant(vid: u16, pid: u16) -> Result<(), String> {
    let path = rule_path(vid, pid);
    // The script is fixed and the path is built from two validated numbers; the
    // rule itself arrives on stdin, so nothing user-supplied is interpreted by
    // the shell.
    let script = format!(
        "cat > '{path}' && udevadm control --reload-rules && udevadm trigger --subsystem-match=hidraw --action=change"
    );
    let mut child = Command::new("pkexec")
        .args(["sh", "-c", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run pkexec: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("cannot reach pkexec")?
        .write_all(rule(vid, pid).as_bytes())
        .map_err(|e| format!("cannot send the rule: {e}"))?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(126) => Err("the password prompt was cancelled".into()),
        Some(127) => Err("not authorised to change udev rules".into()),
        _ => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("could not install the rule: {}", stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("unknown error").trim()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_must_be_exactly_two_hex_numbers() {
        assert_eq!(parse_id("0ecb:203e"), Some((0x0ecb, 0x203e)));
        assert_eq!(parse_id("0ECB:203E"), Some((0x0ecb, 0x203e)));
        for bad in ["", "0ecb", "0ecb:203", "0ecb:203e0", "0ecb:20 e", "0ecb:203e; rm -rf /", "../0ecb:203e", "0ecb:+03e"] {
            assert_eq!(parse_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_rule_names_only_that_device() {
        let text = rule(0x0ecb, 0x203e);
        assert!(text.contains("idVendor}==\"0ecb\""));
        assert!(text.contains("idProduct}==\"203e\""));
        assert!(text.contains("uaccess"));
        assert_eq!(rule_path(0x0ecb, 0x203e), "/etc/udev/rules.d/71-quantumengine-0ecb-203e.rules");
    }
}
