<script setup lang="ts">
// Making or changing one of the user's own EQ profiles. The bands are the
// draft in the store; the EQ panel draws them on the curve (with draggable
// handles) and plays them live while the EQ is on.

import { computed, ref } from "vue";
import { Plus, Trash2, X } from "lucide-vue-next";
import { useApp } from "../stores/app";
import type { Band } from "../api";

const app = useApp();
const selected = defineModel<number | null>("selected", { default: null });

const TYPES = [
  { value: "bell", label: "Bell" },
  { value: "lowshelf", label: "Low shelf" },
  { value: "highshelf", label: "High shelf" },
  { value: "highpass", label: "High-pass" },
  { value: "lowpass", label: "Low-pass" },
];

const e = computed(() => app.editor!);
const full = computed(() => e.value.bands.length >= app.info.eqMaxBands);
const confirmDelete = ref(false);
const busy = computed(() => app.pending !== null);

/** What's wrong with a band, as the service would say it, or "". */
function problem(b: Band): string {
  if (!Number.isFinite(b.freq) || b.freq < 10 || b.freq > 20000) return "Frequency: 10 Hz to 20 kHz.";
  if (!Number.isFinite(b.gain) || b.gain < -30 || b.gain > 30) return "Gain: -30 to +30 dB.";
  if (!Number.isFinite(b.q) || b.q < 0.05 || b.q > 20) return "Q: 0.05 to 20.";
  return "";
}
const valid = computed(() => e.value.bands.every((b) => !problem(b)) && e.value.name.trim() !== "");

function hasGain(kind: string) {
  return kind !== "highpass" && kind !== "lowpass";
}

function add() {
  if (full.value) return;
  e.value.bands.push({ kind: "bell", freq: 1000, gain: 0, q: 1 });
  selected.value = e.value.bands.length - 1;
}

function remove(i: number) {
  e.value.bands.splice(i, 1);
  selected.value = null;
}
</script>

<template>
  <section class="editor" aria-label="Profile editor">
    <div class="head">
      <h3>{{ e.original ? "Edit profile" : "New profile" }}</h3>
      <button type="button" class="icon" aria-label="Close without saving" title="Close without saving" @click="app.closeEditor">
        <X :size="16" :stroke-width="1.75" />
      </button>
    </div>

    <label class="name">
      <span>Name</span>
      <input v-model="e.name" type="text" maxlength="40" spellcheck="false" />
    </label>

    <div class="bands" role="table" aria-label="Bands">
      <div class="row header" role="row">
        <span role="columnheader">#</span>
        <span role="columnheader">Type</span>
        <span role="columnheader">Hz</span>
        <span role="columnheader">dB</span>
        <span role="columnheader">Q</span>
        <span />
      </div>
      <div
        v-for="(b, i) in e.bands"
        :key="i"
        class="row"
        :class="{ on: selected === i, bad: !!problem(b) }"
        role="row"
        @focusin="selected = i"
        @click="selected = i"
      >
        <span class="index num">{{ i + 1 }}</span>
        <select v-model="b.kind" :aria-label="`Band ${i + 1} type`">
          <option v-for="t in TYPES" :key="t.value" :value="t.value">{{ t.label }}</option>
        </select>
        <input v-model.number="b.freq" type="number" min="10" max="20000" step="1" :aria-label="`Band ${i + 1} frequency`" />
        <input
          v-model.number="b.gain"
          type="number"
          min="-30"
          max="30"
          step="0.1"
          :disabled="!hasGain(b.kind)"
          :aria-label="`Band ${i + 1} gain`"
        />
        <input v-model.number="b.q" type="number" min="0.05" max="20" step="0.05" :aria-label="`Band ${i + 1} Q`" />
        <button type="button" class="icon" :aria-label="`Remove band ${i + 1}`" title="Remove band" @click.stop="remove(i)">
          <Trash2 :size="14" :stroke-width="1.75" />
        </button>
        <p v-if="problem(b)" class="problem">{{ problem(b) }}</p>
      </div>
      <p v-if="!e.bands.length" class="empty">No bands: this profile leaves the sound as it is.</p>
    </div>

    <div class="add-row">
      <button type="button" class="secondary small" :disabled="full" @click="add">
        <Plus :size="14" :stroke-width="1.75" />Add band
      </button>
      <span class="count num">{{ e.bands.length }} of {{ app.info.eqMaxBands }}</span>
    </div>

    <p class="note">
      {{
        app.state.eq.enabled && app.serviceUp
          ? app.state.eq.previewing
            ? "You're hearing your changes. Cancel goes back to how it was."
            : "Changes play as you make them."
          : "Turn the equalizer on to hear your changes as you make them."
      }}
      Drag the numbered points on the graph to move a band.
    </p>

    <div class="actions">
      <button type="button" class="primary" :disabled="!valid || busy" @click="app.saveEditor">
        {{ app.pending === "eqedit:save" ? "Saving…" : "Save and use" }}
      </button>
      <button type="button" class="link-button" @click="app.closeEditor">Cancel</button>
      <span class="spacer" />
      <template v-if="e.original">
        <template v-if="confirmDelete">
          <span class="confirm">Delete “{{ e.original }}”?</span>
          <button type="button" class="link-button danger" :disabled="busy" @click="app.deleteProfile(e.original)">Delete</button>
          <button type="button" class="link-button" @click="confirmDelete = false">Keep</button>
        </template>
        <button v-else type="button" class="link-button" @click="confirmDelete = true">Delete profile</button>
      </template>
    </div>
  </section>
</template>

<style scoped>
.editor {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.icon {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--ink-3);
}

.icon:hover {
  background: var(--film-raised);
  color: var(--ink);
}

.name {
  display: grid;
  grid-template-columns: 60px 1fr;
  align-items: center;
  gap: 12px;
  font-size: 13px;
  color: var(--ink-3);
}

input,
select {
  width: 100%;
  min-width: 0;
  height: 32px;
  padding: 0 8px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius-sm);
  background: var(--ground);
  font-variant-numeric: tabular-nums;
}

input:hover,
select:hover {
  border-color: var(--ink-3);
}

input:disabled {
  opacity: 0.35;
}

.name input {
  height: 34px;
  font-size: 14px;
}

.bands {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 18px minmax(92px, 1.4fr) minmax(62px, 1fr) minmax(54px, 0.9fr) minmax(52px, 0.8fr) 28px;
  align-items: center;
  gap: 6px;
  padding: 3px 4px;
  border-radius: var(--radius-sm);
}

.row.header {
  padding-top: 0;
  padding-bottom: 0;
  font-size: 12px;
  color: var(--ink-3);
}

.row.on {
  background: var(--violet-soft);
}

.row.bad input {
  border-color: var(--alert);
}

.index {
  font-size: 12px;
  color: var(--ink-3);
  text-align: center;
}

.row.on .index {
  color: var(--ink);
}

.problem {
  grid-column: 2 / -1;
  margin: 0;
  font-size: 12px;
  color: var(--alert);
}

.empty,
.note {
  margin: 0;
  font-size: 12px;
  color: var(--ink-3);
  text-wrap: pretty;
}

.add-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.small {
  height: 32px;
  padding: 0 12px;
  font-size: 13px;
}

.count {
  font-size: 12px;
  color: var(--ink-3);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 16px;
  padding-top: 4px;
}

.spacer {
  flex: 1;
}

.confirm {
  font-size: 13px;
  color: var(--ink-2);
}

.danger {
  color: var(--alert);
  text-decoration-color: currentColor;
}
</style>
