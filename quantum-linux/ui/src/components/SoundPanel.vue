<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { ChevronDown, CircleAlert, Volume2, X } from "lucide-vue-next";
import { useApp } from "../stores/app";
import type { SurroundProfile } from "../api";
import PanelField from "./PanelField.vue";
import SegmentedControl from "./SegmentedControl.vue";
import ToggleSwitch from "./ToggleSwitch.vue";

const app = useApp();

const NOISE = [
  { value: "off", label: "Off" },
  { value: "on", label: "ANC" },
  { value: "talkthru", label: "TalkThru" },
];
const SIDETONE = [
  { value: "off", label: "Off" },
  { value: "low", label: "Low" },
  { value: "mid", label: "Mid" },
  { value: "high", label: "High" },
];
const SURROUND_OPTIONS = [
  { value: "Default", label: "Default" },
  { value: "Wide", label: "Wide" },
  { value: "Room", label: "Room" },
  { value: "Clear", label: "Clear" },
];
const DEFAULT_SURROUND: SurroundProfile = { spatialCues: 100, reflections: 45, ambience: 10, decayMs: 280 };
const TUNING_CONTROLS = [
  { key: "spatialCues", label: "Spatial cues", hint: "Scales the left/right difference in KEMAR's ear filters; 100% is the original measurement.", min: 70, max: 150, unit: "%" },
  { key: "reflections", label: "Early reflections", hint: "Gain per simulated wall bounce; more can make the room sound larger.", min: 0, max: 70, unit: "%" },
  { key: "ambience", label: "Room ambience", hint: "Late reverberation energy relative to the direct sound.", min: 0, max: 30, unit: "%" },
  { key: "decayMs", label: "Reverb decay", hint: "RT60: time for the late reverb to fade by about 60 dB.", min: 100, max: 700, step: 10, unit: " ms" },
] as const;
const SPEAKER_TESTS = [
  { id: "FL", label: "Front left" },
  { id: "FC", label: "Front center" },
  { id: "FR", label: "Front right" },
  { id: "SL", label: "Side left" },
  { id: "SR", label: "Side right" },
  { id: "RL", label: "Rear left" },
  { id: "RR", label: "Rear right" },
  { id: "LFE", label: "LFE · low-frequency effects" },
];

function pendingOf(prefix: string): string | null {
  const p = app.pending;
  return p?.startsWith(`${prefix}:`) ? p.slice(prefix.length + 1) : null;
}

const surround = computed(() => app.state.surround);
const surroundOn = computed(() => app.serviceUp && surround.value.enabled);
const surroundProfiles = computed<Record<string, SurroundProfile>>(() => {
  try {
    return surround.value.profiles ? JSON.parse(surround.value.profiles) as Record<string, SurroundProfile> : {};
  } catch {
    return {};
  }
});
const surroundTuning = computed(() => surroundProfiles.value[surround.value.profile] ?? DEFAULT_SURROUND);
const surroundProfilePending = computed(() => pendingOf("surround-profile"));
const outputGainPreview = ref<number | null>(null);
const displayedOutputGain = computed(() => outputGainPreview.value ?? surround.value.outputGain);
const speakerDialogOpen = ref(false);
const speakerDialog = ref<HTMLDialogElement | null>(null);
const playingSpeaker = ref<string | null>(null);
let speakerResetTimer: number | undefined;

watch(speakerDialogOpen, async (open) => {
  await nextTick();
  const dialog = speakerDialog.value;
  if (!dialog) return;
  if (open && !dialog.open) dialog.showModal();
  else if (!open && dialog.open) dialog.close();
});

function closeSpeakerDialog() {
  speakerDialogOpen.value = false;
}

async function playSpeakerTest(speaker: string) {
  playingSpeaker.value = speaker;
  window.clearTimeout(speakerResetTimer);
  speakerResetTimer = window.setTimeout(() => (playingSpeaker.value = null), 1200);
  await app.playSurroundSpeakerTest(speaker);
}

function updateSurroundTuning(key: keyof SurroundProfile, event: Event) {
  const value = Number((event.currentTarget as HTMLInputElement).value);
  const next = { ...surroundTuning.value, [key]: value };
  void app.tuneSurroundProfile(surround.value.profile || "Default", next);
}

function previewOutputGain(event: Event) {
  outputGainPreview.value = Number((event.currentTarget as HTMLInputElement).value);
}

async function commitOutputGain() {
  const value = outputGainPreview.value;
  if (value === null) return;
  await app.setSurroundOutputGain(value);
  outputGainPreview.value = null;
}

const anc = computed(() => (app.ready ? app.headset.anc : ""));
const sidetone = computed(() => (app.ready ? app.headset.sidetone : ""));

const noiseHint = computed(() => {
  switch (pendingOf("anc") ?? anc.value) {
    case "on":
      return "Cancels the noise around you.";
    case "talkthru":
      return "Lets your surroundings through, so you can hear people without taking the headset off.";
    case "off":
      return "No noise processing.";
    default:
      return "Switch the headset on to change this.";
  }
});
</script>

<template>
  <div class="panel">
    <PanelField title="Noise control" :hint="noiseHint">
      <SegmentedControl
        label="Noise control"
        :options="NOISE"
        :value="anc"
        :pending="pendingOf('anc')"
        :disabled="app.locked"
        @select="app.setAnc"
      />
    </PanelField>
    <PanelField title="Sidetone" hint="How much of your own voice you hear while you talk.">
      <SegmentedControl
        label="Sidetone"
        :options="SIDETONE"
        :value="sidetone"
        :pending="pendingOf('sidetone')"
        :disabled="app.locked"
        @select="app.setSidetone"
      />
    </PanelField>
    <PanelField
      title="Virtual surround 7.1"
      hint="Applies 7.1 processing transparently to the JBL Game output, which stays selected in KDE. Games set to 7.1 sound from all around you, and stereo stays in the front speakers. The EQ's headset tuning switches to JBL surround with it, as on Windows."
    >
      <template #aside>
        <ToggleSwitch
          label="Virtual surround 7.1"
          :checked="surroundOn"
          :pending="app.pending?.startsWith('surround:') ?? false"
          :disabled="!app.serviceUp || app.pending !== null"
          @toggle="app.setSurround"
        />
      </template>
      <p v-if="surround.status && surroundOn" class="status"><CircleAlert :size="15" :stroke-width="1.75" />{{ surround.status }}</p>
    </PanelField>
    <PanelField title="Headset output gain" hint="Applied to the Game output whether surround is on or off. Higher levels can make loud sounds clip.">
      <label class="tuning-control">
        <span class="tuning-heading">
          <span>Output gain</span>
          <span class="num">{{ displayedOutputGain }}%</span>
        </span>
        <input
          type="range"
          min="25"
          max="300"
          step="5"
          :value="displayedOutputGain"
          aria-label="Headset output gain"
          :disabled="!app.serviceUp || app.state.serviceOutdated || app.pending !== null"
          @input="previewOutputGain"
          @change="commitOutputGain"
        />
      </label>
    </PanelField>
    <details class="advanced">
      <summary>Advanced <ChevronDown :size="15" :stroke-width="1.75" /></summary>
      <PanelField
        title="Spatial sound profile"
        hint="Adjust how KEMAR's non-personalized head measurements are rendered. Default keeps the original QuantumEngine settings; changes are saved per profile and switched in without stopping playback."
      >
        <template #aside>
          <button class="speaker-test-trigger" type="button" :disabled="!app.serviceUp" @click="speakerDialogOpen = true">
            <Volume2 :size="14" :stroke-width="1.75" />Test speakers
          </button>
        </template>
        <SegmentedControl
          label="Spatial sound profile"
          :options="SURROUND_OPTIONS"
          :value="surround.profile || 'Default'"
          :pending="surroundProfilePending"
          :disabled="!app.serviceUp || app.state.serviceOutdated || app.pending !== null"
          @select="app.setSurroundProfile"
        />
        <div class="tuning-list">
          <label v-for="control in TUNING_CONTROLS" :key="control.key" class="tuning-control">
            <span class="tuning-heading">
              <span>{{ control.label }}</span>
              <span class="num">{{ surroundTuning[control.key] }}{{ control.unit }}</span>
            </span>
            <input
              type="range"
              :min="control.min"
              :max="control.max"
              :step="'step' in control ? control.step : 1"
              :value="surroundTuning[control.key]"
              :aria-label="control.label"
              :disabled="!app.serviceUp || app.state.serviceOutdated || app.pending !== null"
              @change="updateSurroundTuning(control.key, $event)"
            />
            <span class="tuning-hint">{{ control.hint }}</span>
          </label>
        </div>
        <p v-if="app.pending?.startsWith('surround-tune:')" class="rebuilding">Rebuilding surround filters…</p>
      </PanelField>
    </details>

    <Teleport to="body">
      <dialog ref="speakerDialog" class="speaker-dialog" aria-labelledby="speaker-dialog-title" @close="speakerDialogOpen = false">
        <header class="dialog-header">
          <div>
            <h2 id="speaker-dialog-title">7.1 speaker test</h2>
            <p>Play a short sound through one virtual speaker at a time.</p>
          </div>
          <button class="dialog-close" type="button" aria-label="Close speaker test" @click="closeSpeakerDialog">
            <X :size="17" :stroke-width="1.75" />
          </button>
        </header>
        <p v-if="!surroundOn" class="dialog-note">Turn on Virtual surround 7.1 above to play the channel tests.</p>
        <div class="speaker-buttons">
          <button
            v-for="speaker in SPEAKER_TESTS"
            :key="speaker.id"
            class="speaker-test-button"
            type="button"
            :disabled="!surroundOn || app.state.serviceOutdated || app.pending !== null"
            @click="playSpeakerTest(speaker.id)"
          >
            <Volume2 :size="16" :stroke-width="1.75" />
            <span>{{ speaker.label }}</span>
            <span class="speaker-id">{{ playingSpeaker === speaker.id ? "Playing…" : speaker.id }}</span>
          </button>
        </div>
        <p class="dialog-footnote">Tests use short, low-level tones. The LFE test is a bass tone; bass is intentionally centered by the surround renderer.</p>
      </dialog>
    </Teleport>
  </div>
</template>

<style scoped>
.status {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  margin: 0;
  font-size: 13px;
  color: var(--alert);
}

.status svg {
  flex: none;
  margin-top: 1px;
}

.speaker-test-trigger,
.dialog-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 30px;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--ground);
  color: var(--ink-2);
  font-size: 11px;
  font-weight: 550;
  cursor: pointer;
}

.speaker-test-trigger:hover:not(:disabled),
.dialog-close:hover {
  border-color: var(--line-strong);
  background: var(--film-raised);
  color: var(--ink);
}

.speaker-test-trigger:disabled {
  opacity: 0.5;
  cursor: default;
}

.speaker-dialog {
  width: min(440px, calc(100vw - 32px));
  max-width: none;
  max-height: min(640px, calc(100vh - 32px));
  padding: 22px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius);
  background: var(--ground);
  color: var(--ink);
  box-shadow: 0 20px 70px rgb(0 0 0 / 45%);
}

.speaker-dialog::backdrop {
  background: rgb(0 0 0 / 62%);
  backdrop-filter: blur(3px);
}

.dialog-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.dialog-header h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.dialog-header p,
.dialog-note,
.dialog-footnote {
  margin: 4px 0 0;
  color: var(--ink-3);
  font-size: 12px;
}

.dialog-close {
  width: 32px;
  min-width: 32px;
  height: 32px;
  padding: 0;
}

.speaker-buttons {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
  margin-top: 14px;
}

.speaker-test-button {
  display: flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
  min-height: 42px;
  padding: 0 11px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--film);
  color: var(--ink-2);
  text-align: left;
  font-size: 12px;
  cursor: pointer;
}

.speaker-test-button:hover:not(:disabled) {
  border-color: var(--violet);
  background: var(--violet-soft);
  color: var(--ink);
}

.speaker-test-button:disabled {
  opacity: 0.5;
  cursor: default;
}

.speaker-test-button svg {
  flex: none;
}

.speaker-test-button > span:nth-child(2) {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.speaker-id {
  margin-left: auto;
  color: var(--ink-3);
  font-size: 10px;
  white-space: nowrap;
}

.dialog-note {
  padding: 9px 11px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--film);
}

.dialog-footnote {
  margin-top: 14px;
  line-height: 1.45;
}

.advanced {
  margin-top: 20px;
  border-top: 1px solid var(--line);
}

.advanced summary {
  display: flex;
  width: fit-content;
  align-items: center;
  gap: 6px;
  padding: 12px 2px 6px;
  color: var(--ink-2);
  font-size: 12px;
  font-weight: 550;
  cursor: pointer;
  list-style: none;
}

.advanced summary::-webkit-details-marker {
  display: none;
}

.advanced summary svg {
  transition: transform 180ms var(--ease-out);
}

.advanced[open] summary svg {
  transform: rotate(180deg);
}

.advanced :deep(.field) {
  margin-top: 8px;
  padding-top: 0;
  border-top: 0;
}

.tuning-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px 20px;
  margin-top: 2px;
}

.tuning-control {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 5px;
}

.tuning-heading {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  font-size: 12px;
  font-weight: 550;
}

.tuning-heading .num {
  color: var(--ink-2);
  white-space: nowrap;
}

input[type="range"] {
  width: 100%;
  margin: 2px 0;
  accent-color: var(--ink);
}

.tuning-hint,
.rebuilding {
  margin: 0;
  font-size: 11px;
  line-height: 1.4;
  color: var(--ink-3);
}

@media (max-width: 560px) {
  .tuning-list {
    grid-template-columns: 1fr;
  }
}
</style>
