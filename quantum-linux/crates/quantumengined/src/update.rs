//! Checks GitHub for a newer release and, when QuantumEngine was installed as
//! a pacman package, installs it.
//!
//! Each release built by `.github/workflows/release.yml` carries the Arch
//! package and a `.sha256` file. Installing downloads both, checks one against
//! the other, and hands the package to `pkexec pacman -U`, so the user
//! confirms with their password and pacman stays in charge of the files. The
//! checksum catches a broken download; it is not a signature, so it trusts the
//! GitHub repository as much as `makepkg` from that repository would.
//!
//! The only network traffic is one request to `api.github.com` a day (and the
//! download itself), which the user can switch off.

use crate::notifier::Notifier;
use crate::settings;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The Arch package QuantumEngine ships as, which owns this binary and names
/// the release assets.
const PACKAGE: &str = "quantumengine";
const FIRST_CHECK_AFTER: Duration = Duration::from_secs(60);
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub url: String,
    /// The Arch package for this machine's architecture, if the release has one.
    pub package: Option<Asset>,
    pub checksum: Option<Asset>,
}

#[derive(Clone, Debug)]
pub struct State {
    /// A release newer than this one, once a check has found it.
    pub available: Option<Release>,
    /// What the updater is doing or last failed at; empty when idle.
    pub status: String,
    pub busy: bool,
    pub checks_enabled: bool,
    /// Whether this copy belongs to the pacman package, so an update can be
    /// installed rather than only announced.
    pub pacman_owned: bool,
}

impl State {
    /// Whether "Install" can be offered for the available release.
    pub fn can_install(&self) -> bool {
        self.pacman_owned
            && self.available.as_ref().is_some_and(|r| r.package.is_some() && r.checksum.is_some())
    }
}

pub type Reply<T> = Sender<Result<T, String>>;

pub enum Command {
    /// Checks now. Answers with the newer version, or `None` when up to date.
    Check(Reply<Option<String>>),
    /// Starts installing the available release. Answers once it has started;
    /// progress shows in [`State::status`].
    Install(Reply<()>),
    SetChecks(bool, Reply<bool>),
}

#[derive(Clone)]
pub struct Handle {
    state: Arc<Mutex<State>>,
    commands: Sender<Command>,
}

impl Handle {
    pub fn state(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.commands.send(cmd);
    }

    pub fn call<T>(&self, make: impl FnOnce(Reply<T>) -> Command) -> Result<T, String> {
        let (reply, rx) = mpsc::channel();
        self.commands.send(make(reply)).map_err(|_| "the update thread has stopped".to_string())?;
        rx.recv().map_err(|_| "no reply from the update thread".to_string())?
    }
}

pub fn no_reply<T>() -> Reply<T> {
    mpsc::channel().0
}

pub fn spawn(notifier: Notifier) -> Handle {
    let state = Arc::new(Mutex::new(State {
        available: None,
        status: String::new(),
        busy: false,
        checks_enabled: settings::load().updates.check,
        // Found out on the update thread: `pacman -Qqo` reads every installed
        // package's file list, which at login can take seconds, and this runs
        // before the service starts answering on D-Bus.
        pacman_owned: false,
    }));
    let (commands, rx) = mpsc::channel();
    let handle = Handle { state: state.clone(), commands };
    let worker = Worker { state, notifier, handle: handle.clone(), announced: None };
    std::thread::Builder::new()
        .name("update".into())
        .spawn(move || worker.run(rx))
        .expect("failed to start the update thread");
    handle
}

struct Worker {
    state: Arc<Mutex<State>>,
    notifier: Notifier,
    /// For the desktop notification's "Install" button to send through.
    handle: Handle,
    /// The last version a desktop notification was shown for.
    announced: Option<String>,
}

impl Worker {
    fn run(mut self, commands: Receiver<Command>) {
        let owned = pacman_owned();
        self.set(|s| s.pacman_owned = owned);
        let mut next_check = Instant::now() + FIRST_CHECK_AFTER;
        loop {
            let wait = next_check.saturating_duration_since(Instant::now());
            match commands.recv_timeout(wait) {
                Ok(Command::Check(reply)) => {
                    let result = self.check();
                    next_check = Instant::now() + CHECK_EVERY;
                    drop(reply.send(result));
                }
                Ok(Command::Install(reply)) => {
                    let state = self.state();
                    let release = match state.available.clone() {
                        _ if state.busy => Err("an update is already in progress".to_string()),
                        None => Err("there is no update to install".to_string()),
                        Some(_) if !state.can_install() => {
                            Err("this copy of QuantumEngine wasn't installed by pacman, so it can't update itself".to_string())
                        }
                        Some(r) => Ok(r),
                    };
                    match release {
                        Err(e) => drop(reply.send(Err(e))),
                        Ok(release) => {
                            drop(reply.send(Ok(())));
                            self.install(&release);
                        }
                    }
                }
                Ok(Command::SetChecks(on, reply)) => {
                    let result = settings::update(|s| s.updates.check = on).map(|_| {
                        self.state.lock().unwrap().checks_enabled = on;
                        on
                    });
                    self.notifier.notify();
                    drop(reply.send(result));
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.state().checks_enabled {
                        let _ = self.check();
                    }
                    next_check = Instant::now() + CHECK_EVERY;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn state(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    fn set(&self, f: impl FnOnce(&mut State)) {
        f(&mut self.state.lock().unwrap());
        self.notifier.notify();
    }

    fn check(&mut self) -> Result<Option<String>, String> {
        self.set(|s| {
            s.busy = true;
            s.status = "Checking for updates…".into();
        });
        let result = latest_release();
        match &result {
            Ok(Some(r)) if is_newer(&r.version, VERSION) => {
                let release = r.clone();
                self.set(|s| {
                    s.available = Some(release);
                    s.busy = false;
                    s.status.clear();
                });
                self.announce();
            }
            Ok(_) => self.set(|s| {
                s.available = None;
                s.busy = false;
                s.status = format!("QuantumEngine {VERSION} is up to date");
            }),
            Err(e) => {
                eprintln!("quantumengined: update check failed: {e}");
                self.set(|s| {
                    s.busy = false;
                    s.status = format!("Couldn't check for updates: {e}");
                });
            }
        }
        result.map(|r| r.filter(|r| is_newer(&r.version, VERSION)).map(|r| r.version))
    }

    /// Shows a desktop notification for the available release, once per version.
    fn announce(&mut self) {
        let state = self.state();
        let Some(release) = state.available.clone() else { return };
        if self.announced.as_deref() == Some(&release.version) {
            return;
        }
        self.announced = Some(release.version.clone());
        let can_install = state.can_install();
        let handle = self.handle.clone();
        std::thread::spawn(move || {
            let mut n = notify_rust::Notification::new();
            n.appname("QuantumEngine")
                .summary("QuantumEngine update")
                .body(&format!("Version {} is available. You have {VERSION}.", release.version))
                .icon("quantumengine");
            if can_install {
                n.action("install", "Install");
            }
            n.action("view", "What's new");
            match n.show() {
                Ok(shown) => shown.wait_for_action(|action| match action {
                    "install" => handle.send(Command::Install(no_reply())),
                    "view" => open_url(&release.url),
                    _ => {}
                }),
                Err(e) => eprintln!("quantumengined: cannot show the update notification: {e}"),
            }
        });
    }

    fn install(&self, release: &Release) {
        self.set(|s| s.busy = true);
        match self.download_and_install(release) {
            Ok(()) => {
                self.set(|s| s.status = format!("Installed {}, restarting…", release.version));
                let err = restart();
                self.set(|s| {
                    s.busy = false;
                    s.status = format!("Installed {}, but couldn't restart: {err}. Log out and back in.", release.version);
                });
            }
            Err(e) => {
                eprintln!("quantumengined: update failed: {e}");
                self.set(|s| {
                    s.busy = false;
                    s.status = format!("Update failed: {e}");
                });
            }
        }
    }

    fn download_and_install(&self, release: &Release) -> Result<(), String> {
        let package = release.package.as_ref().ok_or("the release has no package for this system")?;
        let checksum = release.checksum.as_ref().ok_or("the release has no checksum file")?;
        let dir = cache_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let path = dir.join(&package.name);

        self.set(|s| s.status = format!("Downloading {}…", release.version));
        download(&package.url, &path)?;
        let expected = parse_checksum(&fetch_text(&checksum.url)?).ok_or("the release's checksum file is malformed")?;
        let actual = sha256(&path)?;
        if actual != expected {
            let _ = std::fs::remove_file(&path);
            return Err("the download doesn't match the release's checksum".into());
        }

        self.set(|s| s.status = "Waiting for your password, then installing…".into());
        // pacman's errors go to a file, not a pipe, and the wait is for pkexec
        // itself to exit. pacman's hooks can leave a process running that
        // inherited the pipe; reading a pipe to its end would then never finish,
        // though the install had.
        let mut pkexec = Process::new("pkexec");
        pkexec.args(["pacman", "-U", "--noconfirm"]).arg(&path);
        let ran = run_logged(pkexec, &dir.join("pacman.log"));
        let _ = std::fs::remove_file(&path);
        let (code, stderr) = ran.map_err(|e| format!("cannot run pkexec: {e}"))?;
        pkexec_outcome(code, &stderr)
    }
}

/// Runs `cmd` until it exits and returns its exit code and what it wrote to
/// stderr. Its stderr goes to `log` (then removed), never a pipe, so a process
/// it leaves behind holding that stream can't keep this waiting.
fn run_logged(mut cmd: Process, log: &Path) -> std::io::Result<(Option<i32>, String)> {
    let file = std::fs::File::create(log)?;
    let status = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(file).status();
    let stderr = std::fs::read_to_string(log).unwrap_or_default();
    let _ = std::fs::remove_file(log);
    Ok((status?.code(), stderr))
}

/// What pkexec's exit status means. 126 and 127 are pkexec's own (cancelled,
/// not authorised); anything else non-zero is pacman's, explained by the last
/// line it wrote.
fn pkexec_outcome(code: Option<i32>, stderr: &str) -> Result<(), String> {
    match code {
        Some(0) => Ok(()),
        Some(126) => Err("the password prompt was cancelled".into()),
        Some(127) => Err("not authorised to install packages".into()),
        _ => {
            let reason = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("unknown error");
            Err(format!("pacman failed: {}", reason.trim()))
        }
    }
}

/// Replaces this process with the newly installed binary. Only returns on
/// failure. Every descriptor is close-on-exec, so the bus name is released
/// for the new process, and the EQ filter stops with its parent thread.
fn restart() -> std::io::Error {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return e,
    };
    // After pacman replaces the file, the link reads "... (deleted)".
    let path = PathBuf::from(exe.to_string_lossy().trim_end_matches(" (deleted)").to_owned());
    Process::new(path).exec()
}

fn pacman_owned() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    Process::new("pacman")
        .args(["-Qqo"])
        .arg(&exe)
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == PACKAGE)
}

fn repo() -> &'static str {
    env!("CARGO_PKG_REPOSITORY").trim_start_matches("https://github.com/")
}

fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("quantumengine")
        .join("updates")
}

fn curl() -> Process {
    let mut cmd = Process::new("curl");
    cmd.args(["-sSL", "--max-time", "60", "--retry", "2", "-H"])
        .arg(format!("User-Agent: quantumengine/{VERSION}"))
        .stdin(Stdio::null());
    cmd
}

/// The latest published release, or `None` if the repository has none yet.
fn latest_release() -> Result<Option<Release>, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo());
    let out = curl()
        .args(["-H", "Accept: application/vnd.github+json", "-w", "\n%{http_code}"])
        .arg(&url)
        .output()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n').ok_or("empty reply from GitHub")?;
    match code.trim() {
        "200" => parse_release(body, std::env::consts::ARCH).map(Some),
        "404" => Ok(None),
        code => Err(format!("GitHub answered HTTP {code}")),
    }
}

fn parse_release(json: &str, arch: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("unexpected reply from GitHub: {e}"))?;
    let tag = v["tag_name"].as_str().ok_or("the release has no tag")?;
    let version = tag.trim_start_matches('v');
    let assets: Vec<Asset> = v["assets"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|a| {
                    Some(Asset { name: a["name"].as_str()?.to_owned(), url: a["browser_download_url"].as_str()?.to_owned() })
                })
                .collect()
        })
        .unwrap_or_default();
    // `quantumengine-0.3.0-1-x86_64.pkg.tar.zst`, but not a
    // `quantumengine-debug-…` package if one was ever attached.
    let prefix = format!("{PACKAGE}-{version}-");
    let suffix = format!("-{arch}.pkg.tar.zst");
    let package = assets.iter().find(|a| a.name.starts_with(&prefix) && a.name.ends_with(&suffix)).cloned();
    let checksum = package
        .as_ref()
        .and_then(|p| assets.iter().find(|a| a.name == format!("{}.sha256", p.name)).cloned());
    Ok(Release {
        version: version.to_owned(),
        url: v["html_url"].as_str().unwrap_or_default().to_owned(),
        package,
        checksum,
    })
}

fn fetch_text(url: &str) -> Result<String, String> {
    let out = curl().arg("-f").arg(url).output().map_err(|e| format!("cannot run curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("download failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn download(url: &str, to: &Path) -> Result<(), String> {
    let out = curl().arg("-f").arg("-o").arg(to).arg(url).output().map_err(|e| format!("cannot run curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("download failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(())
}

/// The hash from a `sha256sum`-style line.
fn parse_checksum(text: &str) -> Option<String> {
    let hash = text.split_whitespace().next()?.to_ascii_lowercase();
    (hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hash)
}

fn sha256(path: &Path) -> Result<String, String> {
    let out = Process::new("sha256sum").arg(path).output().map_err(|e| format!("cannot run sha256sum: {e}"))?;
    parse_checksum(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| "cannot checksum the download".into())
}

pub fn open_url(url: &str) {
    let _ = Process::new("xdg-open").arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

/// Parses `major.minor.patch`, ignoring a leading `v` and any `-suffix`.
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    matches!((parse_version(candidate), parse_version(current)), (Some(a), Some(b)) if a > b)
}

#[cfg(test)]
mod tests {
    /// The install hang: pacman had finished, but a process its hooks started
    /// still held the output pipe, and reading that pipe to its end never
    /// returned. `run_logged` returns as soon as the command itself exits.
    #[test]
    fn a_leftover_background_process_does_not_keep_the_install_waiting() {
        let dir = std::env::temp_dir().join(format!("qe-run-logged-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log");
        let mut cmd = super::Process::new("sh");
        cmd.args(["-c", "echo 'error: something broke' >&2; sleep 30 & exit 3"]);
        let started = std::time::Instant::now();
        let (code, stderr) = super::run_logged(cmd, &log).unwrap();
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "waited {:?}", started.elapsed());
        assert_eq!(code, Some(3));
        assert!(stderr.contains("something broke"));
        assert!(!log.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pkexec_exit_codes_say_what_happened() {
        assert_eq!(super::pkexec_outcome(Some(0), ""), Ok(()));
        assert!(super::pkexec_outcome(Some(126), "").unwrap_err().contains("cancelled"));
        assert!(super::pkexec_outcome(Some(127), "").unwrap_err().contains("not authorised"));
        assert_eq!(
            super::pkexec_outcome(Some(1), "loading packages...\nerror: failed to commit transaction\n\n"),
            Err("pacman failed: error: failed to commit transaction".into())
        );
    }

    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("v0.3.0", "0.2.1"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("v0.2.1", "0.2.1"));
        assert!(!is_newer("0.2.0", "0.2.1"));
        assert!(!is_newer("garbage", "0.2.1"));
        assert!(!is_newer("0.3", "0.2.1"));
        assert_eq!(parse_version("v1.2.3-rc1"), Some((1, 2, 3)));
    }

    #[test]
    fn release_picks_this_architectures_package_and_its_checksum() {
        let json = r#"{
            "tag_name": "v0.3.0",
            "html_url": "https://github.com/sidmax7/QuantumEngine/releases/tag/v0.3.0",
            "assets": [
                { "name": "PKGBUILD", "browser_download_url": "https://x/PKGBUILD" },
                { "name": "quantumengine-debug-0.3.0-1-x86_64.pkg.tar.zst", "browser_download_url": "https://x/debug" },
                { "name": "quantumengine-0.3.0-1-x86_64.pkg.tar.zst", "browser_download_url": "https://x/pkg" },
                { "name": "quantumengine-0.3.0-1-x86_64.pkg.tar.zst.sha256", "browser_download_url": "https://x/sum" }
            ]
        }"#;
        let r = parse_release(json, "x86_64").unwrap();
        assert_eq!(r.version, "0.3.0");
        assert_eq!(r.package.unwrap().url, "https://x/pkg");
        assert_eq!(r.checksum.unwrap().url, "https://x/sum");

        let other = parse_release(json, "aarch64").unwrap();
        assert!(other.package.is_none() && other.checksum.is_none());
    }

    #[test]
    fn checksum_lines_are_parsed_strictly() {
        let hash = "8796e64775b13d8f1571e8552404b1810ca40e9fa9ee715229418e505e58e325";
        assert_eq!(parse_checksum(&format!("{hash}  quantumengine.pkg.tar.zst\n")).as_deref(), Some(hash));
        assert_eq!(parse_checksum(&hash.to_uppercase()).as_deref(), Some(hash));
        assert!(parse_checksum("not-a-hash file").is_none());
        assert!(parse_checksum("").is_none());
    }

    #[test]
    fn install_needs_pacman_and_a_complete_release() {
        let asset = || Some(Asset { name: "p".into(), url: "u".into() });
        let release = Release { version: "9.9.9".into(), url: String::new(), package: asset(), checksum: asset() };
        let mut s = State { available: Some(release), status: String::new(), busy: false, checks_enabled: true, pacman_owned: true };
        assert!(s.can_install());
        s.pacman_owned = false;
        assert!(!s.can_install());
        s.pacman_owned = true;
        s.available.as_mut().unwrap().checksum = None;
        assert!(!s.can_install());
    }
}
