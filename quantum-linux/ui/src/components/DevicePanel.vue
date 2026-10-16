<script setup lang="ts">
// The parts list: what the headset says about itself, the versions, and
// the update settings.

import { computed } from "vue";
import { ExternalLink } from "lucide-vue-next";
import { api } from "../api";
import { useApp } from "../stores/app";
import PanelField from "./PanelField.vue";
import UnverifiedPanel from "./UnverifiedPanel.vue";
import ToggleSwitch from "./ToggleSwitch.vue";

const app = useApp();

const h = computed(() => app.headset);
const u = computed(() => app.state.update);
const known = computed(() => app.ready || h.value.serial !== "");

const rows = computed(() => [
  { name: "Paired as", value: known.value ? h.value.deviceName || "—" : "—" },
  { name: "Serial", value: known.value ? h.value.serial || "—" : "—" },
  { name: "Firmware", value: known.value && h.value.firmware.length ? h.value.firmware.join(" · ") : "—" },
  { name: "Window", value: app.info.version || "—" },
  { name: "Service", value: u.value.serviceVersion || "—" },
]);

const checking = computed(() => app.pending === "update:check" || (u.value.busy && !u.value.available));
const updateLine = computed(() => {
  if (u.value.available) return `QuantumEngine ${u.value.available} is available.`;
  return u.value.status || "";
});

async function openProject() {
  try {
    await api.openProjectPage();
  } catch (e) {
    app.toast(e instanceof Error ? e.message : String(e));
  }
}
</script>

<template>
  <div class="panel">
    <UnverifiedPanel v-if="app.state.unverified.id" />
    <PanelField title="Headset">
      <dl class="parts">
        <div v-for="r in rows" :key="r.name" class="part">
          <dt>{{ r.name }}</dt>
          <dd class="num">{{ r.value }}</dd>
        </div>
      </dl>
      <p v-if="known && h.firmware.length" class="note">
        The headset reports {{ h.firmware.length }} firmware versions without saying which part of it each one
        belongs to, so they're listed in the order it sends them.
      </p>
    </PanelField>

    <PanelField
      title="Updates"
      hint="Checks GitHub for a new release once a day. That one request is the only network traffic QuantumEngine makes."
    >
      <template #aside>
        <ToggleSwitch
          label="Check GitHub for updates once a day"
          :checked="u.checks"
          :pending="app.pending?.startsWith('update:checks:') ?? false"
          :disabled="!app.serviceUp || app.pending !== null"
          @toggle="app.setUpdateChecks"
        />
      </template>
      <div class="update-row">
        <button type="button" class="secondary" :disabled="!app.serviceUp || checking || app.pending !== null" @click="app.checkForUpdates">
          {{ checking ? "Checking…" : "Check now" }}
        </button>
        <span class="update-line" role="status">{{ updateLine }}</span>
      </div>
    </PanelField>

    <section class="about">
      <p>
        QuantumEngine is an unofficial project, not affiliated with JBL or Harman. The headset protocol was
        worked out from the device itself.
      </p>
      <button type="button" class="link-button" @click="openProject">
        Project page <ExternalLink :size="13" :stroke-width="1.75" />
      </button>
    </section>
  </div>
</template>

<style scoped>
.parts {
  margin: 0;
}

.part {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 16px;
  padding: 9px 0;
  border-bottom: 1px solid var(--line);
}

.part:first-child {
  border-top: 1px solid var(--line);
}

dt {
  color: var(--ink-3);
  font-size: 13px;
}

dd {
  margin: 0;
  text-align: right;
  font-size: 13px;
  overflow-wrap: anywhere;
}

.note {
  margin: 0;
  font-size: 12px;
  color: var(--ink-3);
  text-wrap: pretty;
}

.update-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 14px;
}

.update-line {
  font-size: 13px;
  color: var(--ink-2);
}

.about {
  margin-top: 30px;
  padding-top: 16px;
  border-top: 1px solid var(--line);
  font-size: 12px;
  color: var(--ink-3);
}

.about p {
  margin: 0 0 8px;
  max-width: 60ch;
  text-wrap: pretty;
}

.about .link-button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
}
</style>
