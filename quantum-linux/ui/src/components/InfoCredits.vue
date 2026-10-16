<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { Info, X } from "lucide-vue-next";

const open = ref(false);
const root = ref<HTMLElement | null>(null);
const trigger = ref<HTMLButtonElement | null>(null);

function onPointerDown(event: PointerEvent) {
  if (open.value && root.value && !root.value.contains(event.target as Node)) open.value = false;
}

function onKeyDown(event: KeyboardEvent) {
  if (event.key !== "Escape" || !open.value) return;
  closePopup();
}

function closePopup() {
  open.value = false;
  trigger.value?.focus();
}

onMounted(() => {
  document.addEventListener("pointerdown", onPointerDown);
  document.addEventListener("keydown", onKeyDown);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onPointerDown);
  document.removeEventListener("keydown", onKeyDown);
});
</script>

<template>
  <div ref="root" class="info-root">
    <section
      v-show="open"
      id="app-credits"
      class="info-popup"
      role="dialog"
      aria-modal="false"
      aria-labelledby="credits-title"
    >
      <header>
        <h2 id="credits-title">About QuantumEngine</h2>
        <button class="close" type="button" aria-label="Close credits" @click="closePopup">
          <X :size="16" :stroke-width="1.8" />
        </button>
      </header>
      <p>QuantumEngine was created by <strong>sidmax7</strong>.</p>
      <p>EQ profiles are based on JBL QuantumENGINE's configuration, extracted by <strong>alessandromrc</strong> and tested by <strong>DefSteam</strong>.</p>
      <p>User interface design and application improvements by <strong>DefSteam</strong>.</p>
    </section>
    <button
      ref="trigger"
      class="info-button"
      type="button"
      aria-label="About QuantumEngine and credits"
      aria-haspopup="dialog"
      aria-controls="app-credits"
      :aria-expanded="open"
      title="About and credits"
      @click="open = !open"
    >
      <Info :size="19" :stroke-width="1.7" />
    </button>
  </div>
</template>

<style scoped>
.info-root {
  position: fixed;
  right: max(16px, env(safe-area-inset-right));
  bottom: max(16px, env(safe-area-inset-bottom));
  z-index: 20;
}

.info-button,
.close {
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 1px solid var(--line-strong);
  background: var(--film-raised);
  color: var(--ink-2);
  transition: color 160ms, border-color 160ms, background 160ms;
}

.info-button {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  box-shadow: 0 4px 18px rgb(0 0 0 / 0.28);
}

.info-button:hover,
.close:hover {
  border-color: var(--ink-2);
  background: #25282e;
  color: var(--ink);
}

.info-popup {
  position: absolute;
  right: 0;
  bottom: calc(100% + 10px);
  width: min(340px, calc(100vw - 32px));
  padding: 16px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius);
  background: #191b20;
  box-shadow: 0 12px 40px rgb(0 0 0 / 0.48);
}

.info-popup header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.close {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
}

p {
  margin: 0;
  color: var(--ink-2);
  font-size: 12px;
  line-height: 1.55;
  text-wrap: pretty;
}

p + p {
  margin-top: 9px;
  padding-top: 9px;
  border-top: 1px solid var(--line);
}

strong {
  color: var(--ink);
  font-weight: 600;
}
</style>
