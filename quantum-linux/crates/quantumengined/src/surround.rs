//! Virtual 7.1 surround: a WirePlumber smart filter with eight input
//! channels, each played through a binaural room impulse response (see
//! [`crate::brir`]: the MIT KEMAR head libmysofa ships, corrected in a small
//! simulated room), mixed down to stereo for the headset's game output.
//!
//! This is the Linux side of what QuantumENGINE does on Windows, where the
//! headphones are set to 7.1 in the sound settings and the PC renders the
//! channels binaurally. WirePlumber inserts this filter transparently before
//! the headset's Game output. The desktop keeps “JBL Quantum810 Wireless
//! Game” selected, so no extra selectable output is needed. Games that offer
//! 7.1 send eight channels through the filter; stereo stays in the front
//! speakers. The EQ runs after this filter, as it does on the headset.
//!
//! The graph follows the structure of Harman's HX3D virtualiser, as found by
//! static analysis of QuantumENGINE 2.8.6 (`reverse engineer/surroundsound/`):
//! the same 7.1 speaker angles, a 0.9 trim on multichannel input, bass below
//! 120 Hz kept out of the HRTF and sent to both ears, LFE added to both ears
//! at half level, and JBL's surround tuning after the binaural stage (the EQ
//! switches to it while surround is on). Only those numbers and that
//! structure are reused; the HRTF data is the open MIT KEMAR set.
//!
//! ```text
//! FL FR FC RL RR SL SR ─┬─ BRIR per ear ──── sum L/R ─ high-pass 120 Hz (LR4) ─┐
//!                       └─ sum ×0.45 ──────┐                                      ├─ L / R
//! LFE ─ HP 15 Hz ─ shelf ─ ×0.45 ──────────┴─ low-pass 120 Hz (LR4) ─ both ears ─┘
//! ```
//!
//! One thread owns the graph; everything else reads [`State`] and sends
//! [`Command`]s, as with the EQ.

use crate::brir;
use crate::eq::{self, pipewire::HEADSET_GAME_OUTPUT, profiles::Correction};
use crate::notifier::Notifier;
use crate::pwgraph::{self, quoted, Graph};
use crate::settings::{self, SurroundProfile};
use std::fmt::Write as _;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const NODE_NAME: &str = "quantumengine_surround";
const DESCRIPTION: &str = "JBL Quantum810 Wireless Game";
/// libmysofa's own copy of the MIT KEMAR measurements (BSD-3-Clause);
/// `pipewire-audio` depends on libmysofa, so it's there wherever PipeWire's
/// audio stack is.
const HRTF: &str = "/usr/share/libmysofa/MIT_KEMAR_normal_pinna.sofa";
const RATE: u32 = 48_000;
const WATCH_EVERY: Duration = Duration::from_secs(5);
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// The seven directional 7.1 channels in PipeWire's order, with where each
/// speaker sits: degrees anticlockwise from straight ahead, as SOFA files
/// measure them (so 30 is front left). These are the angles of
/// QuantumENGINE's "Dolby 7.1" layout, which are also ITU-R BS.775's. LFE
/// has no direction and takes the bass path instead.
const SPEAKERS: [(&str, f64); 7] = [
    ("FL", 30.0),
    ("FR", 330.0),
    ("FC", 0.0),
    ("RL", 150.0),
    ("RR", 210.0),
    ("SL", 90.0),
    ("SR", 270.0),
];
/// Every channel PipeWire sends, in order: the speakers with LFE fourth.
const POSITIONS: &str = "FL FR FC LFE RL RR SL SR";
const TEST_CHANNEL_MAP: &str = "FL,FR,FC,LFE,RL,RR,SL,SR";
/// HX3D trims 5.1 and 7.1 input to 0.9 before rendering.
const TRIM: f64 = 0.9;
/// Below this, sound skips the HRTF and goes to both ears (HX3D's
/// `XOverFc`): KEMAR's low end adds nothing but colouring, and bass has no
/// direction you could hear anyway.
const CROSSOVER_HZ: f64 = 120.0;
/// Each channel's share of the summed bass, and LFE's, both as HX3D adds
/// LFE: half level to each ear (then trimmed).
const BASS_GAIN: f64 = 0.5 * TRIM;
/// Butterworth Q; two in a row make a Linkwitz-Riley crossover, whose low
/// and high halves add back up flat.
const BUTTERWORTH_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;
/// Convolution partition sizes, set rather than left to defaults: a short
/// first block keeps per-cycle work small (this is what stopped the
/// crackling), and the 12288-tap responses take three tail blocks.
const BLOCKSIZE: u32 = 256;
const TAILSIZE: u32 = 4096;
/// The buffer the surround sink asks for. Bigger than PipeWire's minimum, so
/// every cycle has room for 7 convolutions; 21 ms of latency.
const LATENCY: &str = "1024/48000";

#[derive(Clone, Debug)]
pub struct State {
    pub enabled: bool,
    pub output_gain: u16,
    pub profile: String,
    pub profiles: std::collections::BTreeMap<String, SurroundProfile>,
    /// Why surround isn't working, or empty when it is (or is off).
    pub status: String,
}

pub type Reply<T> = Sender<Result<T, String>>;

pub enum Command {
    Enable(bool, Reply<bool>),
    SetOutputGain(u16, Reply<u16>),
    TestSpeaker(String, Reply<()>),
    SelectProfile(String, Reply<String>),
    SetProfile(String, SurroundProfile, Reply<()>),
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
        self.commands.send(make(reply)).map_err(|_| "the surround thread has stopped".to_string())?;
        rx.recv().map_err(|_| "no reply from the surround thread".to_string())?
    }
}

pub fn no_reply<T>() -> Reply<T> {
    mpsc::channel().0
}

/// Starts the surround thread, applying the saved setting straight away.
/// `eq` is told which JBL tuning goes with surround on or off.
pub fn spawn(notifier: Notifier, eq: eq::Handle) -> Handle {
    let saved = settings::load().surround;
    let state = Arc::new(Mutex::new(State {
        enabled: saved.enabled,
        output_gain: saved.output_gain,
        profile: saved.profile,
        profiles: saved.profiles,
        status: String::new(),
    }));
    let (commands, rx) = mpsc::channel();
    let worker = Worker {
        state: state.clone(), notifier, eq, graph: None, graph_name: None, generation: 0, target: None, brir: None, retry_at: None, speaker_player: None,
    };
    std::thread::Builder::new()
        .name("surround".into())
        .spawn(move || worker.run(rx))
        .expect("failed to start the surround thread");
    Handle { state, commands }
}

struct Worker {
    state: Arc<Mutex<State>>,
    notifier: Notifier,
    eq: eq::Handle,
    graph: Option<Graph>,
    graph_name: Option<String>,
    generation: u64,
    /// The headset sink the running graph plays into.
    target: Option<String>,
    /// The impulse-response file, built once per run of the service.
    brir: Option<PathBuf>,
    retry_at: Option<Instant>,
    speaker_player: Option<Child>,
}

impl Worker {
    fn run(mut self, commands: Receiver<Command>) {
        self.apply(false);
        loop {
            match commands.recv_timeout(WATCH_EVERY) {
                Ok(Command::Enable(on, reply)) => {
                    let result = settings::update(|s| s.surround.enabled = on).map(|_| {
                        self.state.lock().unwrap().enabled = on;
                        on
                    });
                    let changed = result.is_ok();
                    drop(reply.send(result));
                    if changed {
                        self.retry_at = None;
                        self.apply(true);
                        self.match_tuning(on);
                    }
                }
                Ok(Command::SetOutputGain(percent, reply)) => {
                    let result = if !(25..=300).contains(&percent) {
                        Err("output gain must be between 25 and 300%".to_string())
                    } else {
                        settings::update(|s| s.surround.output_gain = percent).map(|_| {
                            self.state.lock().unwrap().output_gain = percent;
                            self.eq.send(eq::Command::OutputGain(percent));
                            self.notifier.notify();
                            percent
                        })
                    };
                    drop(reply.send(result));
                }
                Ok(Command::TestSpeaker(speaker, reply)) => {
                    let result = self.play_speaker_test(&speaker);
                    drop(reply.send(result));
                }
                Ok(Command::SelectProfile(name, reply)) => {
                    let exists = self.state.lock().unwrap().profiles.contains_key(&name);
                    let result = if !exists {
                        Err(format!("unknown surround profile '{name}'"))
                    } else {
                        settings::update(|s| s.surround.profile = name.clone()).map(|_| {
                            self.state.lock().unwrap().profile = name.clone();
                            name
                        })
                    };
                    let changed = result.is_ok();
                    if changed { self.rebuild(); }
                    drop(reply.send(result));
                }
                Ok(Command::SetProfile(name, profile, reply)) => {
                    let exists = self.state.lock().unwrap().profiles.contains_key(&name);
                    let result = if !exists {
                        Err(format!("unknown surround profile '{name}'"))
                    } else { check_profile(&profile).and_then(|()| {
                        settings::update(|s| {
                            s.surround.profiles.insert(name.clone(), profile.clone());
                        }).map(|_| {
                            self.state.lock().unwrap().profiles.insert(name.clone(), profile);
                        })
                    }) };
                    let changed = result.is_ok();
                    if changed {
                        if self.state.lock().unwrap().profile == name { self.rebuild(); }
                        else { self.notifier.notify(); }
                    }
                    drop(reply.send(result));
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.cleanup_speaker_test();
                    self.watch();
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn play_speaker_test(&mut self, speaker: &str) -> Result<(), String> {
        if !["FL", "FR", "FC", "LFE", "RL", "RR", "SL", "SR"].contains(&speaker) {
            return Err(format!("unknown 7.1 speaker '{speaker}'"));
        }
        let node_name = self.graph_name.as_deref().unwrap_or(NODE_NAME);
        if !self.state.lock().unwrap().enabled || pwgraph::node_id(node_name)?.is_none() {
            return Err("Turn on Virtual surround 7.1 before testing its speakers.".into());
        }

        self.stop_speaker_test();
        let target = self.target.as_deref().ok_or("the headset Game output is unavailable")?;
        let dir = pwgraph::runtime_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let wav = dir.join(format!("{NODE_NAME}_test_{speaker}.wav"));
        write_speaker_test(&wav, speaker)?;

        let mut command = ProcessCommand::new("pw-play");
        command
            .args([
                "--target", target, "--format", "s16", "--rate", "48000", "--channels", "8", "--channel-map",
                TEST_CHANNEL_MAP, "--volume", "0.5",
            ])
            .arg(&wav)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // Stop the short test promptly if the service exits unexpectedly.
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
        self.speaker_player = Some(command.spawn().map_err(|e| format!("cannot start pw-play: {e}"))?);
        Ok(())
    }

    fn stop_speaker_test(&mut self) {
        if let Some(mut child) = self.speaker_player.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn cleanup_speaker_test(&mut self) {
        if self.speaker_player.as_mut().is_some_and(|child| child.try_wait().ok().flatten().is_some()) {
            self.speaker_player = None;
        }
    }

    fn rebuild(&mut self) {
        self.brir = None;
        self.retry_at = None;
        if self.graph.is_some() {
            let enabled = self.state.lock().unwrap().enabled;
            let result = self.replace_graph(enabled);
            if let Err(e) = &result { eprintln!("quantumengined: surround: {e}"); }
            self.state.lock().unwrap().status = result.err().unwrap_or_default();
            self.notifier.notify();
        } else {
            self.notifier.notify();
        }
    }

    /// Swaps JBL's stereo tuning for its surround tuning and back, as HX3D
    /// does: the surround curve is the driver EQ it puts after the binaural
    /// stage, the stereo one what it uses with 3D off. A tuning the user set
    /// to off stays off.
    fn match_tuning(&self, surround: bool) {
        let (from, to) = if surround {
            (Correction::Stereo, Correction::Surround)
        } else {
            (Correction::Surround, Correction::Stereo)
        };
        if self.eq.state().correction == from {
            self.eq.send(eq::Command::Correction(to, eq::no_reply()));
        }
    }

    /// Follows the headset's dongle coming and going, and restarts a graph
    /// that died (PipeWire restarted, say).
    fn watch(&mut self) {
        if !self.state.lock().unwrap().enabled {
            return;
        }
        let target = headset_sink().ok().flatten();
        // PipeWire removes the sink without stopping the process when its
        // target vanishes, so the node is checked as well as the process.
        let healthy = self.graph.as_mut().is_some_and(Graph::is_running)
            && target.is_some()
            && target == self.target
            && self.graph_name.as_deref().and_then(|name| pwgraph::node_id(name).ok().flatten()).is_some();
        if healthy {
            return;
        }
        if self.retry_at.is_none_or(|t| Instant::now() >= t) || target != self.target {
            self.apply(false);
        }
    }

    /// Makes the graph match the state. `switched` is true when the user
    /// just turned surround on or off.
    fn apply(&mut self, switched: bool) {
        let enabled = self.state.lock().unwrap().enabled;
        let status = if enabled { self.start(switched) } else { self.stop() };
        if let Err(e) = &status {
            eprintln!("quantumengined: surround: {e}");
        }
        self.state.lock().unwrap().status = status.err().unwrap_or_default();
        self.notifier.notify();
    }

    fn start(&mut self, make_default: bool) -> Result<(), String> {
        let target = match headset_sink()? {
            Some(t) => t,
            None => {
                // Nothing to play into, and a sink with nowhere to go would
                // just be a silent default. It starts when the dongle is back.
                self.graph = None;
                self.target = None;
                return Err("Waiting for the headset's dongle.".into());
            }
        };
        if !Path::new(HRTF).is_file() {
            return Err(format!("Needs libmysofa's HRTF data ({HRTF}); install libmysofa."));
        }
        let running = self.graph.as_mut().is_some_and(Graph::is_running)
            && self.graph_name.as_deref().is_some_and(|name| pwgraph::node_id(name).ok().flatten().is_some());
        if !running || self.target.as_deref() != Some(&target) {
            self.target = Some(target.clone());
            self.replace_graph(true).inspect_err(|_| self.retry_at = Some(Instant::now() + RETRY_AFTER))?;
        } else if let Some(name) = self.graph_name.as_deref() {
            pwgraph::set_smart_filter_enabled(name, true)?;
        }
        // Enabling surround selects the headset Game output, which remains
        // the visible/default device while WirePlumber inserts the filter.
        // Also migrate a remembered virtual-sink selection from older builds.
        if make_default || pwgraph::default_sink()?.as_deref() == Some(NODE_NAME) {
            let id = pwgraph::node_id(&target)?.ok_or("the headset Game output disappeared")?;
            pwgraph::set_default(id)?;
        }
        Ok(())
    }

    /// Builds the BRIR file the first time surround starts.
    fn brir_file(&mut self) -> Result<PathBuf, String> {
        if let Some(path) = self.brir.as_ref().filter(|p| p.is_file()) {
            return Ok(path.clone());
        }
        let azimuths: Vec<f64> = SPEAKERS.iter().map(|(_, az)| *az).collect();
        let profile = {
            let state = self.state.lock().unwrap();
            state.profiles.get(&state.profile).cloned().unwrap_or_default()
        };
        let brirs = brir::build(Path::new(HRTF), &azimuths, &profile)?;
        let dir = pwgraph::runtime_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        let path = dir.join(format!("{NODE_NAME}_brir_{}.wav", self.generation));
        brir::write_wav(&path, &brirs)?;
        self.brir = Some(path.clone());
        Ok(path)
    }

    fn stop(&mut self) -> Result<(), String> {
        if let Some(name) = self.graph_name.as_deref() {
            pwgraph::set_smart_filter_enabled(name, false)?;
        }
        let was_default = pwgraph::default_sink()?.as_deref() == Some(NODE_NAME);
        if was_default {
            // Clean up a remembered default left by an older QuantumEngine.
            if let Some((id, _)) = pwgraph::find_node(|p| HEADSET_GAME_OUTPUT.matches(p) && p["media.class"] == "Audio/Sink")? {
                pwgraph::set_default(id)?;
            }
        }
        Ok(())
    }

    /// Bring up a disabled replacement before switching WirePlumber over.
    /// This keeps the client's playback stream alive while its links move.
    fn replace_graph(&mut self, activate: bool) -> Result<(), String> {
        let target = self.target.clone().or(headset_sink()?).ok_or("the headset Game output is unavailable")?;
        let old_name = self.graph_name.clone();
        self.generation = self.generation.wrapping_add(1);
        let name = format!("{NODE_NAME}_{}", self.generation);
        let brir = self.brir_file()?;
        let graph = Graph::start(&name, "the replacement PipeWire surround filter", &config(&name, &target, &brir, true))?;
        // New filter is ready but disabled. Unlink old first, then link new;
        // WirePlumber transparently re-routes active streams to the headset.
        if let Some(old) = old_name.as_deref() {
            pwgraph::set_smart_filter_enabled(old, false)?;
        }
        if activate {
            if let Err(error) = pwgraph::set_smart_filter_enabled(&name, true) {
                if let Some(old) = old_name.as_deref() {
                    let _ = pwgraph::set_smart_filter_enabled(old, true);
                }
                return Err(error);
            }
        }
        let old_graph = self.graph.replace(graph);
        self.graph_name = Some(name);
        self.target = Some(target);
        drop(old_graph);
        self.brir = None;
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop_speaker_test();
    }
}

/// Writes a short two-pulse, low-level test signal to one channel of an
/// eight-channel PCM WAV. The LFE test uses a bass tone; other speakers use a
/// quiet harmonic tone with enough upper-frequency content for HRTF cues.
fn write_speaker_test(path: &Path, speaker: &str) -> Result<(), String> {
    let channel = POSITIONS.split_whitespace().position(|p| p == speaker).ok_or("invalid speaker channel")?;
    const SAMPLE_RATE: u32 = 48_000;
    const CHANNELS: u16 = 8;
    const FRAMES: usize = SAMPLE_RATE as usize * 9 / 10;
    const BITS: u16 = 16;
    const DATA_LEN: u32 = (FRAMES * CHANNELS as usize * (BITS as usize / 8)) as u32;
    let mut wav = Vec::with_capacity(DATA_LEN as usize + 44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + DATA_LEN).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&CHANNELS.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * CHANNELS as u32 * 2).to_le_bytes());
    wav.extend_from_slice(&(CHANNELS * 2).to_le_bytes());
    wav.extend_from_slice(&BITS.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&DATA_LEN.to_le_bytes());

    for frame in 0..FRAMES {
        let time = frame as f32 / SAMPLE_RATE as f32;
        let pulse_time = if time < 0.35 { Some(time) } else if (0.5..0.85).contains(&time) { Some(time - 0.5) } else { None };
        let sample = pulse_time.map_or(0.0, |t| {
            let fade = (t / 0.04).min((0.35 - t) / 0.04).clamp(0.0, 1.0);
            let tone = if speaker == "LFE" {
                (std::f32::consts::TAU * 65.0 * t).sin()
            } else {
                [440.0, 880.0, 1760.0, 3520.0, 5280.0]
                    .iter()
                    .enumerate()
                    .map(|(i, f)| (std::f32::consts::TAU * f * t).sin() / (i as f32 + 1.0))
                    .sum::<f32>()
                    / 2.3
            };
            tone * fade * 0.16
        });
        for output_channel in 0..CHANNELS as usize {
            let value = if output_channel == channel { (sample * i16::MAX as f32) as i16 } else { 0 };
            wav.extend_from_slice(&value.to_le_bytes());
        }
    }
    std::fs::write(path, wav).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn check_profile(profile: &SurroundProfile) -> Result<(), String> {
    if !(70..=150).contains(&profile.spatial_cues) {
        return Err("spatial cues must be between 70 and 150%".into());
    }
    if profile.reflections > 70 {
        return Err("early reflections must be between 0 and 70%".into());
    }
    if profile.ambience > 30 {
        return Err("room ambience must be between 0 and 30%".into());
    }
    if !(100..=700).contains(&profile.decay_ms) {
        return Err("reverb decay must be between 100 and 700 ms".into());
    }
    Ok(())
}

/// The node name of the headset's game output, if its dongle is plugged in.
fn headset_sink() -> Result<Option<String>, String> {
    Ok(pwgraph::find_node(|p| HEADSET_GAME_OUTPUT.matches(p) && p["media.class"] == "Audio/Sink")?.map(|(_, name)| name))
}

fn config(node_name: &str, target: &str, brir: &Path, initially_disabled: bool) -> String {
    let brir = quoted(&brir.to_string_lossy());
    let mut nodes = String::new();
    let mut links = String::new();
    let mut spatial_gains = String::new();
    let mut bass_gains = String::new();
    let mut node = |line: String| {
        let _ = writeln!(nodes, "          {line}");
    };
    let mut link = |from: &str, to: &str| {
        let _ = writeln!(links, "          {{ output = \"{from}\" input = \"{to}\" }}");
    };

    for (i, (channel, _)) in SPEAKERS.iter().enumerate() {
        let n = i + 1;
        // One input feeds three places, both ears' convolvers and the bass
        // sum, so it goes through a copy node first.
        node(format!("{{ type = builtin label = copy name = in{channel} }}"));
        // One convolver per ear; the WAV holds left then right for each
        // speaker, in SPEAKERS order.
        for (ear, side) in ["L", "R"].iter().enumerate() {
            let wav_channel = 2 * i + ear;
            node(format!(
                "{{ type = builtin label = convolver name = cv{channel}{side} config = {{ filename = {brir} channel = {wav_channel} blocksize = {BLOCKSIZE} tailsize = {TAILSIZE} }} }}"
            ));
            link(&format!("in{channel}:Out"), &format!("cv{channel}{side}:In"));
            link(&format!("cv{channel}{side}:Out"), &format!("hrtf{side}:In {n}"));
        }
        link(&format!("in{channel}:Out"), &format!("bass:In {n}"));
        let _ = write!(spatial_gains, " \"Gain {n}\" = {TRIM:.2}");
        let _ = write!(bass_gains, " \"Gain {n}\" = {BASS_GAIN:.2}");
    }
    let lfe = SPEAKERS.len() + 1;
    let _ = write!(bass_gains, " \"Gain {lfe}\" = {BASS_GAIN:.2}");

    node(format!("{{ type = builtin label = mixer name = hrtfL control = {{{spatial_gains} }} }}"));
    node(format!("{{ type = builtin label = mixer name = hrtfR control = {{{spatial_gains} }} }}"));
    // HX3D's LFE channel EQ: a 15 Hz high-pass and a gentle shelf.
    node(format!("{{ type = builtin label = bq_highpass name = lfeHp control = {{ \"Freq\" = 15.0 \"Q\" = {BUTTERWORTH_Q:.4} }} }}"));
    node("{ type = builtin label = bq_lowshelf name = lfeShelf control = { \"Freq\" = 500.0 \"Q\" = 0.7071 \"Gain\" = -2.0 } }".into());
    node(format!("{{ type = builtin label = mixer name = bass control = {{{bass_gains} }} }}"));
    link("lfeHp:Out", "lfeShelf:In");
    link("lfeShelf:Out", &format!("bass:In {lfe}"));

    // The crossover: the binaural sum loses its lows, the bass sum its highs.
    let bq = |label: &str, name: &str| {
        format!("{{ type = builtin label = {label} name = {name} control = {{ \"Freq\" = {CROSSOVER_HZ:.1} \"Q\" = {BUTTERWORTH_Q:.4} }} }}")
    };
    for side in ["L", "R"] {
        node(bq("bq_highpass", &format!("hp{side}1")));
        node(bq("bq_highpass", &format!("hp{side}2")));
        link(&format!("hrtf{side}:Out"), &format!("hp{side}1:In"));
        link(&format!("hp{side}1:Out"), &format!("hp{side}2:In"));
        node(format!("{{ type = builtin label = mixer name = out{side} }}"));
        link(&format!("hp{side}2:Out"), &format!("out{side}:In 1"));
        link("lp2:Out", &format!("out{side}:In 2"));
    }
    node(bq("bq_lowpass", "lp1"));
    node(bq("bq_lowpass", "lp2"));
    link("bass:Out", "lp1:In");
    link("lp1:Out", "lp2:In");

    let inputs: Vec<String> = POSITIONS
        .split(' ')
        .map(|c| if c == "LFE" { "\"lfeHp:In\"".to_string() } else { format!("\"in{c}:In\"") })
        .collect();
    let inputs = inputs.join(" ");
    let name = quoted(node_name);
    let output_name = quoted(&format!("{node_name}.output"));
    let description = quoted(DESCRIPTION);
    let disabled = if initially_disabled { "true" } else { "false" };
    let target = quoted(target);
    format!(
        r#"# Written by quantumengined; regenerated every time surround starts.
context.properties = {{ log.level = 0 }}
context.spa-libs = {{
  audio.convert.* = audioconvert/libspa-audioconvert
  support.* = support/libspa-support
}}
context.modules = [
  {{ name = libpipewire-module-rt args = {{ }} flags = [ ifexists nofail ] }}
  {{ name = libpipewire-module-protocol-native }}
  {{ name = libpipewire-module-client-node }}
  {{ name = libpipewire-module-adapter }}
  {{ name = libpipewire-module-filter-chain
    args = {{
      node.description = {description}
      media.name = {description}
      filter.graph = {{
        nodes = [
{nodes}        ]
        links = [
{links}        ]
        inputs = [ {inputs} ]
        outputs = [ "outL:Out" "outR:Out" ]
      }}
      capture.props = {{
        node.name = {name}
        media.class = Audio/Sink
        audio.rate = {RATE}
        audio.channels = 8
        audio.position = [ {POSITIONS} ]
        filter.smart = true
        filter.smart.name = {name}
        filter.smart.disabled = {disabled}
        filter.smart.targetable = false
        filter.smart.target = {{ node.name = {target} }}
        filter.smart.before = [ "quantumengine_eq" ]
        node.latency = {LATENCY}
        # Stereo stays in the two front speakers; PipeWire's default upmix
        # would spread diffuse copies into the sides and rear.
        channelmix.upmix = false
      }}
      playback.props = {{
        node.name = {output_name}
        node.passive = true
        node.latency = {LATENCY}
        audio.rate = {RATE}
        audio.channels = 2
        audio.position = [ FL FR ]
        stream.dont-remix = true
      }}
    }}
  }}
]
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_renders_seven_speakers_and_keeps_bass_out_of_the_hrtf() {
        let conf = config("test_surround", "some_headset", Path::new("/run/brir.wav"), false);
        assert_eq!(conf.matches("label = convolver").count(), 14);
        assert!(conf.contains("audio.position = [ FL FR FC LFE RL RR SL SR ]"));
        assert!(conf.contains("inputs = [ \"inFL:In\" \"inFR:In\" \"inFC:In\" \"lfeHp:In\" \"inRL:In\""));
        // SL is the sixth speaker: WAV channels 10 (left ear) and 11 (right).
        assert!(conf.contains(
            "name = cvSLR config = { filename = \"/run/brir.wav\" channel = 11 blocksize = 256 tailsize = 4096 }"
        ));
        // Every speaker feeds both the HRTF and the bass sum; LFE only the bass.
        assert_eq!(conf.matches("input = \"bass:In ").count(), 8);
        assert_eq!(conf.matches("input = \"hrtfL:In ").count(), 7);
        assert!(conf.contains("{ output = \"lfeShelf:Out\" input = \"bass:In 8\" }"));
        // Both ears get the high-passed binaural sum and the shared bass.
        assert!(conf.contains("{ output = \"lp2:Out\" input = \"outL:In 2\" }"));
        assert!(conf.contains("{ output = \"lp2:Out\" input = \"outR:In 2\" }"));
        assert_eq!(conf.matches("label = bq_highpass name = hp").count(), 4);
        assert!(conf.contains("\"Gain 1\" = 0.90"));
        assert!(conf.contains("\"Gain 8\" = 0.45"));
        assert!(conf.contains("channelmix.upmix = false"));
        assert!(conf.contains("filter.smart.target = { node.name = \"some_headset\" }"));
        assert!(conf.contains("filter.smart.targetable = false"));
        assert!(conf.contains("filter.smart.before = [ \"quantumengine_eq\" ]"));
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Run with `cargo test -- --ignored`. It targets no real output, so
    /// nothing is routed through the test filter.
    #[test]
    #[ignore = "starts a real PipeWire filter in the current session"]
    fn graph_starts_with_eight_channels_and_goes_away() {
        let name = "quantumengine_surround_test";
        let target = "quantumengine_surround_test_no_target";
        let azimuths: Vec<f64> = SPEAKERS.iter().map(|(_, az)| *az).collect();
        let wav = pwgraph::runtime_dir().join("quantumengine_surround_test_brir.wav");
        brir::write_wav(&wav, &brir::build(Path::new(HRTF), &azimuths, &SurroundProfile::default()).unwrap()).unwrap();
        let graph = Graph::start(name, "the test surround filter", &config(name, &target, &wav, false)).unwrap();
        let found = pwgraph::find_node(|p| p["node.name"] == name && p["audio.channels"] == 8).unwrap();
        assert!(found.is_some(), "an 8-channel sink called {name}");
        drop(graph);
        std::thread::sleep(Duration::from_millis(500));
        assert!(pwgraph::node_id(name).unwrap().is_none());
    }
}
