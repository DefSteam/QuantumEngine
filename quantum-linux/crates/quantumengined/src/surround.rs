//! Virtual 7.1 surround: a PipeWire sink with eight channels, each placed
//! around the listener with a measured HRTF (the MIT KEMAR dummy head that
//! libmysofa ships), mixed down to stereo for the headset's game output.
//!
//! This is the Linux side of what QuantumENGINE does on Windows, where the
//! headphones are set to 7.1 in the sound settings and the PC renders the
//! channels binaurally. Here the sink is the 7.1 device: games that offer
//! 7.1 send it eight channels, and stereo plays from two virtual speakers in
//! front. Switching it on makes it the default output; switching it off puts
//! the headset back. Its output goes to the headset's game output like any
//! app, so the EQ still applies on top.
//!
//! One thread owns the graph; everything else reads [`State`] and sends
//! [`Command`]s, as with the EQ.

use crate::eq::pipewire::HEADSET_GAME_OUTPUT;
use crate::notifier::Notifier;
use crate::pwgraph::{self, quoted, Graph};
use crate::settings;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const NODE_NAME: &str = "quantumengine_surround";
const DESCRIPTION: &str = "QuantumEngine Surround 7.1";
/// libmysofa's own copy of the MIT KEMAR measurements (BSD-3-Clause);
/// `pipewire-audio` depends on libmysofa, so it's there wherever the
/// spatializer plugin is.
const HRTF: &str = "/usr/share/libmysofa/MIT_KEMAR_normal_pinna.sofa";
const RATE: u32 = 48_000;
const WATCH_EVERY: Duration = Duration::from_secs(5);
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// The 7.1 channels in PipeWire's order, each with where its speaker sits:
/// degrees anticlockwise from straight ahead, as SOFA files measure them
/// (so 30 is front left). The angles are ITU-R BS.775's, the standard 7.1
/// layout. LFE has no direction; it plays from the front, quieter.
const SPEAKERS: [(&str, f64, f64); 8] = [
    ("FL", 30.0, 1.0),
    ("FR", 330.0, 1.0),
    ("FC", 0.0, 1.0),
    ("LFE", 0.0, 0.5),
    ("RL", 150.0, 1.0),
    ("RR", 210.0, 1.0),
    ("SL", 90.0, 1.0),
    ("SR", 270.0, 1.0),
];

#[derive(Clone, Debug)]
pub struct State {
    pub enabled: bool,
    /// Why surround isn't working, or empty when it is (or is off).
    pub status: String,
}

pub type Reply<T> = Sender<Result<T, String>>;

pub enum Command {
    Enable(bool, Reply<bool>),
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
pub fn spawn(notifier: Notifier) -> Handle {
    let state = Arc::new(Mutex::new(State { enabled: settings::load().surround.enabled, status: String::new() }));
    let (commands, rx) = mpsc::channel();
    let worker = Worker { state: state.clone(), notifier, graph: None, target: None, retry_at: None };
    std::thread::Builder::new()
        .name("surround".into())
        .spawn(move || worker.run(rx))
        .expect("failed to start the surround thread");
    Handle { state, commands }
}

struct Worker {
    state: Arc<Mutex<State>>,
    notifier: Notifier,
    graph: Option<Graph>,
    /// The headset sink the running graph plays into.
    target: Option<String>,
    retry_at: Option<Instant>,
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
                    }
                }
                Err(RecvTimeoutError::Timeout) => self.watch(),
                Err(RecvTimeoutError::Disconnected) => return,
            }
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
            && pwgraph::node_id(NODE_NAME).ok().flatten().is_some();
        if healthy {
            return;
        }
        if self.retry_at.is_none_or(|t| Instant::now() >= t) || target != self.target {
            self.apply(false);
        }
    }

    /// Makes the graph match the state. `switched` is true when the user
    /// just turned surround on or off, which also moves the default output.
    fn apply(&mut self, switched: bool) {
        let enabled = self.state.lock().unwrap().enabled;
        let status = if enabled { self.start(switched) } else { self.stop(switched) };
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
        let running = self.graph.as_mut().is_some_and(Graph::is_running) && pwgraph::node_id(NODE_NAME)?.is_some();
        if !running || self.target.as_deref() != Some(&target) {
            self.graph = None;
            let graph = Graph::start(NODE_NAME, "the PipeWire surround filter", &config(NODE_NAME, &target))
                .inspect_err(|_| self.retry_at = Some(Instant::now() + RETRY_AFTER))?;
            self.graph = Some(graph);
            self.target = Some(target);
        }
        if make_default {
            let id = pwgraph::node_id(NODE_NAME)?.ok_or("the surround output disappeared")?;
            pwgraph::set_default(id)?;
        }
        Ok(())
    }

    fn stop(&mut self, restore_default: bool) -> Result<(), String> {
        let was_default = restore_default && pwgraph::default_sink()?.as_deref() == Some(NODE_NAME);
        self.graph = None;
        self.target = None;
        if was_default {
            // WirePlumber would pick some output on its own once ours is
            // gone; the headset is the one that was swapped out for it.
            if let Some((id, _)) = pwgraph::find_node(|p| HEADSET_GAME_OUTPUT.matches(p) && p["media.class"] == "Audio/Sink")? {
                pwgraph::set_default(id)?;
            }
        }
        Ok(())
    }
}

/// The node name of the headset's game output, if its dongle is plugged in.
fn headset_sink() -> Result<Option<String>, String> {
    Ok(pwgraph::find_node(|p| HEADSET_GAME_OUTPUT.matches(p) && p["media.class"] == "Audio/Sink")?.map(|(_, name)| name))
}

fn config(node_name: &str, target: &str) -> String {
    let hrtf = quoted(HRTF);
    let mut nodes = String::new();
    let mut links = String::new();
    let mut inputs = String::new();
    let mut gains = (String::new(), String::new());
    for (i, (channel, azimuth, gain)) in SPEAKERS.iter().enumerate() {
        let n = i + 1;
        let _ = writeln!(
            nodes,
            "          {{ type = sofa label = spatializer name = sp{channel} config = {{ filename = {hrtf} }} control = {{ \"Azimuth\" = {azimuth:.1} \"Elevation\" = 0.0 \"Radius\" = 1.0 }} }}"
        );
        let _ = writeln!(links, "          {{ output = \"sp{channel}:Out L\" input = \"mixL:In {n}\" }}");
        let _ = writeln!(links, "          {{ output = \"sp{channel}:Out R\" input = \"mixR:In {n}\" }}");
        let _ = write!(inputs, " \"sp{channel}:In\"");
        let _ = write!(gains.0, " \"Gain {n}\" = {gain:.2}");
        let _ = write!(gains.1, " \"Gain {n}\" = {gain:.2}");
    }
    let positions: Vec<&str> = SPEAKERS.iter().map(|(c, ..)| *c).collect();
    let positions = positions.join(" ");
    let (gains_l, gains_r) = gains;
    let name = quoted(node_name);
    let output_name = quoted(&format!("{node_name}.output"));
    let description = quoted(DESCRIPTION);
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
{nodes}          {{ type = builtin label = mixer name = mixL control = {{{gains_l} }} }}
          {{ type = builtin label = mixer name = mixR control = {{{gains_r} }} }}
        ]
        links = [
{links}        ]
        inputs = [{inputs} ]
        outputs = [ "mixL:Out" "mixR:Out" ]
      }}
      capture.props = {{
        node.name = {name}
        media.class = Audio/Sink
        audio.rate = {RATE}
        audio.channels = 8
        audio.position = [ {positions} ]
      }}
      playback.props = {{
        node.name = {output_name}
        node.passive = true
        audio.rate = {RATE}
        audio.channels = 2
        audio.position = [ FL FR ]
        stream.dont-remix = true
        target.object = {target}
        # Never fall back to another output (that could be this sink itself):
        # if the headset goes away, this waits for it.
        node.dont-fallback = true
        node.dont-reconnect = true
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
    fn config_places_all_eight_channels_and_mixes_them_to_stereo() {
        let conf = config("test_surround", "some_headset");
        assert_eq!(conf.matches("label = spatializer").count(), 8);
        assert_eq!(conf.matches("input = \"mixL:In ").count(), 8);
        assert_eq!(conf.matches("input = \"mixR:In ").count(), 8);
        assert!(conf.contains("audio.position = [ FL FR FC LFE RL RR SL SR ]"));
        assert!(conf.contains("name = spSL config = { filename = \"/usr/share/libmysofa/MIT_KEMAR_normal_pinna.sofa\" } control = { \"Azimuth\" = 90.0"));
        assert!(conf.contains("target.object = \"some_headset\""));
        assert!(conf.contains("\"Gain 4\" = 0.50"));
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Run with `cargo test -- --ignored`. Plays into an existing output but
    /// is never made the default and nothing plays into it, so it stays
    /// silent. (A target that doesn't exist makes PipeWire remove the sink.)
    #[test]
    #[ignore = "starts a real PipeWire filter in the current session"]
    fn graph_starts_with_eight_channels_and_goes_away() {
        let name = "quantumengine_surround_test";
        let (_, target) = pwgraph::find_node(|p| p["media.class"] == "Audio/Sink" && !HEADSET_GAME_OUTPUT.matches(p))
            .unwrap()
            .expect("an audio output to play into");
        let graph = Graph::start(name, "the test surround filter", &config(name, &target)).unwrap();
        let found = pwgraph::find_node(|p| p["node.name"] == name && p["audio.channels"] == 8).unwrap();
        assert!(found.is_some(), "an 8-channel sink called {name}");
        drop(graph);
        std::thread::sleep(Duration::from_millis(500));
        assert!(pwgraph::node_id(name).unwrap().is_none());
    }
}
