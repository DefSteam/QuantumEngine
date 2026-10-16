<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useApp } from "../stores/app";
import EqCurvePanel from "./EqCurvePanel.vue";

const app = useApp();
const layoutQuery = window.matchMedia("(min-width: 840px)");
const wideLayout = ref(layoutQuery.matches);

function updateLayout() {
  wideLayout.value = layoutQuery.matches;
}

onMounted(() => layoutQuery.addEventListener("change", updateLayout));
onBeforeUnmount(() => layoutQuery.removeEventListener("change", updateLayout));

const eq = computed(() => app.state.eq);
const usable = computed(() => app.serviceUp && eq.value.profiles.length > 0);
const busy = computed(() => app.pending !== null);
const active = computed(() => app.serviceUp && eq.value.enabled);
const correctionLabel = computed(() => {
  const labels: Record<string, string> = {
    off: "Off",
    stereo: "JBL stereo",
    surround: "JBL surround",
  };
  return labels[eq.value.correction] ?? "no";
});

const points = ref<number[] | null>(null);
watch(
  () => [eq.value.profile, eq.value.correction] as const,
  async ([profile, correction]) => {
    if (!profile || !correction) {
      points.value = null;
      return;
    }
    try {
      points.value = await app.curve(profile, correction);
    } catch {
      points.value = null;
    }
  },
  { immediate: true },
);
</script>

<template>
  <EqCurvePanel
    v-if="wideLayout"
    :freqs="app.info.eqFrequencies"
    :points="points"
    :preview="null"
    :active="active"
    :pending="app.pending?.startsWith('eq:') ?? false"
    :usable="usable"
    :busy="busy"
    :profile="eq.profile"
    :correction-label="correctionLabel"
    :previewing="null"
    :service-up="app.serviceUp"
    :service-outdated="app.state.serviceOutdated"
    :has-profiles="eq.profiles.length > 0"
    :status="eq.status"
    class="eq-curve-dock"
    @toggle="app.setEqEnabled"
  />
</template>
