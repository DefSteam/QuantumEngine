<script setup lang="ts">
// A hue/saturation wheel for one lighting colour: hue runs round the rim,
// saturation from white at the centre to the pure hue at the edge. The LEDs
// have their own brightness slider, so the wheel always picks at full value.

import { computed, ref, watch } from "vue";

const props = defineProps<{ modelValue: string; label: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

/** Colours the LEDs show cleanly: one or two channels fully on. */
const PRESETS = ["#ff0000", "#ff8000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#8000ff", "#ff00ff", "#ffffff"];

const wheel = ref<HTMLElement | null>(null);
const hsv = ref(toHsv(props.modelValue));
const hex = ref(props.modelValue);

watch(
  () => props.modelValue,
  (v) => {
    if (v.toLowerCase() !== fromHsv(hsv.value)) hsv.value = toHsv(v);
    hex.value = v;
  },
);

const thumb = computed(() => {
  const a = (hsv.value.h * Math.PI) / 180;
  return { left: `${50 + 50 * hsv.value.s * Math.sin(a)}%`, top: `${50 - 50 * hsv.value.s * Math.cos(a)}%` };
});

function set(h: number, s: number) {
  hsv.value = { h: (h + 360) % 360, s: Math.min(1, Math.max(0, s)), v: 1 };
  emit("update:modelValue", fromHsv(hsv.value));
}

function pick(e: PointerEvent) {
  const r = wheel.value!.getBoundingClientRect();
  const x = e.clientX - (r.left + r.width / 2);
  const y = e.clientY - (r.top + r.height / 2);
  // The LEDs show even a little white mixed in, so the outer band snaps to
  // the full colour: dragging to the rim gives #ff0000, not #ff0808.
  const s = Math.hypot(x, y) / (r.width / 2);
  set((Math.atan2(x, -y) * 180) / Math.PI, s > 0.9 ? 1 : s);
}

function down(e: PointerEvent) {
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  pick(e);
}

function key(e: KeyboardEvent) {
  const { h, s } = hsv.value;
  const step = e.shiftKey ? 15 : 5;
  const moves: Record<string, [number, number]> = {
    ArrowRight: [h + step, s],
    ArrowLeft: [h - step, s],
    ArrowUp: [h, s + step / 100],
    ArrowDown: [h, s - step / 100],
  };
  const m = moves[e.key];
  if (!m) return;
  e.preventDefault();
  set(...m);
}

function typed() {
  const v = hex.value.trim().replace(/^#?/, "#").toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(v)) emit("update:modelValue", v);
  else hex.value = props.modelValue;
}

function toHsv(colour: string) {
  const n = parseInt(colour.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => c / 255);
  const max = Math.max(r, g, b);
  const d = max - Math.min(r, g, b);
  let h = 0;
  if (d) h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return { h: (h * 60 + 360) % 360, s: max ? d / max : 0, v: max };
}

function fromHsv({ h, s, v }: { h: number; s: number; v: number }) {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return Math.round(255 * (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))));
  };
  return "#" + [f(5), f(3), f(1)].map((c) => c.toString(16).padStart(2, "0")).join("");
}
</script>

<template>
  <div class="picker">
    <div
      ref="wheel"
      class="wheel"
      role="slider"
      tabindex="0"
      :aria-label="label"
      :aria-valuetext="modelValue"
      @pointerdown="down"
      @pointermove="(e) => e.buttons && pick(e)"
      @keydown="key"
    >
      <span class="thumb" :style="{ ...thumb, '--c': modelValue }" />
    </div>

    <div class="side">
      <div class="presets">
        <button
          v-for="p in PRESETS"
          :key="p"
          type="button"
          class="preset"
          :class="{ on: p === modelValue.toLowerCase() }"
          :style="{ '--c': p }"
          :aria-label="p"
          :title="p"
          @click="emit('update:modelValue', p)"
        />
      </div>
      <label class="hex">
        <span class="swatch" :style="{ '--c': modelValue }" />
        <input v-model="hex" class="num" spellcheck="false" maxlength="7" aria-label="Hex colour" @change="typed" />
      </label>
    </div>
  </div>
</template>

<style scoped>
.picker {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 16px 20px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--film);
}

.wheel {
  position: relative;
  flex: none;
  width: 168px;
  height: 168px;
  border-radius: 50%;
  background:
    radial-gradient(closest-side, #fff, rgb(255 255 255 / 0) 100%),
    conic-gradient(#f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
  box-shadow: 0 0 0 1px var(--line);
  cursor: crosshair;
  touch-action: none;
}

.wheel:focus-visible {
  outline: 2px solid var(--violet);
  outline-offset: 3px;
}

.thumb {
  position: absolute;
  width: 18px;
  height: 18px;
  border: 2px solid #fff;
  border-radius: 50%;
  background: var(--c);
  box-shadow: 0 1px 4px rgb(0 0 0 / 0.6);
  transform: translate(-50%, -50%);
  pointer-events: none;
}

.side {
  display: flex;
  flex: 1 1 140px;
  flex-direction: column;
  gap: 14px;
}

.presets {
  display: grid;
  grid-template-columns: repeat(auto-fill, 24px);
  gap: 8px;
}

.preset {
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--c);
  box-shadow: 0 0 0 1px var(--line);
  transition: transform 160ms var(--ease-out);
}

.preset:hover {
  transform: scale(1.1);
}

.preset.on {
  box-shadow:
    0 0 0 2px var(--ground),
    0 0 0 4px var(--ink);
}

.hex {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 10px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius-sm);
  background: var(--ground);
}

.hex .swatch {
  flex: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--c);
}

.hex input {
  width: 100%;
  min-width: 0;
  border: 0;
  background: transparent;
  color: var(--ink);
  font-size: 13px;
  text-transform: lowercase;
  outline: none;
}
</style>
