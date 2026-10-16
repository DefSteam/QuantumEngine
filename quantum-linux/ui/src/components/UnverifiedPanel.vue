<script setup lang="ts">
// A headset from the same vendor that QuantumEngine isn't verified on. Nothing
// is sent to it until the user says so: first it can be checked with reads only,
// then used read-only, then given writes one feature at a time.

import { computed, ref } from "vue";
import { Copy } from "lucide-vue-next";
import { useApp } from "../stores/app";
import PanelField from "./PanelField.vue";
import ToggleSwitch from "./ToggleSwitch.vue";

const app = useApp();

const u = computed(() => app.state.unverified);
const busy = computed(() => app.pending !== null);
const probing = computed(() => app.pending === "unverified:probe");
const asking = computed(() => u.value.access.startsWith("Waiting") || app.pending === "unverified:grant");
const copied = ref(false);

const FEATURES = [
  { id: "anc", label: "Noise control", hint: "Off, ANC and TalkThru." },
  { id: "sidetone", label: "Sidetone", hint: "How much of your own voice you hear." },
  { id: "lights", label: "Lights", hint: "On and off, colours and effects." },
];

function allow(feature: string, on: boolean) {
  const next = new Set(u.value.allowed);
  if (on) next.add(feature);
  else next.delete(feature);
  void app.setUnverified(u.value.inUse, [...next]);
}

async function copyReport() {
  try {
    await navigator.clipboard.writeText(app.probeText);
    copied.value = true;
    window.setTimeout(() => (copied.value = false), 1800);
  } catch {
    app.toast("Couldn't copy. Select the report and copy it by hand.");
  }
}
</script>

<template>
  <PanelField title="Unverified headset">
    <p class="lead">
      QuantumEngine is verified on the JBL Quantum 810 Wireless only. This model might work, partly work or not work
      at all, and nothing is sent to it until you choose.
    </p>
    <p class="who">
      {{ u.name || "Unknown headset" }}
      <span class="num id">{{ u.id }}</span>
    </p>

    <div class="step">
      <h4>1. Check what it answers</h4>
      <p class="note">
        This only reads: battery, noise control, sidetone and so on, the same questions the 810 answers. It writes
        nothing to the headset.
      </p>
      <div class="row">
        <button type="button" class="secondary" :disabled="busy" @click="app.probeUnverified">
          {{ probing ? "Checking…" : app.probeText ? "Check again" : "Check it" }}
        </button>
        <button
          type="button"
          class="secondary"
          :disabled="busy || asking"
          title="Needs your password. Adds a rule for this one device."
          @click="app.grantAccess"
        >
          {{ asking ? "Waiting for your password…" : "Allow access…" }}
        </button>
      </div>
      <p v-if="u.access && !asking" class="problem" role="status">{{ u.access }}</p>
      <template v-if="app.probeText">
        <pre class="report" tabindex="0" aria-label="Compatibility report">{{ app.probeText }}</pre>
        <div class="row">
          <button type="button" class="secondary" @click="copyReport">
            <Copy :size="14" :stroke-width="1.75" />{{ copied ? "Copied" : "Copy report" }}
          </button>
          <span class="note">Paste it in a GitHub issue to help us support this model.</span>
        </div>
      </template>
      <p v-else class="note">If it says it couldn't open the device, use Allow access first.</p>
    </div>

    <div class="step">
      <div class="switch-row">
        <h4>2. Use this headset</h4>
        <ToggleSwitch
          label="Use this unverified headset"
          :checked="u.inUse"
          :pending="app.pending === 'unverified:set'"
          :disabled="busy"
          @toggle="app.setUnverified($event, u.allowed)"
        />
      </div>
      <p class="note">
        On, QuantumEngine shows what the headset reports, read-only. Anything the headset doesn't answer is left
        blank.
      </p>
    </div>

    <div class="step" :class="{ off: !u.inUse }">
      <h4>3. Allow changes</h4>
      <p class="note">
        Each one sends commands built for the 810. Turn on only what you are happy to try; they may not behave on
        this model.
      </p>
      <div v-for="f in FEATURES" :key="f.id" class="switch-row feature">
        <span>
          {{ f.label }}
          <span class="note">{{ f.hint }}</span>
        </span>
        <ToggleSwitch
          :label="`Allow changing ${f.label.toLowerCase()}`"
          :checked="u.allowed.includes(f.id)"
          :disabled="busy || !u.inUse"
          @toggle="allow(f.id, $event)"
        />
      </div>
    </div>
  </PanelField>
</template>

<style scoped>
.lead {
  margin: 0 0 12px;
  font-size: 13px;
  color: var(--ink-2);
  text-wrap: pretty;
}

.who {
  margin: 0 0 14px;
  font-weight: 600;
}

.id {
  margin-left: 8px;
  font-size: 12px;
  font-weight: 400;
  color: var(--ink-3);
}

.step {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 0;
  border-top: 1px solid var(--line);
}

.step.off {
  opacity: 0.55;
}

h4 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.note {
  margin: 0;
  font-size: 12px;
  color: var(--ink-3);
  text-wrap: pretty;
}

.row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 12px;
}

.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.feature {
  padding: 6px 0;
  font-size: 13px;
}

.feature .note {
  display: block;
}

.problem {
  margin: 0;
  font-size: 13px;
  color: var(--alert);
}

.report {
  margin: 0;
  max-height: 240px;
  padding: 12px;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--ground);
  font: 12px/1.5 ui-monospace, "JetBrains Mono", monospace;
  color: var(--ink-2);
  white-space: pre;
}
</style>
