<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useApp } from "../stores/app";
import EqCurvePanel from "./EqCurvePanel.vue";
import PanelField from "./PanelField.vue";
import SegmentedControl from "./SegmentedControl.vue";

const app = useApp();
const layoutQuery = typeof window === "undefined"
  ? null
  : window.matchMedia("(min-width: 840px)");
const wideLayout = ref(layoutQuery?.matches ?? false);

function updateLayout() {
  wideLayout.value = layoutQuery?.matches ?? false;
}

onMounted(() => layoutQuery?.addEventListener("change", updateLayout));
onBeforeUnmount(() => layoutQuery?.removeEventListener("change", updateLayout));

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

const groups = computed(() => [
  { name: "General", profiles: eq.value.profiles.filter((p) => GENERAL.has(p)) },
  { name: "Games", profiles: eq.value.profiles.filter((p) => !GENERAL.has(p)) },
]);

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
    <div class="eq-layout">
      <div v-if="!wideLayout" class="eq-column eq-curve-column">
        <EqCurvePanel
          :freqs="app.info.eqFrequencies"
          :points="points"
          :preview="preview"
          :active="on"
          :pending="pendingOf('eq') !== null"
          :usable="usable"
          :busy="busy"
          :profile="eq.profile"
          :correction-label="CORRECTIONS.find((c) => c.value === eq.correction)?.label ?? 'no'"
          :previewing="previewing"
          :service-up="app.serviceUp"
          :service-outdated="app.state.serviceOutdated"
          :has-profiles="eq.profiles.length > 0"
          :status="eq.status"
          @toggle="app.setEqEnabled"
        />
      </div>

      <div class="eq-column eq-options-column">
        <PanelField title="Profile" :reading="eq.profile">
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
        </PanelField>

        <PanelField
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
    </div>
  </div>
</template>

<style scoped>
.eq-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
}

.eq-column {
  min-width: 0;
}

.eq-column + .eq-column {
  margin-top: 24px;
  padding-top: 18px;
  border-top: 1px solid var(--line);
}

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

</style>
