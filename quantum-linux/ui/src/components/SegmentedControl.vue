<script setup lang="ts">
// Mutually exclusive choices in one row. A filled pill marks the choice and
// slides to a new one as soon as it is clicked, while it is being applied; if
// the change fails the pending choice clears and the pill slides back.

import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = defineProps<{
  label: string;
  options: { value: string; label: string }[];
  value: string;
  /** The value being applied, if any. */
  pending?: string | null;
  disabled?: boolean;
}>();

const emit = defineEmits<{ select: [value: string] }>();

/**
 * What the pill sits on: the choice just clicked, until the confirmed value
 * catches up (the window learns of it on its next poll) or a failed change
 * has had a moment to be reported; else the confirmed choice.
 */
const held = ref<string | null>(null);
const shown = computed(() => held.value ?? props.pending ?? props.value);
let release: number | undefined;

function choose(value: string) {
  held.value = value;
  window.clearTimeout(release);
  release = window.setTimeout(() => (held.value = null), 3000);
  emit("select", value);
}

watch(
  () => props.value,
  (value) => {
    if (held.value === value) held.value = null;
  },
);

watch(
  () => props.pending,
  (now, before) => {
    // Finished without the value arriving (a failure): let go shortly.
    if (before && !now) {
      window.clearTimeout(release);
      release = window.setTimeout(() => (held.value = null), 400);
    }
  },
);

const root = ref<HTMLElement | null>(null);
const pill = ref<{ x: number; w: number } | null>(null);
const sliding = ref(false);

function place() {
  const el = root.value?.querySelector<HTMLElement>("button.on");
  pill.value = el ? { x: el.offsetLeft, w: el.offsetWidth } : null;
}

// Follows the window's width without animating.
const resized = new ResizeObserver(() => {
  sliding.value = false;
  place();
});

watch(
  () => [shown.value, props.options.length],
  async () => {
    sliding.value = true;
    await nextTick();
    place();
  },
);

onMounted(() => {
  if (root.value) resized.observe(root.value);
  place();
});
onBeforeUnmount(() => {
  resized.disconnect();
  window.clearTimeout(release);
});
</script>

<template>
  <div ref="root" class="segmented" role="radiogroup" :aria-label="label" :class="{ disabled }">
    <span
      v-if="pill"
      class="pill"
      :class="{ sliding }"
      aria-hidden="true"
      :style="{ width: `${pill.w}px`, transform: `translateX(${pill.x}px)` }"
    />
    <button
      v-for="o in options"
      :key="o.value"
      type="button"
      role="radio"
      :aria-checked="o.value === value"
      :class="{ on: o.value === shown, pending: o.value === pending }"
      :disabled="disabled"
      @click="o.value !== shown && choose(o.value)"
    >
      {{ o.label }}
    </button>
  </div>
</template>

<style scoped>
.segmented {
  position: relative;
  display: flex;
  gap: 2px;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: calc(var(--radius-sm) + 3px);
  background: var(--ground);
}

.pill {
  position: absolute;
  top: 3px;
  bottom: 3px;
  left: 0;
  border-radius: var(--radius-sm);
  background: var(--ink);
  pointer-events: none;
}

.pill.sliding {
  transition:
    transform 260ms var(--ease-out),
    width 260ms var(--ease-out);
}

.disabled .pill {
  background: var(--ink-2);
}

@media (prefers-reduced-motion: reduce) {
  .pill.sliding {
    transition: none;
  }
}

button {
  position: relative;
  z-index: 1;
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
    color 260ms var(--ease-out);
}

button:hover:not(:disabled):not(.on) {
  background: var(--film-raised);
  color: var(--ink);
}

button.on {
  color: var(--ground);
  font-weight: 600;
}

.disabled button:not(.on) {
  color: var(--ink-3);
}

button:disabled {
  cursor: default;
}
</style>
