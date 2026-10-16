<script setup lang="ts">
import { computed, onMounted } from "vue";
import { CircleAlert } from "lucide-vue-next";
import { useApp, type Section } from "./stores/app";
import DevicePanel from "./components/DevicePanel.vue";
import EqCurveDock from "./components/EqCurveDock.vue";
import EqPanel from "./components/EqPanel.vue";
import InfoCredits from "./components/InfoCredits.vue";
import HeadsetFigure from "./components/HeadsetFigure.vue";
import LightsPanel from "./components/LightsPanel.vue";
import SoundPanel from "./components/SoundPanel.vue";
import StatusHeader from "./components/StatusHeader.vue";
import ToastHost from "./components/ToastHost.vue";
import UpdateStrip from "./components/UpdateStrip.vue";

const app = useApp();

const SECTIONS: { id: Section; label: string }[] = [
  { id: "sound", label: "Sound" },
  { id: "lights", label: "Lights" },
  { id: "eq", label: "EQ" },
  { id: "device", label: "Device" },
];

const panels = { sound: SoundPanel, lights: LightsPanel, eq: EqPanel, device: DevicePanel };

/** The service can't be reached; it keeps being retried in the background. */
const serviceDown = computed(() => app.state.service === "down");

/** The headset's last failure, where it matters. */
const headsetError = computed(() =>
  app.ready && (app.section === "sound" || app.section === "lights") ? app.headset.lastError : "",
);

function onTabKey(e: KeyboardEvent) {
  const i = SECTIONS.findIndex((s) => s.id === app.section);
  const step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
  if (!step) return;
  e.preventDefault();
  const next = SECTIONS[(i + step + SECTIONS.length) % SECTIONS.length]!;
  app.section = next.id;
  document.getElementById(`tab-${next.id}`)?.focus();
}

onMounted(() => {
  app.start().catch((e) => app.toast(e instanceof Error ? e.message : String(e)));
});
</script>

<template>
  <div class="layout" :class="{ 'eq-open': app.section === 'eq' }">
    <div class="overview">
      <UpdateStrip />
      <StatusHeader />
      <HeadsetFigure />
      <EqCurveDock />
    </div>

    <main class="work">
      <nav class="tabs" role="tablist" aria-label="Sections" @keydown="onTabKey">
        <button
          v-for="s in SECTIONS"
          :id="`tab-${s.id}`"
          :key="s.id"
          type="button"
          role="tab"
          :aria-selected="app.section === s.id"
          :aria-controls="`panel-${s.id}`"
          :tabindex="app.section === s.id ? 0 : -1"
          :class="{ on: app.section === s.id }"
          @click="app.section = s.id"
        >
          {{ s.label }}
        </button>
      </nav>

      <p v-if="serviceDown" class="headset-error" role="alert">
        <CircleAlert :size="15" :stroke-width="1.75" />
        <span>
          QuantumEngine keeps trying to start its background service. If this doesn't go away, run
          <code>quantumengined</code> in a terminal to see why it stops.
        </span>
      </p>
      <p v-else-if="headsetError" class="headset-error" role="alert">
        <CircleAlert :size="15" :stroke-width="1.75" />{{ headsetError }}
      </p>

      <Transition name="panel" mode="out-in">
        <section
          :id="`panel-${app.section}`"
          :key="app.section"
          class="panel-host"
          role="tabpanel"
          :aria-labelledby="`tab-${app.section}`"
        >
          <component :is="panels[app.section]" />
        </section>
      </Transition>
    </main>
  </div>
  <ToastHost />
  <InfoCredits />
</template>

<style scoped>
.layout {
  display: grid;
  gap: 24px;
  max-width: 600px;
  margin: 0 auto;
  padding: 22px var(--gutter) 28px;
}

.overview {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.tabs {
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  gap: 3px;
  margin: 0 calc(-1 * var(--gutter));
  padding: 5px var(--gutter);
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--film);
}

.tabs button {
  position: relative;
  flex: 1;
  height: 36px;
  padding: 0 10px;
  border: 0;
  background: transparent;
  color: var(--ink-3);
  font-weight: 500;
  border-radius: 7px;
  transition: color 160ms, background 160ms;
}

.tabs button:hover {
  color: var(--ink);
}

.tabs button.on {
  color: var(--ink);
  font-weight: 600;
  background: var(--film-raised);
  box-shadow: inset 0 0 0 1px var(--line-strong);
}

.tabs button:focus-visible {
  outline-offset: 1px;
  border-radius: 4px;
}

.headset-error {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  margin: 4px 0 0;
  padding: 10px 12px;
  border: 1px solid rgb(255 124 105 / 0.24);
  border-radius: var(--radius-sm);
  background: rgb(255 124 105 / 0.07);
  font-size: 13px;
  color: var(--alert);
}

.headset-error svg {
  flex: none;
  margin-top: 1px;
}

.headset-error code {
  font-family: ui-monospace, "JetBrains Mono", monospace;
  font-size: 12px;
  color: var(--ink);
}

/* Narrow window with room: the overview stays put and only the open
   section scrolls, so the figure is in view while you change something. */
@media (max-width: 839px) and (min-height: 560px) {
  .layout {
    grid-template-rows: auto minmax(0, 1fr);
    gap: 16px;
    height: 100vh;
    padding-top: 16px;
  }

  .overview {
    gap: 14px;
  }

  .work {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .work .panel-host {
    flex: 1;
    min-height: 0;
    margin: 0 calc(-1 * var(--gutter));
    padding: 22px var(--gutter) 0;
    overflow-y: auto;
  }
}

.panel-host {
  padding: 22px 0 28px;
}

.panel-enter-active {
  transition:
    opacity 220ms var(--ease-out),
    transform 220ms var(--ease-out);
}

.panel-leave-active {
  transition: opacity 90ms;
}

.panel-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

.panel-leave-to {
  opacity: 0;
}

/* Wide window: the figure stays put on the left, the work on the right. */
@media (min-width: 840px) {
  .layout {
    grid-template-columns: minmax(340px, 0.86fr) minmax(400px, 1.14fr);
    gap: clamp(32px, 4vw, 56px);
    max-width: 1200px;
    padding: 28px 24px 36px;
  }

  .overview {
    position: sticky;
    top: 28px;
    align-self: start;
  }

  .tabs {
    margin: 0;
    padding: 5px;
  }
}

/* Landscape windows have less vertical room. Keep the overview in view and
   give the controls their own scroll area instead of scrolling the whole app. */
@media (min-width: 840px) and (max-height: 760px) {
  .layout {
    grid-template-rows: minmax(0, 1fr);
    height: 100vh;
    height: 100dvh;
    min-height: 0;
    padding: 16px 24px;
    gap: 24px;
  }

  .overview {
    top: 16px;
    gap: 12px;
    min-height: 0;
  }

  .layout.eq-open .overview {
    align-self: stretch;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .work {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .work .panel-host {
    flex: 1;
    min-height: 0;
    margin: 0 -12px;
    padding: 18px 12px 24px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
}

@media (min-width: 600px) and (max-width: 839px) and (max-height: 559px) {
  .layout {
    grid-template-columns: minmax(220px, 0.8fr) minmax(320px, 1.2fr);
    align-items: start;
    gap: 20px;
    max-width: 1100px;
    padding: 14px var(--gutter) 18px;
  }

  .overview {
    gap: 12px;
  }

  .tabs {
    margin-left: 0;
    margin-right: 0;
    padding-inline: 4px;
  }

  .tabs button {
    height: 34px;
    padding-inline: 5px;
    font-size: 12px;
  }

  .panel-host {
    padding-top: 16px;
  }
}
</style>
