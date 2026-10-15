<script setup lang="ts">
// Mutually exclusive choices in one row. The confirmed value is filled with
// ink; the one being applied shows a moving rule until it's confirmed.

defineProps<{
  label: string;
  options: { value: string; label: string }[];
  value: string;
  /** The value being applied, if any. */
  pending?: string | null;
  disabled?: boolean;
}>();

const emit = defineEmits<{ select: [value: string] }>();
</script>

<template>
  <div class="segmented" role="radiogroup" :aria-label="label" :class="{ disabled }">
    <button
      v-for="o in options"
      :key="o.value"
      type="button"
      role="radio"
      :aria-checked="o.value === value"
      :class="{ on: o.value === value, pending: o.value === pending }"
      :disabled="disabled"
      @click="o.value !== value && emit('select', o.value)"
    >
      {{ o.label }}
    </button>
  </div>
</template>

<style scoped>
.segmented {
  display: flex;
  gap: 2px;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: calc(var(--radius-sm) + 3px);
  background: var(--ground);
}

button {
  position: relative;
  flex: 1 1 0;
  min-width: 0;
  height: 32px;
  padding: 0 10px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--ink-2);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition:
    background 180ms var(--ease-out),
    color 180ms var(--ease-out);
}

button:hover:not(:disabled):not(.on) {
  background: var(--film-raised);
  color: var(--ink);
}

button.on {
  background: var(--ink);
  color: var(--ground);
  font-weight: 600;
}

button.pending {
  color: var(--ink);
}

button.pending::after {
  content: "";
  position: absolute;
  left: 10%;
  bottom: 4px;
  width: 30%;
  height: 2px;
  border-radius: 2px;
  background: var(--violet);
  animation: sweep 900ms var(--ease-out) infinite alternate;
}

@keyframes sweep {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(133%);
  }
}

.disabled button:not(.on):not(.pending) {
  color: var(--ink-3);
}

.disabled button.on {
  background: var(--ink-2);
}

button:disabled {
  cursor: default;
}
</style>
