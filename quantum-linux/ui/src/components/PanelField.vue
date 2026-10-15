<script setup lang="ts">
// One setting: its name on the left, its current reading right-aligned
// against the margin, the control below, and a line on what it does.

defineProps<{
  title: string;
  reading?: string;
  hint?: string;
  /** The part number of this setting on Fig. 1, when it has one. */
  part?: number;
}>();
</script>

<template>
  <section class="field">
    <div class="head">
      <h3>
        <span v-if="part" class="key num" :aria-label="`Part ${part}`">{{ part }}</span>{{ title }}
      </h3>
      <slot name="aside">
        <span v-if="reading" class="reading num">{{ reading }}</span>
      </slot>
    </div>
    <slot />
    <p v-if="hint" class="hint">{{ hint }}</p>
  </section>
</template>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.field + .field {
  margin-top: 26px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 26px;
}

h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

/* Matches the part numbers on Fig. 1's callouts. */
.key {
  display: inline-grid;
  place-items: center;
  width: 17px;
  height: 17px;
  border: 1px solid var(--line-strong);
  border-radius: 50%;
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  color: var(--ink-2);
}

.reading {
  font-size: 13px;
  color: var(--ink-2);
}

.hint {
  margin: -2px 0 0;
  font-size: 12px;
  color: var(--ink-3);
  text-wrap: pretty;
}
</style>
