<script setup lang="ts">
// The EQ's frequency response, drawn from the service's own numbers: gain in
// dB against frequency on a log scale. A previewed profile (hovered or
// focused in the list) is drawn over the current one in violet.

import { computed } from "vue";

const props = defineProps<{
  freqs: number[];
  /** dB at each of `freqs`. */
  points: number[] | null;
  preview: number[] | null;
  /** Whether the curve is being applied (else it's drawn as a pencil line). */
  active: boolean;
}>();

const W = 400;
const H = 150;
const PAD_L = 30;
const PAD_R = 8;
const PAD_T = 10;
const PAD_B = 22;
const RANGE = 12;
const LO = Math.log10(20);
const HI = Math.log10(20000);

const x = (f: number) => PAD_L + ((Math.log10(f) - LO) / (HI - LO)) * (W - PAD_L - PAD_R);
const y = (db: number) => {
  const clamped = Math.max(-RANGE, Math.min(RANGE, db));
  return PAD_T + ((RANGE - clamped) / (2 * RANGE)) * (H - PAD_T - PAD_B);
};

function line(points: number[] | null): string {
  if (!points || points.length !== props.freqs.length) return "";
  return points.map((db, i) => `${i ? "L" : "M"}${x(props.freqs[i]!).toFixed(1)} ${y(db).toFixed(1)}`).join(" ");
}

const current = computed(() => line(props.points));
const area = computed(() => {
  if (!current.value) return "";
  const first = x(props.freqs[0]!).toFixed(1);
  const last = x(props.freqs[props.freqs.length - 1]!).toFixed(1);
  return `${current.value} L${last} ${y(0)} L${first} ${y(0)} Z`;
});
const previewLine = computed(() => line(props.preview));

const GRID_F = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
const LABEL_F: [number, string][] = [
  [100, "100"],
  [1000, "1k"],
  [10000, "10k"],
];
const GRID_DB = [-12, -6, 0, 6, 12];
</script>

<template>
  <svg :viewBox="`0 0 ${W} ${H}`" class="curve" role="img" aria-label="Equalizer frequency response">
    <g class="grid">
      <line v-for="f in GRID_F" :key="f" :x1="x(f)" :x2="x(f)" :y1="PAD_T" :y2="H - PAD_B" />
      <line
        v-for="db in GRID_DB"
        :key="db"
        :class="{ zero: db === 0 }"
        :x1="PAD_L"
        :x2="W - PAD_R"
        :y1="y(db)"
        :y2="y(db)"
      />
    </g>
    <g class="labels">
      <text v-for="[f, label] in LABEL_F" :key="f" :x="x(f)" :y="H - 6" text-anchor="middle">{{ label }}</text>
      <text v-for="db in [-6, 0, 6]" :key="db" :x="PAD_L - 7" :y="y(db) + 3.5" text-anchor="end">
        {{ db > 0 ? `+${db}` : db === 0 ? "0" : `−${-db}` }}
      </text>
    </g>
    <path v-if="area && active" class="area" :d="area" />
    <path v-if="current" class="line" :class="{ inactive: !active }" :d="current" />
    <path v-if="previewLine" class="preview" :d="previewLine" />
  </svg>
</template>

<style scoped>
.curve {
  display: block;
  width: 100%;
  height: auto;
}

.grid line {
  stroke: var(--line);
  stroke-width: 1;
}

.grid line.zero {
  stroke: var(--line-strong);
}

.labels text {
  font-size: 10px;
  fill: var(--ink-3);
  font-variant-numeric: tabular-nums;
}

.area {
  fill: rgb(236 232 225 / 0.07);
}

.line {
  fill: none;
  stroke: var(--ink);
  stroke-width: 2;
  stroke-linejoin: round;
  stroke-linecap: round;
  transition: d 380ms var(--ease-out);
}

.line.inactive {
  stroke: var(--ink-3);
  stroke-width: 1.5;
  stroke-dasharray: 4 4;
}

.preview {
  fill: none;
  stroke: var(--violet);
  stroke-width: 1.5;
  stroke-linejoin: round;
}
</style>
