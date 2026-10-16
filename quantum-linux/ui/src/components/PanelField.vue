<script setup lang="ts">
// One setting: its name on the left, its current reading right-aligned
// against the margin, the control below, and a line on what it does.

defineProps<{
  title: string;
  reading?: string;
  hint?: string;
}>();
</script>

<template>
  <section class="field">
    <div class="head">
      <h3>{{ title }}</h3>
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
  margin-top: 24px;
  padding-top: 18px;
  border-top: 1px solid var(--line);
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
  letter-spacing: 0.005em;
  font-weight: 600;
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
