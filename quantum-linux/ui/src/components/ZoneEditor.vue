<script setup lang="ts">
// One lighting zone's colours, effect and brightness, edited in the draft.
// The figure repaints as you go; nothing reaches the headset until Apply.

import { computed } from "vue";
import { Minus, Plus } from "lucide-vue-next";
import { useApp, type ZoneName } from "../stores/app";

const props = defineProps<{ zone: ZoneName; title: string }>();
const app = useApp();

const z = computed(() => app.draft?.[props.zone]);
const effectLabel = computed(() => app.info.effects.find((e) => e.id === z.value?.effect)?.label ?? "Custom effect");

function setColour(i: number, value: string) {
  if (z.value) z.value.colours[i] = value;
}

function addColour() {
  const zone = z.value;
  if (!zone || zone.colours.length >= app.info.maxColours) return;
  zone.colours.push(zone.colours[zone.colours.length - 1] ?? "#ffffff");
}

function removeColour() {
  const zone = z.value;
  if (zone && zone.colours.length > 1) zone.colours.pop();
}
</script>

<template>
  <section v-if="z" class="zone">
    <div class="head">
      <h3>{{ title }}</h3>
      <span class="reading num">{{ effectLabel }} · {{ z.brightness }}%</span>
    </div>

    <div class="row">
      <span class="name">Colours</span>
      <div class="swatches">
        <label v-for="(c, i) in z.colours" :key="i" class="swatch" :style="{ '--c': c }" :title="c">
          <input
            type="color"
            :value="c"
            :aria-label="`${title} colour ${i + 1}`"
            @input="setColour(i, ($event.target as HTMLInputElement).value)"
          />
        </label>
        <button
          v-if="z.colours.length > 1"
          type="button"
          class="icon-button"
          :aria-label="`Remove the last ${title.toLowerCase()} colour`"
          title="Remove the last colour"
          @click="removeColour"
        >
          <Minus :size="15" :stroke-width="1.75" />
        </button>
        <button
          v-if="z.colours.length < app.info.maxColours"
          type="button"
          class="icon-button"
          :aria-label="`Add a ${title.toLowerCase()} colour`"
          title="Add a colour to cycle through"
          @click="addColour"
        >
          <Plus :size="15" :stroke-width="1.75" />
        </button>
      </div>
    </div>

    <div class="row">
      <label class="name" :for="`${zone}-effect`">Effect</label>
      <div class="select-wrap">
        <select :id="`${zone}-effect`" v-model.number="z.effect">
          <option v-for="e in app.info.effects" :key="e.id" :value="e.id">{{ e.label }}</option>
          <option v-if="!app.info.effects.some((e) => e.id === z!.effect)" :value="z.effect">Custom effect</option>
        </select>
      </div>
    </div>

    <div class="row">
      <label class="name" :for="`${zone}-brightness`">Brightness</label>
      <input
        :id="`${zone}-brightness`"
        v-model.number="z.brightness"
        class="range"
        type="range"
        min="0"
        max="100"
        step="5"
        :style="{ '--fill': `${z.brightness}%` }"
      />
    </div>
  </section>
</template>

<style scoped>
.zone {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px 0 16px;
  border-top: 1px solid var(--line);
}

.head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.reading {
  font-size: 13px;
  color: var(--ink-2);
}

.row {
  display: grid;
  grid-template-columns: 84px 1fr;
  align-items: center;
  gap: 12px;
  min-height: 34px;
}

.name {
  font-size: 13px;
  color: var(--ink-3);
}

.swatches {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.swatch {
  position: relative;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--c);
  box-shadow:
    inset 0 0 0 1px rgb(255 255 255 / 0.18),
    0 0 0 1px var(--line);
  cursor: pointer;
  transition: transform 160ms var(--ease-out);
}

.swatch:hover {
  transform: scale(1.08);
}

.swatch:focus-within {
  outline: 2px solid var(--violet);
  outline-offset: 2px;
}

.swatch input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  opacity: 0;
  cursor: pointer;
}

.icon-button {
  display: inline-grid;
  place-items: center;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 1px dashed var(--line-strong);
  border-radius: 50%;
  background: transparent;
  color: var(--ink-2);
  transition:
    border-color 160ms,
    color 160ms;
}

.icon-button:hover {
  border-color: var(--ink-2);
  color: var(--ink);
}

.select-wrap {
  position: relative;
}

.select-wrap::after {
  content: "";
  position: absolute;
  top: 50%;
  right: 13px;
  width: 6px;
  height: 6px;
  border-right: 1.5px solid var(--ink-2);
  border-bottom: 1.5px solid var(--ink-2);
  transform: translateY(-70%) rotate(45deg);
  pointer-events: none;
}

select {
  width: 100%;
  height: 34px;
  padding: 0 34px 0 11px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius-sm);
  background: var(--ground);
  appearance: none;
  cursor: pointer;
}

select:hover {
  border-color: var(--ink-3);
}

.range {
  width: 100%;
  height: 20px;
  margin: 0;
  background: transparent;
  appearance: none;
  cursor: pointer;
}

.range::-webkit-slider-runnable-track {
  height: 3px;
  border-radius: 3px;
  background: linear-gradient(to right, var(--ink) var(--fill), var(--line-strong) var(--fill));
}

.range::-webkit-slider-thumb {
  width: 16px;
  height: 16px;
  margin-top: -6.5px;
  border-radius: 50%;
  background: var(--ink);
  box-shadow: 0 1px 3px rgb(0 0 0 / 0.5);
  appearance: none;
}

.range::-moz-range-track {
  height: 3px;
  border-radius: 3px;
  background: linear-gradient(to right, var(--ink) var(--fill), var(--line-strong) var(--fill));
}

.range::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border: 0;
  border-radius: 50%;
  background: var(--ink);
}

.range:focus-visible {
  outline: none;
}

.range:focus-visible::-webkit-slider-thumb {
  outline: 2px solid var(--violet);
  outline-offset: 2px;
}
</style>
