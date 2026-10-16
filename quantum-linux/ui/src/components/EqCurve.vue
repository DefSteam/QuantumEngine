<script setup lang="ts">
// The EQ's frequency response, drawn from the service's own numbers: gain in
// dB against frequency on a log scale. A previewed profile (hovered or
// focused in the list) is drawn over the current one in violet. While a
// custom profile is edited, each band has a handle that can be dragged to
// change its frequency and gain.

import { computed, ref } from "vue";

export interface Handle {
  freq: number;
  gain: number;
  /** Pass filters have no gain; their handle sits on the 0 dB line. */
  hasGain: boolean;
}

const props = defineProps<{
  freqs: number[];
  /** dB at each of `freqs`. */
  points: number[] | null;
  preview: number[] | null;
  /** Whether the curve is being applied (else it's drawn as a pencil line). */
  active: boolean;
  handles?: Handle[];
  selected?: number | null;
}>();

const emit = defineEmits<{
  drag: [index: number, freq: number, gain: number];
  select: [index: number];
}>();

const W = 400;
const H = 150;
const PAD_L = 30;
const PAD_R = 8;
const PAD_T = 10;
const PAD_B = 22;
const LO = Math.log10(20);
const HI = Math.log10(20000);

/** ±12 dB, or wider in 6 dB steps when an edited band goes further. */
const range = computed(() => {
  const most = Math.max(0, ...(props.handles ?? []).map((h) => Math.abs(h.gain)), ...(props.points ?? []).map(Math.abs));
  return Math.max(12, Math.ceil(most / 6) * 6);
});

const x = (f: number) => PAD_L + ((Math.log10(f) - LO) / (HI - LO)) * (W - PAD_L - PAD_R);
const y = (db: number) => {
  const r = range.value;
  const clamped = Math.max(-r, Math.min(r, db));
  return PAD_T + ((r - clamped) / (2 * r)) * (H - PAD_T - PAD_B);
};
const freqAt = (px: number) => 10 ** (LO + ((px - PAD_L) / (W - PAD_L - PAD_R)) * (HI - LO));
const gainAt = (py: number) => range.value - ((py - PAD_T) / (H - PAD_T - PAD_B)) * 2 * range.value;

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
const gridDb = computed(() => {
  const r = range.value;
  const step = r > 12 ? 12 : 6;
  const out: number[] = [];
  for (let db = -r; db <= r; db += step) out.push(db);
  return out;
});
const labelDb = computed(() => gridDb.value.filter((db) => Math.abs(db) < range.value));

// Dragging a handle.
const svg = ref<SVGSVGElement | null>(null);
const dragging = ref<number | null>(null);

function toSvg(e: PointerEvent): { px: number; py: number } | null {
  const ctm = svg.value?.getScreenCTM();
  if (!ctm) return null;
  const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(ctm.inverse());
  return { px: p.x, py: p.y };
}

function start(i: number, e: PointerEvent) {
  dragging.value = i;
  emit("select", i);
  (e.target as Element).setPointerCapture(e.pointerId);
}

function move(e: PointerEvent) {
  const i = dragging.value;
  if (i === null) return;
  const p = toSvg(e);
  const h = props.handles?.[i];
  if (!p || !h) return;
  const freq = Math.min(20000, Math.max(20, freqAt(p.px)));
  const gain = h.hasGain ? Math.min(30, Math.max(-30, gainAt(p.py))) : 0;
  emit("drag", i, Math.round(freq), Math.round(gain * 10) / 10);
}

function end() {
  dragging.value = null;
}
</script>

<template>
  <svg
    ref="svg"
    :viewBox="`0 0 ${W} ${H}`"
    class="curve"
    :class="{ editing: !!handles }"
    role="img"
    aria-label="Equalizer frequency response"
    @pointermove="move"
    @pointerup="end"
    @pointercancel="end"
  >
    <g class="grid">
      <line v-for="f in GRID_F" :key="f" :x1="x(f)" :x2="x(f)" :y1="PAD_T" :y2="H - PAD_B" />
      <line
        v-for="db in gridDb"
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
      <text v-for="db in labelDb" :key="db" :x="PAD_L - 7" :y="y(db) + 3.5" text-anchor="end">
        {{ db > 0 ? `+${db}` : db === 0 ? "0" : `−${-db}` }}
      </text>
    </g>
    <path v-if="area && active" class="area" :d="area" />
    <path v-if="current" class="line" :class="{ inactive: !active }" :d="current" />
    <path v-if="previewLine" class="preview" :d="previewLine" />
    <g v-if="handles" class="handles" aria-hidden="true">
      <g
        v-for="(h, i) in handles"
        :key="i"
        class="handle"
        :class="{ on: selected === i, dragging: dragging === i }"
        @pointerdown.prevent="start(i, $event)"
      >
        <circle class="hit" :cx="x(h.freq)" :cy="y(h.hasGain ? h.gain : 0)" r="13" />
        <circle class="dot" :cx="x(h.freq)" :cy="y(h.hasGain ? h.gain : 0)" r="8" />
        <text class="num" :x="x(h.freq)" :y="y(h.hasGain ? h.gain : 0) + 3.5" text-anchor="middle">{{ i + 1 }}</text>
      </g>
    </g>
  </svg>
</template>

<style scoped>
.curve {
  display: block;
  width: 100%;
  height: auto;
  touch-action: none;
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

.handle {
  cursor: grab;
}

.handle.dragging {
  cursor: grabbing;
}

.handle .hit {
  fill: transparent;
}

.handle .dot {
  fill: var(--film-raised);
  stroke: var(--ink);
  stroke-width: 1.4;
  transition: fill 120ms;
}

.handle text {
  font-size: 10px;
  font-weight: 600;
  fill: var(--ink);
  pointer-events: none;
}

.handle:hover .dot,
.handle.on .dot {
  fill: var(--violet);
  stroke: var(--violet);
}

.handle.on text,
.handle:hover text {
  fill: var(--ground);
}
</style>
