<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { CircleAlert, FileUp, Pencil, Plus } from "lucide-vue-next";
import { api } from "../api";
import { useApp } from "../stores/app";
import EqCurve, { type Handle } from "./EqCurve.vue";
import EqEditor from "./EqEditor.vue";
import PanelField from "./PanelField.vue";
import SegmentedControl from "./SegmentedControl.vue";
import ToggleSwitch from "./ToggleSwitch.vue";

const app = useApp();

const CORRECTIONS = [
  { value: "off", label: "Off" },
  { value: "stereo", label: "JBL stereo" },
  { value: "surround", label: "JBL surround" },
];
/** The profiles that aren't for one particular game. */
const GENERAL = new Set(["Flat", "Bass Boost", "Cinematic", "FPS", "MOBA", "RPG"]);

const eq = computed(() => app.state.eq);
const usable = computed(() => app.serviceUp && eq.value.profiles.length > 0);
/** On as far as anyone can tell: the service runs the EQ, so it's off when that's down. */
const on = computed(() => app.serviceUp && eq.value.enabled);
const busy = computed(() => app.pending !== null);

const groups = computed(() => {
  const mine = new Set(eq.value.custom);
  return [
    { name: "General", profiles: eq.value.profiles.filter((p) => GENERAL.has(p) && !mine.has(p)) },
    { name: "Games", profiles: eq.value.profiles.filter((p) => !GENERAL.has(p) && !mine.has(p)) },
  ];
});

// Editing a custom profile: the curve follows the draft, and while the EQ
// is on the draft plays too (both a moment after the last change).
const editing = computed(() => app.editor !== null);
const selectedBand = ref<number | null>(null);
const editorPoints = ref<number[] | null>(null);
const handles = computed<Handle[] | undefined>(() =>
  app.editor?.bands.map((b) => ({ freq: b.freq, gain: b.gain, hasGain: b.kind !== "highpass" && b.kind !== "lowpass" })),
);
let pause: number | undefined;

function draftValid(): boolean {
  return !!app.editor?.bands.every(
    (b) => b.freq >= 10 && b.freq <= 20000 && Math.abs(b.gain) <= 30 && b.q >= 0.05 && b.q <= 20,
  );
}

watch(
  () => app.editor && JSON.stringify(app.editor.bands),
  (draft) => {
    window.clearTimeout(pause);
    if (!draft) {
      editorPoints.value = null;
      selectedBand.value = null;
      return;
    }
    pause = window.setTimeout(async () => {
      const e = app.editor;
      if (!e || !draftValid()) return;
      const bands = e.bands.map((b) => ({ ...b }));
      try {
        editorPoints.value = await api.eqBandsResponse(bands, eq.value.correction);
        if (on.value) await api.previewEqBands(bands);
      } catch (err) {
        app.toast(err instanceof Error ? err.message : String(err));
      }
    }, 120);
  },
  { immediate: true },
);

function dragBand(i: number, freq: number, gain: number) {
  const b = app.editor?.bands[i];
  if (!b) return;
  b.freq = freq;
  if (handles.value?.[i]?.hasGain) b.gain = gain;
}

const points = ref<number[] | null>(null);
const preview = ref<number[] | null>(null);
const previewing = ref<string | null>(null);

watch(
  () => [eq.value.profile, eq.value.correction] as const,
  async ([profile, correction]) => {
    if (!profile || !correction) return;
    try {
      points.value = await app.curve(profile, correction);
    } catch {
      points.value = null;
    }
  },
  { immediate: true },
);

async function showPreview(profile: string | null) {
  previewing.value = profile;
  if (!profile || profile === eq.value.profile) {
    preview.value = null;
    return;
  }
  try {
    const curve = await app.curve(profile, eq.value.correction);
    if (previewing.value === profile) preview.value = curve;
  } catch {
    preview.value = null;
  }
}

function pendingOf(prefix: string): string | null {
  const p = app.pending;
  return p?.startsWith(`${prefix}:`) ? p.slice(prefix.length + 1) : null;
}

const pendingProfile = computed(() => pendingOf("eqprofile"));
</script>

<template>
  <div class="panel">
    <PanelField title="Equalizer" hint="Applied on this PC to the headset's game audio. Chat is left alone.">
      <template #aside>
        <ToggleSwitch
          label="Equalizer"
          :checked="on"
          :pending="pendingOf('eq') !== null"
          :disabled="!usable || busy"
          @toggle="app.setEqEnabled"
        />
      </template>
      <div class="plot">
        <EqCurve
          :freqs="app.info.eqFrequencies"
          :points="editing ? editorPoints : points"
          :preview="editing ? null : preview"
          :active="on"
          :handles="handles"
          :selected="selectedBand"
          @drag="dragBand"
          @select="selectedBand = $event"
        />
        <p class="plot-note">
          <template v-if="editing">
            <span class="swatch-line violet" />{{ app.editor?.name || "New profile" }}, editing
          </template>
          <template v-else-if="previewing && previewing !== eq.profile && preview">
            <span class="swatch-line violet" />{{ previewing }}, preview
          </template>
          <template v-else-if="app.state.serviceOutdated && !eq.profiles.length">
            The background service running now is too old for the EQ; restart it above
          </template>
          <template v-else-if="on">{{ eq.profile }} with {{ CORRECTIONS.find((c) => c.value === eq.correction)?.label ?? "no" }} tuning</template>
          <template v-else-if="app.serviceUp">Off: the sound passes through unchanged</template>
          <template v-else>Off: the background service isn't running</template>
        </p>
      </div>
      <p v-if="eq.status" class="status"><CircleAlert :size="15" :stroke-width="1.75" />{{ eq.status }}</p>
    </PanelField>

    <EqEditor v-if="editing" v-model:selected="selectedBand" class="editor-block" />
    <PanelField v-else title="Profile" :reading="eq.profile">
      <div v-for="g in groups" :key="g.name" class="group">
        <h4>{{ g.name }}</h4>
        <div class="profiles" role="radiogroup" :aria-label="`${g.name} profiles`" @mouseleave="showPreview(null)">
          <button
            v-for="p in g.profiles"
            :key="p"
            type="button"
            role="radio"
            class="profile"
            :aria-checked="p === eq.profile"
            :class="{ on: p === eq.profile, pending: p === pendingProfile }"
            :disabled="!usable || busy"
            @mouseenter="showPreview(p)"
            @focus="showPreview(p)"
            @blur="showPreview(null)"
            @click="p !== eq.profile && app.setEqProfile(p)"
          >
            <span class="mark" aria-hidden="true" />{{ p }}
          </button>
        </div>
      </div>
      <div class="group">
        <div class="group-head">
          <h4>Yours</h4>
          <div class="group-actions">
            <button
              type="button"
              class="small-action"
              :disabled="!usable || busy"
              :title="`Start from ${eq.profile}`"
              @click="app.openEditor(eq.profile, false)"
            >
              <Plus :size="14" :stroke-width="1.75" />New
            </button>
            <button
              type="button"
              class="small-action"
              :disabled="!usable || busy"
              title="AutoEQ or Equalizer APO ParametricEQ.txt, or an EasyEffects preset"
              @click="app.importProfile"
            >
              <FileUp :size="14" :stroke-width="1.75" />{{ app.pending === "eqedit:import" ? "Importing…" : "Import…" }}
            </button>
          </div>
        </div>
        <div v-if="eq.custom.length" class="profiles mine" role="radiogroup" aria-label="Your profiles" @mouseleave="showPreview(null)">
          <div v-for="p in eq.custom" :key="p" class="mine-row">
            <button
              type="button"
              role="radio"
              class="profile"
              :aria-checked="p === eq.profile"
              :class="{ on: p === eq.profile, pending: p === pendingProfile }"
              :disabled="!usable || busy"
              @mouseenter="showPreview(p)"
              @focus="showPreview(p)"
              @blur="showPreview(null)"
              @click="p !== eq.profile && app.setEqProfile(p)"
            >
              <span class="mark" aria-hidden="true" />{{ p }}
            </button>
            <button
              type="button"
              class="edit"
              :aria-label="`Edit ${p}`"
              title="Edit"
              :disabled="!usable || busy"
              @click="app.openEditor(p, true)"
            >
              <Pencil :size="14" :stroke-width="1.75" />
            </button>
          </div>
        </div>
        <p v-else class="mine-empty">
          Make your own from the current profile, or import a curve from
          <span class="nowrap">AutoEQ</span>, Equalizer APO or EasyEffects.
        </p>
      </div>
    </PanelField>

    <PanelField
      v-if="!editing"
      title="Headset tuning"
      hint="JBL's own correction for the Quantum 810, under the profile. JBL stereo is a gentle bass cut; JBL surround is the curve it uses with spatial sound."
    >
      <SegmentedControl
        label="Headset tuning"
        :options="CORRECTIONS"
        :value="eq.correction"
        :pending="pendingOf('eqcorrection')"
        :disabled="!usable || busy"
        @select="app.setEqCorrection"
      />
    </PanelField>

  </div>
</template>

<style scoped>
.plot {
  padding: 12px 12px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--film);
}

.plot-note {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--ink-3);
}

.swatch-line {
  width: 14px;
  height: 2px;
  border-radius: 2px;
}

.swatch-line.violet {
  background: var(--violet);
}

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

.group + .group {
  margin-top: 6px;
}

h4 {
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 500;
  color: var(--ink-3);
}

.profiles {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 2px 8px;
}

.profile {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 34px;
  padding: 0 10px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--ink-2);
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition:
    background 160ms,
    color 160ms;
}

.profile:hover:not(:disabled) {
  background: var(--violet-soft);
  color: var(--ink);
}

.profile.on {
  color: var(--ink);
  font-weight: 600;
}

.profile:disabled:not(.on) {
  color: var(--ink-3);
}

.mark {
  flex: none;
  width: 14px;
  height: 14px;
  border: 1.5px solid var(--line-strong);
  border-radius: 50%;
  transition:
    border-color 160ms,
    box-shadow 160ms;
}

.profile.on .mark {
  border-color: var(--ink);
  box-shadow: inset 0 0 0 3px var(--ground), inset 0 0 0 7px var(--ink);
}

.profile.pending .mark {
  border-color: var(--violet);
  animation: pulse 900ms ease-in-out infinite alternate;
}

@keyframes pulse {
  to {
    box-shadow: inset 0 0 0 3px var(--ground), inset 0 0 0 7px var(--violet);
  }
}

.group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 6px;
}

.group-head h4 {
  margin: 0;
}

.group-actions {
  display: flex;
  gap: 4px;
}

.small-action {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 9px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--ink-2);
  font-size: 12px;
  font-weight: 500;
  transition:
    border-color 160ms,
    color 160ms;
}

.small-action:hover:not(:disabled) {
  border-color: var(--ink-3);
  color: var(--ink);
}

.small-action:disabled {
  opacity: 0.4;
}

.mine-row {
  display: flex;
  align-items: center;
  min-width: 0;
}

.mine-row .profile {
  flex: 1;
  min-width: 0;
}

.edit {
  display: grid;
  place-items: center;
  flex: none;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--ink-3);
  opacity: 0;
  transition:
    opacity 120ms,
    color 120ms;
}

.mine-row:hover .edit,
.edit:focus-visible {
  opacity: 1;
}

.edit:hover {
  color: var(--ink);
  background: var(--film-raised);
}

.mine-empty {
  margin: 0;
  font-size: 12px;
  color: var(--ink-3);
  text-wrap: pretty;
}

.nowrap {
  white-space: nowrap;
}

.editor-block {
  margin-top: 26px;
}

</style>
