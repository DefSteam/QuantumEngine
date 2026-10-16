<script setup lang="ts">
import { CircleAlert } from "lucide-vue-next";
import EqCurve from "./EqCurve.vue";
import PanelField from "./PanelField.vue";
import ToggleSwitch from "./ToggleSwitch.vue";

defineProps<{
  freqs: number[];
  points: number[] | null;
  preview: number[] | null;
  active: boolean;
  pending: boolean;
  usable: boolean;
  busy: boolean;
  profile: string;
  correctionLabel: string;
  previewing: string | null;
  serviceUp: boolean;
  serviceOutdated: boolean;
  hasProfiles: boolean;
  status: string;
}>();

const emit = defineEmits<{ toggle: [on: boolean] }>();
</script>

<template>
  <PanelField title="Equalizer" hint="Applied on this PC to the headset's game audio. Chat is left alone.">
    <template #aside>
      <ToggleSwitch
        label="Equalizer"
        :checked="active"
        :pending="pending"
        :disabled="!usable || busy"
        @toggle="emit('toggle', $event)"
      />
    </template>
    <div class="plot">
      <EqCurve :freqs="freqs" :points="points" :preview="preview" :active="active" />
      <p class="plot-note">
        <template v-if="previewing && previewing !== profile && preview">
          <span class="swatch-line violet" />{{ previewing }}, preview
        </template>
        <template v-else-if="serviceOutdated && !hasProfiles">
          The background service running now is too old for the EQ; restart it above
        </template>
        <template v-else-if="active">{{ profile }} with {{ correctionLabel }} tuning</template>
        <template v-else-if="serviceUp">Off: the sound passes through unchanged</template>
        <template v-else>Off: the background service isn't running</template>
      </p>
    </div>
    <p v-if="status" class="status"><CircleAlert :size="15" :stroke-width="1.75" />{{ status }}</p>
  </PanelField>
</template>

<style scoped>
.plot {
  padding: 12px 12px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--film);
}

.plot-note {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--ink-3);
}

.swatch-line {
  width: 14px;
  height: 2px;
  border-radius: 2px;
}

.swatch-line.violet {
  background: var(--violet);
}

.status {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  margin: 0;
  font-size: 13px;
  color: var(--alert);
}

.status svg {
  flex: none;
  margin-top: 1px;
}
</style>
