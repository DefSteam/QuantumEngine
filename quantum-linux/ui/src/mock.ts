// A stand-in backend for working on the interface in a plain browser
// (`npm run dev`, then open http://localhost:1420/?scenario=ready). Only
// loaded by the dev server outside Tauri; see main.ts. The data is made up.
//
// Scenarios: ready, custom, update, outdated, low, off, nodongle, noaccess, down.
// `&editor` opens the EQ profile editor on the selected profile.
// Add `&section=lights|eq|device` to open another section.

import { mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import type { InfoDto, LightingDto, StateDto } from "./bindings";

const PROFILES = [
  "Flat", "Bass Boost", "Cinematic", "FPS", "MOBA", "RPG", "Apex Legends", "CS2", "Dota 2",
  "Escape from Tarkov", "Fortnite", "GTA 5", "League of Legends", "LN3 Immersion", "LN3 Thrill",
  "Overwatch 2", "PUBG", "Valorant", "WoW",
];

const FREQS = Array.from({ length: 64 }, (_, i) => 20 * 1000 ** (i / 63));

/** Rough made-up curves: a few bells per profile, on a log axis. */
const SHAPES: Record<string, [number, number, number][]> = {
  Flat: [],
  "Bass Boost": [[60, 6, 1.2], [180, 2.5, 1]],
  Cinematic: [[50, 4, 1], [3000, 2, 1], [12000, 3, 1.2]],
  FPS: [[80, -3, 1], [2500, 4, 1.3], [6000, 3, 1.5]],
  MOBA: [[120, 2, 1], [1500, 2.5, 1], [8000, 2, 1]],
  RPG: [[60, 3, 1], [800, 1.5, 1], [10000, 2.5, 1]],
};

function curve(profile: string, correction: string): number[] {
  const shape = SHAPES[profile] ?? [[100, 2, 1], [2000 + (profile.length * 317) % 4000, 3, 1.3], [9000, -2, 1]];
  const bells: [number, number, number][] = [...shape];
  if (correction === "stereo") bells.push([200, -1.6, 0.9]);
  if (correction === "surround") bells.push([13000, 7, 1], [5000, -4, 1.4]);
  return FREQS.map((f) =>
    bells.reduce((sum, [fc, g, width]) => sum + g * Math.exp(-((Math.log2(f / fc) / width) ** 2)), 0),
  );
}

function scenarioState(scenario: string): StateDto {
  const ready = !["off", "nodongle", "noaccess", "down"].includes(scenario);
  return {
    service: scenario === "down" ? "down" : "up",
    serviceError: scenario === "down" ? "The background service (quantumengined) didn't start." : "",
    serviceOutdated: scenario === "outdated",
    headset: {
      link: ready ? "ready" : scenario === "off" ? "headset_off" : scenario === "noaccess" ? "no_access" : "no_dongle",
      linkDetail: "",
      battery: scenario === "low" ? 11 : 82,
      anc: "on",
      sidetone: "low",
      micActive: true,
      dial: 11,
      lightsOn: true,
      deviceName: "PC-cachyos #1",
      serial: ready ? "SN-0000-DEMO" : "",
      firmware: ready ? ["0.7.2", "0.8.2", "1.0.2", "0.7.2"] : [],
      busy: false,
      lastError: "",
    },
    eq:
      scenario === "outdated"
        ? { enabled: false, profile: "", profiles: [], correction: "", status: "", custom: [], previewing: false }
        : {
            enabled: true,
            profile: scenario === "custom" ? "Night Curve" : "CS2",
            profiles: [...PROFILES, "Night Curve"],
            correction: "stereo",
            status: "",
            custom: ["Night Curve"],
            previewing: false,
          },
    surround: { enabled: false, status: "" },
    update: {
      serviceVersion: scenario === "outdated" ? "" : "0.3.0",
      available: scenario === "update" ? "0.3.1" : "",
      canInstall: true,
      status: "",
      busy: false,
      checks: true,
    },
  };
}

export function installMock(scenario: string) {
  let state = scenarioState(scenario);
  let lighting: LightingDto = {
    ring: { brightness: 90, colours: ["#8a3cff", "#2fb7ff"], effect: 0 },
    logo: { brightness: 100, colours: ["#ffffff"], effect: 0 },
  };
  const info: InfoDto = {
    version: "0.3.0",
    effects: [
      [0, "Fade"], [1, "Instant"], [2, "Chase"], [3, "Strobe"], [4, "Chase strobe logo"],
      [5, "Chase flash logo"], [7, "Fast"], [8, "Chase blink logo"], [9, "Rainbow logo"], [10, "Spectrum"],
    ].map(([id, label]) => ({ id: id as number, label: label as string })),
    maxColours: 5,
    eqFrequencies: FREQS,
    eqMaxBands: 18,
  };
  type MockBand = { kind: string; freq: number; gain: number; q: number };
  const custom: Record<string, MockBand[]> = {
    "Night Curve": [
      { kind: "lowshelf", freq: 105, gain: -4, q: 0.7 },
      { kind: "bell", freq: 2900, gain: 3, q: 2.7 },
      { kind: "highshelf", freq: 9000, gain: -3, q: 0.7 },
      { kind: "highpass", freq: 25, gain: 0, q: 0.7 },
    ],
  };
  const bandsCurve = (bands: MockBand[]) =>
    FREQS.map((f) =>
      bands.reduce((sum, b) => {
        const oct = Math.log2(f / b.freq);
        if (b.kind === "bell") return sum + b.gain * Math.exp(-((oct * b.q) ** 2));
        if (b.kind === "lowshelf") return sum + b.gain / (1 + Math.exp(oct * 4));
        if (b.kind === "highshelf") return sum + b.gain / (1 + Math.exp(-oct * 4));
        if (b.kind === "highpass") return sum + Math.min(0, 12 * oct);
        return sum + Math.min(0, -12 * oct);
      }, 0),
    );

  const change = (f: (s: StateDto) => void, delay = 650) =>
    new Promise<void>((resolve) => {
      window.setTimeout(() => {
        const next = structuredClone(state);
        f(next);
        state = next;
        void emit("state-event", state);
        resolve();
      }, delay);
    });

  mockIPC(
    async (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "state":
          return state;
        case "info":
          return info;
        case "lighting":
          return lighting;
        case "set_anc":
          await change((s) => (s.headset.anc = a.mode as string));
          return a.mode;
        case "set_sidetone":
          await change((s) => (s.headset.sidetone = a.level as string));
          return a.level;
        case "set_lights":
          await change((s) => (s.headset.lightsOn = a.on as boolean));
          return a.on;
        case "apply_lighting":
          await change((s) => (s.headset.lightsOn = true), 900);
          lighting = a.lighting as LightingDto;
          return null;
        case "set_eq_enabled":
          await change((s) => (s.eq.enabled = a.enabled as boolean), 200);
          return a.enabled;
        case "set_eq_profile":
          await change((s) => (s.eq.profile = a.name as string), 200);
          return a.name;
        case "set_eq_correction":
          await change((s) => (s.eq.correction = a.correction as string), 200);
          return a.correction;
        case "set_surround_enabled":
          await change((s) => (s.surround.enabled = a.enabled as boolean), 500);
          return a.enabled;
        case "eq_profile_bands": {
          const name = a.name as string;
          if (custom[name]) return custom[name];
          const gains = [-6, -4, -3, -2, 2, 3, 4, 4, 3, 0];
          return [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000].map((freq, i) => ({ kind: "bell", freq, gain: gains[i], q: 1 }));
        }
        case "eq_bands_response":
          return bandsCurve(a.bands as MockBand[]);
        case "save_eq_profile": {
          const name = a.name as string;
          if (a.original && a.original !== name) delete custom[a.original as string];
          custom[name] = a.bands as MockBand[];
          await change((s) => {
            s.eq.custom = Object.keys(custom);
            s.eq.profiles = [...PROFILES, ...s.eq.custom];
            s.eq.profile = name;
            s.eq.previewing = false;
          }, 300);
          return name;
        }
        case "delete_eq_profile":
          delete custom[a.name as string];
          await change((s) => {
            s.eq.custom = Object.keys(custom);
            s.eq.profiles = [...PROFILES, ...s.eq.custom];
            if (s.eq.profile === a.name) s.eq.profile = "Flat";
          }, 300);
          return null;
        case "preview_eq_bands":
          if (!state.eq.previewing) await change((s) => (s.eq.previewing = true), 50);
          return null;
        case "end_eq_preview":
          await change((s) => (s.eq.previewing = false), 50);
          return null;
        case "import_eq_profile":
          return null;
        case "eq_response":
          if (custom[a.profile as string]) return bandsCurve(custom[a.profile as string]!);
          return curve(a.profile as string, a.correction as string);
        case "check_for_updates":
          await change((s) => (s.update.status = "QuantumEngine 0.3.0 is up to date"), 900);
          return "";
        case "install_update":
          await change((s) => (s.update.busy = true), 100);
          return null;
        case "set_update_checks":
          await change((s) => (s.update.checks = a.enabled as boolean), 200);
          return a.enabled;
        case "restart_service":
          await change((s) => {
            Object.assign(s, scenarioState("ready"));
          }, 1200);
          return null;
        case "open_release_page":
        case "open_project_page":
          return null;
        default:
          throw new Error(`mock: no handler for ${cmd}`);
      }
    },
    { shouldMockEvents: true },
  );
}
