<script setup lang="ts">
// An older background service still running, a newer release, or a service
// that's already newer than this window.

import { computed } from "vue";
import { api } from "../api";
import { useApp } from "../stores/app";

const app = useApp();
const u = computed(() => app.state.update);

const restarting = computed(() => app.pending === "app:restart");
const installing = computed(() => app.pending === "update:install" || (u.value.busy && !!u.value.available));
/** The service updated itself but this window is still the old one. */
const stale = computed(() => !app.state.serviceOutdated && !!u.value.serviceVersion && !!app.info.version && u.value.serviceVersion !== app.info.version);

async function whatsNew() {
  try {
    await api.openReleasePage();
  } catch (e) {
    app.toast(e instanceof Error ? e.message : String(e));
  }
}
</script>

<template>
  <div v-if="app.serviceUp && (app.state.serviceOutdated || stale)" class="strip" role="status">
    <div class="text">
      <strong>Restart QuantumEngine to finish updating</strong>
      <span class="sub">
        The update is installed. Restart the app to load the new version.
      </span>
    </div>
    <div class="buttons">
      <button type="button" class="primary" :disabled="app.pending !== null" @click="app.restartApp">
        {{ restarting ? "Restarting…" : "Restart app" }}
      </button>
    </div>
  </div>
  <div v-else-if="u.available" class="strip" role="status">
    <div class="text">
      <strong>QuantumEngine {{ u.available }} is available</strong>
      <span class="sub">
        {{
          u.status ||
          (u.canInstall
            ? "Installs with pacman after asking for your password."
            : "Update it with your package manager, or rebuild it with makepkg.")
        }}
      </span>
    </div>
    <div class="buttons">
      <button v-if="u.canInstall" type="button" class="primary" :disabled="installing || app.pending !== null" @click="app.installUpdate">
        {{ installing ? "Installing…" : "Install" }}
      </button>
      <button type="button" class="link-button" @click="whatsNew">What's new</button>
    </div>
  </div>
</template>

<style scoped>
.strip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 16px;
  padding: 12px 14px;
  border: 1px solid var(--line-strong);
  border-radius: var(--radius);
  background: var(--film-raised);
}

.strip.quiet {
  border-color: var(--line-strong);
  background: transparent;
}

.text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

strong {
  font-weight: 600;
}

.sub {
  font-size: 12px;
  color: var(--ink-2);
}

.buttons {
  display: flex;
  align-items: center;
  gap: 14px;
}

.primary {
  height: 32px;
}
</style>
