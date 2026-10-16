<script setup lang="ts">
// The equalizer's switch and frequency response. On a wide window it stays
// under the headset on every tab (the idea is DefSteam's); on a narrow one it
// sits at the top of the EQ tab. While a custom profile is edited the curve
// follows the draft and its numbered handles can be dragged.

import { computed, ref, watch } from "vue";
import { CircleAlert } from "lucide-vue-next";
import { api } from "../api";
import { useApp } from "../stores/app";
import EqCurve, { type Handle } from "./EqCurve.vue";
import PanelField from "./PanelField.vue";
import ToggleSwitch from "./ToggleSwitch.vue";

const app = useApp();

const CORRECTIONS: Record<string, string> = { off: "no", stereo: "JBL stereo", surround: "JBL surround" };

const eq = computed(() => app.state.eq);
const usable = computed(() => app.serviceUp && eq.value.profiles.length > 0);
/** On as far as anyone can tell: the service runs the EQ, so it's off when that's down. */
const on = computed(() => app.serviceUp && eq.value.enabled);
const busy = computed(() => app.pending !== null);
const switching = computed(() => app.pending?.startsWith("eq:") ?? false);

// Editing a custom profile: the curve follows the draft, and while the EQ
// is on the draft plays too (both a moment after the last change).
const editing = computed(() => app.editor !== null);
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
      app.eqBand = null;
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

// A profile hovered in the list is drawn over the current one.
watch(
  () => app.eqHover,
  async (profile) => {
    if (!profile || profile === eq.value.profile) {
      preview.value = null;
      return;
    }
    try {
      const curve = await app.curve(profile, eq.value.correction);
      if (app.eqHover === profile) preview.value = curve;
    } catch {
      preview.value = null;
    }
  },
);
</script>

<template>
  <PanelField title="Equalizer" :hint="app.wide ? undefined : `Applied on this PC to the headset's game audio. Chat is left alone.`">
    <template #aside>
      <ToggleSwitch label="Equalizer" :checked="on" :pending="switching" :disabled="!usable || busy" @toggle="app.setEqEnabled" />
    </template>
    <div class="plot">
      <EqCurve
        :freqs="app.info.eqFrequencies"
        :points="editing ? editorPoints : points"
        :preview="editing ? null : preview"
        :active="on"
        :handles="handles"
        :selected="app.eqBand"
        @drag="dragBand"
        @select="app.eqBand = $event"
      />
      <p class="plot-note">
        <template v-if="editing">
          <span class="swatch-line violet" />{{ app.editor?.name || "New profile" }}, editing
        </template>
        <template v-else-if="app.eqHover && app.eqHover !== eq.profile && preview">
          <span class="swatch-line violet" />{{ app.eqHover }}, preview
        </template>
        <template v-else-if="app.state.serviceOutdated && !eq.profiles.length">
          The background service running now is too old for the EQ; restart it above
        </template>
        <template v-else-if="on">{{ eq.profile }} with {{ CORRECTIONS[eq.correction] ?? "no" }} tuning</template>
        <template v-else-if="app.serviceUp">Off: the sound passes through unchanged</template>
        <template v-else>Off: the background service isn't running</template>
      </p>
    </div>
    <p v-if="eq.status" class="status"><CircleAlert :size="15" :stroke-width="1.75" />{{ eq.status }}</p>
  </PanelField>
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
  margin: 8px 0 0;
  font-size: 13px;
  color: var(--alert);
}

.status svg {
  flex: none;
  margin-top: 1px;
}
</style>
