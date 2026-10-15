<script setup lang="ts">
// An on/off switch. Shows `checked` (the confirmed state); while `pending`,
// the knob travels to the requested side and waits there.

const props = defineProps<{
  label: string;
  checked: boolean;
  pending?: boolean;
  disabled?: boolean;
}>();

const emit = defineEmits<{ toggle: [on: boolean] }>();
</script>

<template>
  <button
    type="button"
    role="switch"
    class="switch"
    :class="{ on: props.pending ? !checked : checked, pending }"
    :aria-checked="checked"
    :aria-label="label"
    :disabled="disabled"
    @click="emit('toggle', !checked)"
  >
    <span class="knob" />
  </button>
</template>

<style scoped>
.switch {
  position: relative;
  flex: none;
  width: 44px;
  height: 26px;
  padding: 0;
  border: 1px solid var(--line-strong);
  border-radius: 13px;
  background: var(--ground);
  transition:
    background 200ms var(--ease-out),
    border-color 200ms var(--ease-out);
}

.knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--ink-3);
  transition:
    transform 260ms var(--ease-out),
    background 200ms;
}

.switch.on {
  border-color: var(--ink);
  background: var(--ink);
}

.switch.on .knob {
  transform: translateX(18px);
  background: var(--ground);
}

.switch.pending .knob {
  box-shadow: 0 0 0 3px var(--violet);
  animation: wait 900ms ease-in-out infinite alternate;
}

@keyframes wait {
  to {
    box-shadow: 0 0 0 1px var(--violet);
  }
}

.switch:hover:not(:disabled):not(.on) {
  border-color: var(--ink-3);
}

.switch:disabled {
  opacity: 0.45;
}
</style>
