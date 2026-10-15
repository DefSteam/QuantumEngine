<script setup lang="ts">
// Failures, shown for a few seconds at the bottom of the window.

import { CircleAlert, X } from "lucide-vue-next";
import { useApp } from "../stores/app";

const app = useApp();
</script>

<template>
  <div class="toasts" aria-live="assertive">
    <TransitionGroup name="toast">
      <div v-for="t in app.toasts" :key="t.id" class="toast" role="alert">
        <CircleAlert class="icon" :size="16" :stroke-width="1.75" />
        <span class="message">{{ t.message }}</span>
        <button type="button" class="close" aria-label="Dismiss" @click="app.dismiss(t.id)">
          <X :size="15" :stroke-width="1.75" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toasts {
  position: fixed;
  right: var(--gutter);
  bottom: var(--gutter);
  left: var(--gutter);
  z-index: 10;
  display: flex;
  flex-direction: column;
  gap: 8px;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 11px 10px 11px 12px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius);
  background: var(--film-raised);
  box-shadow: 0 10px 30px -8px rgb(0 0 0 / 0.6);
  pointer-events: auto;
}

.icon {
  flex: none;
  margin-top: 1px;
  color: var(--alert);
}

.message {
  flex: 1;
  font-size: 13px;
  text-wrap: pretty;
}

.close {
  display: grid;
  place-items: center;
  flex: none;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--ink-3);
}

.close:hover {
  color: var(--ink);
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 220ms var(--ease-out),
    transform 220ms var(--ease-out);
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
