//! The equalizer: JBL's headset correction plus a sound profile, applied on
//! the PC by a PipeWire filter in front of the headset's game output. It never
//! talks to the headset itself.
//!
//! One thread owns the filter. Everything else reads [`State`] and sends
//! [`Command`]s, the same arrangement as the headset worker in `device`.

pub mod biquad;
pub mod pipewire;
pub mod profiles;

use crate::notifier::Notifier;
use crate::settings;
use biquad::{peak_db, response_db, Coefficients};
use pipewire::{Chain, Filter, HEADSET_GAME_OUTPUT, RATE, SLOTS};
use profiles::{Correction, Profile, PROFILES};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How often to check that the filter is still running while the EQ is on.
const WATCH_EVERY: Duration = Duration::from_secs(5);
/// How long to wait before restarting a filter that failed.
const RETRY_AFTER: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct State {
    pub enabled: bool,
    pub profile: &'static str,
    pub correction: Correction,
    /// Why the EQ isn't working, or empty when it is (or is off).
    pub status: String,
}

pub type Reply<T> = Sender<Result<T, String>>;

pub enum Command {
    Enable(bool, Reply<bool>),
    Profile(String, Reply<&'static str>),
    Correction(Correction, Reply<Correction>),
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
        // Only fails if the EQ thread has died, which it never does on purpose.
        let _ = self.commands.send(cmd);
    }

    /// Sends `make(reply)` and waits for the answer.
    pub fn call<T>(&self, make: impl FnOnce(Reply<T>) -> Command) -> Result<T, String> {
        let (reply, rx) = mpsc::channel();
        self.commands.send(make(reply)).map_err(|_| "the EQ thread has stopped".to_string())?;
        rx.recv().map_err(|_| "no reply from the EQ thread".to_string())?
    }
}

pub fn no_reply<T>() -> Reply<T> {
    mpsc::channel().0
}

/// Starts the EQ thread, applying the saved settings straight away.
pub fn spawn(notifier: Notifier) -> Handle {
    let saved = settings::load().eq;
    let profile = profiles::find(&saved.profile).unwrap_or(&PROFILES[0]);
    let state = Arc::new(Mutex::new(State {
        enabled: saved.enabled,
        profile: profile.name,
        correction: saved.correction,
        status: String::new(),
    }));
    let (commands, rx) = mpsc::channel();
    let worker = Worker { state: state.clone(), notifier, filter: None, retry_at: None };
    std::thread::Builder::new()
        .name("eq".into())
        .spawn(move || worker.run(rx))
        .expect("failed to start the EQ thread");
    Handle { state, commands }
}

/// The coefficients for `correction` under `profile`, turned down by however
/// much the combined curve boosts at its highest point so it can never clip.
pub fn chain(correction: Correction, profile: &Profile) -> Chain {
    let bands: Vec<_> = correction.bands().into_iter().chain(profile.bands()).collect();
    // The last slot is kept for the volume reduction.
    assert!(bands.len() < SLOTS, "{} bands do not fit the filter", bands.len());
    let mut chain = [Coefficients::IDENTITY; SLOTS];
    for (slot, band) in chain.iter_mut().zip(&bands) {
        // A 0 dB bell or shelf does nothing; keep it an exact pass-through.
        if band.gain_db != 0.0 {
            *slot = Coefficients::design(band, RATE);
        }
    }
    let headroom = peak_db(&chain, RATE);
    // Anything under a hundredth of a dB is rounding, not a boost.
    if headroom > 0.01 {
        chain[SLOTS - 1] = Coefficients::IDENTITY.scaled(10f64.powf(-headroom / 20.0));
    }
    chain
}

/// The shape of `correction` under `profile`, in dB, at each of `freqs`;
/// [`chain`] without the turn-down that keeps it from clipping.
pub fn response(correction: Correction, profile: &Profile, freqs: impl Iterator<Item = f64>) -> Vec<f64> {
    let filters: Vec<_> = correction
        .bands()
        .into_iter()
        .chain(profile.bands())
        .filter(|b| b.gain_db != 0.0)
        .map(|b| Coefficients::design(&b, RATE))
        .collect();
    freqs.map(|f| response_db(&filters, f, RATE)).collect()
}

struct Worker {
    state: Arc<Mutex<State>>,
    notifier: Notifier,
    filter: Option<Filter>,
    retry_at: Option<Instant>,
}

impl Worker {
    fn run(mut self, commands: Receiver<Command>) {
        self.apply();
        loop {
            match commands.recv_timeout(WATCH_EVERY) {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => self.watch(),
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::Enable(on, reply) => {
                let result = self.save(|s| s.eq.enabled = on).map(|()| {
                    self.state.lock().unwrap().enabled = on;
                    on
                });
                drop(reply.send(result));
            }
            Command::Profile(name, reply) => {
                let result = match profiles::find(&name) {
                    None => Err(format!("unknown EQ profile '{name}'")),
                    Some(p) => self.save(|s| s.eq.profile = p.name.into()).map(|()| {
                        self.state.lock().unwrap().profile = p.name;
                        p.name
                    }),
                };
                drop(reply.send(result));
            }
            Command::Correction(c, reply) => {
                let result = self.save(|s| s.eq.correction = c).map(|()| {
                    self.state.lock().unwrap().correction = c;
                    c
                });
                drop(reply.send(result));
            }
        }
        self.retry_at = None;
        self.apply();
    }

    fn save(&self, change: impl FnOnce(&mut settings::Settings)) -> Result<(), String> {
        settings::update(change).map(drop)
    }

    /// Restarts a filter that has died, e.g. because PipeWire restarted.
    fn watch(&mut self) {
        if !self.state.lock().unwrap().enabled {
            return;
        }
        if let Some(f) = &mut self.filter {
            if f.is_running() {
                return;
            }
            self.filter = None;
        }
        if self.retry_at.is_none_or(|t| Instant::now() >= t) {
            self.apply();
        }
    }

    /// Makes the filter match the current state.
    fn apply(&mut self) {
        let state = self.state.lock().unwrap().clone();
        let status = if state.enabled {
            let profile = profiles::find(state.profile).expect("state holds a known profile");
            let chain = chain(state.correction, profile);
            if self.filter.as_mut().is_some_and(|f| !f.is_running()) {
                self.filter = None;
            }
            let result = match &mut self.filter {
                Some(f) => f.set(&chain),
                None => Filter::start(&HEADSET_GAME_OUTPUT, &chain).map(|f| self.filter = Some(f)),
            };
            match result {
                Ok(()) => String::new(),
                Err(e) => {
                    self.filter = None;
                    self.retry_at = Some(Instant::now() + RETRY_AFTER);
                    eprintln!("quantumengined: EQ: {e}");
                    e
                }
            }
        } else {
            self.filter = None;
            String::new()
        };
        self.state.lock().unwrap().status = status;
        self.notifier.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_combination_fits_and_never_boosts_above_zero() {
        for c in Correction::ALL {
            for p in PROFILES {
                let chain = chain(c, p);
                let peak = peak_db(&chain, RATE);
                assert!(peak <= 0.01, "{} + {}: peak {peak} dB", c.as_str(), p.name);
            }
        }
    }

    #[test]
    fn flat_without_correction_is_a_pass_through() {
        let chain = chain(Correction::Off, &PROFILES[0]);
        assert!(chain.iter().all(|c| *c == Coefficients::IDENTITY));
    }

    #[test]
    fn the_drawn_curve_is_the_chain_without_the_turn_down() {
        let profile = profiles::find("Bass Boost").unwrap();
        let freqs = [50.0, 1_000.0, 12_000.0];
        let filters = chain(Correction::Stereo, profile);
        let headroom = -filters[SLOTS - 1].magnitude_db(1_000.0, RATE);
        for (f, db) in freqs.into_iter().zip(response(Correction::Stereo, profile, freqs.into_iter())) {
            assert!((db - (response_db(&filters, f, RATE) + headroom)).abs() < 1e-9, "{f} Hz");
        }
        assert!(response(Correction::Off, &PROFILES[0], freqs.into_iter()).iter().all(|&db| db == 0.0));
    }

    #[test]
    fn the_volume_reduction_matches_the_curves_peak() {
        // JBL's surround curve tops out about 6 dB up.
        let chain = chain(Correction::Surround, &PROFILES[0]);
        let reduction = chain[SLOTS - 1].magnitude_db(1_000.0, RATE);
        assert!((reduction + 6.0).abs() < 0.05, "turned down {reduction} dB");
    }
}
