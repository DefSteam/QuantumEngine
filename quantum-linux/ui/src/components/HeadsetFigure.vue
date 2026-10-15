<script setup lang="ts">
// Fig. 1: the headset as a technical drawing whose callouts read its live
// state. The earcup rings and logos glow in the lighting colours (the draft
// being edited, which matches what was last applied until you change it),
// animated roughly as the chosen effect runs on the headset. Everything else
// is ink.

import { computed, ref } from "vue";
import { useApp, type Section } from "../stores/app";
import type { ZoneDto } from "../bindings";

const app = useApp();

const hovered = ref<Section | null>(null);
const reducedMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

const ready = computed(() => app.ready);
const h = computed(() => app.headset);
const lit = computed(() => ready.value && h.value.lightsOn);
const draft = computed(() => app.draft);

const ANC_LABELS: Record<string, string> = { off: "Off", on: "ANC", talkthru: "TalkThru" };

const noiseReading = computed(() => (ready.value ? (ANC_LABELS[h.value.anc] ?? "—") : "—"));
const micReading = computed(() => (ready.value ? (h.value.micActive ? "Live" : "Muted") : "—"));
const lightsReading = computed(() => {
  if (!ready.value) return "—";
  if (!h.value.lightsOn) return "Off";
  const effect = app.info.effects.find((e) => e.id === draft.value?.ring.effect);
  return effect ? `On · ${effect.label}` : "On";
});
/** 0 (full chat) to 16 (full game). */
const dial = computed(() => (ready.value ? h.value.dial : null));
const dialReading = computed(() => {
  const d = dial.value;
  if (d === null) return "—";
  if (d === 8) return "Even";
  return d > 8 ? `Game +${d - 8}` : `Chat +${8 - d}`;
});

/** The part under attention: hovered, else the open section's. */
function active(part: Section): boolean {
  if (!ready.value) return false;
  return hovered.value === part || (hovered.value === null && app.section === part);
}

/** SMIL values for cycling through a zone's colours. */
function cycle(zone: ZoneDto): string {
  const c = zone.colours;
  return [...c, c[0]].join(";");
}

type Motion = "steady" | "fade" | "step" | "blink" | "chase" | "spectrum";

/** How the figure animates an effect. A sketch of it, not a recording. */
function motion(zone: ZoneDto, which: "logo" | "ring"): Motion {
  if (reducedMotion) return "steady";
  const label = app.info.effects.find((e) => e.id === zone.effect)?.label.toLowerCase() ?? "";
  if (label === "spectrum" || (which === "logo" && label === "rainbow logo")) return "spectrum";
  if (label.startsWith("chase")) {
    if (which === "ring") return "chase";
    return label.includes("strobe") || label.includes("flash") || label.includes("blink") ? "blink" : "steady";
  }
  if (label === "strobe") return "blink";
  if (label === "instant" || label === "fast") return zone.colours.length > 1 ? "step" : "blink";
  if (label === "fade") return zone.colours.length > 1 ? "fade" : "steady";
  return "steady";
}

const SPECTRUM = "#ff3b3b;#ffb02e;#f5f23a;#3ddc6b;#2fc8ff;#5b6cff;#c34bff;#ff3b3b";

function glowOpacity(zone: ZoneDto): number {
  return 0.25 + 0.65 * (zone.brightness / 100);
}

const cups = [
  { side: "left", cx: 108 },
  { side: "right", cx: 292 },
] as const;
const CY = 170;
</script>

<template>
  <figure class="figure" :class="{ offline: !ready, dirty: app.lightingDirty }">
    <div class="plate">
      <div class="stage">
      <svg viewBox="0 0 400 300" class="drawing" aria-hidden="true">
        <defs>
          <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="5" />
          </filter>
        </defs>

        <!-- Leader lines, under the drawing so they tuck behind it. -->
        <g class="leaders">
          <polyline class="leader" :class="{ on: active('sound') }" points="54,150 16,150 16,60" />
          <polyline class="leader" :class="{ on: active('lights') }" points="322,133 384,133 384,60" />
          <polyline class="leader" points="167,268 176,268" />
          <polyline class="leader" points="347,196 384,196 384,244" />
        </g>

        <!-- Headband -->
        <path
          class="band"
          d="M 90 124 C 90 -2 310 -2 310 124 L 288 124 C 288 36 112 36 112 124 Z"
        />
        <path class="seam" d="M 101 120 C 101 17 299 17 299 120" />
        <rect class="slider" x="95" y="94" width="14" height="22" rx="3" />
        <rect class="slider" x="291" y="94" width="14" height="22" rx="3" />

        <!-- Mic boom, from the left cup to the capsule. -->
        <path class="boom-outer" d="M 72 214 C 62 256 108 282 152 270" />
        <path class="boom-inner" d="M 72 214 C 62 256 108 282 152 270" />
        <g class="mic" :class="{ live: ready && h.micActive }">
          <ellipse class="capsule" cx="158" cy="268" rx="10" ry="8" />
          <circle class="mic-dot" cx="158" cy="268" r="3" />
          <line v-if="ready && !h.micActive" class="mic-slash" x1="151" y1="275" x2="165" y2="261" />
        </g>

        <!-- Earcups -->
        <g v-for="cup in cups" :key="cup.side" class="cup">
          <ellipse class="shell" :cx="cup.cx" :cy="CY" rx="52" ry="64" />

          <template v-if="lit && draft">
            <ellipse
              class="ring-glow"
              :cx="cup.cx"
              :cy="CY"
              rx="42"
              ry="53"
              filter="url(#glow)"
              :stroke="draft.ring.colours[0]"
              :opacity="glowOpacity(draft.ring)"
            >
              <animate
                v-if="motion(draft.ring, 'ring') === 'fade'"
                attributeName="stroke"
                :values="cycle(draft.ring)"
                :dur="`${draft.ring.colours.length * 1.6}s`"
                repeatCount="indefinite"
              />
              <animate
                v-else-if="motion(draft.ring, 'ring') === 'step'"
                attributeName="stroke"
                :values="cycle(draft.ring)"
                :dur="`${draft.ring.colours.length * 0.6}s`"
                calcMode="discrete"
                repeatCount="indefinite"
              />
              <animate
                v-else-if="motion(draft.ring, 'ring') === 'spectrum'"
                attributeName="stroke"
                :values="SPECTRUM"
                dur="6s"
                repeatCount="indefinite"
              />
              <animate
                v-if="motion(draft.ring, 'ring') === 'blink'"
                attributeName="opacity"
                :values="`${glowOpacity(draft.ring)};0.05`"
                dur="0.5s"
                calcMode="discrete"
                repeatCount="indefinite"
              />
            </ellipse>
            <ellipse
              class="ring"
              :class="{ chase: motion(draft.ring, 'ring') === 'chase' }"
              :cx="cup.cx"
              :cy="CY"
              rx="42"
              ry="53"
              :stroke="draft.ring.colours[0]"
            >
              <animate
                v-if="motion(draft.ring, 'ring') === 'fade'"
                attributeName="stroke"
                :values="cycle(draft.ring)"
                :dur="`${draft.ring.colours.length * 1.6}s`"
                repeatCount="indefinite"
              />
              <animate
                v-else-if="motion(draft.ring, 'ring') === 'step' || motion(draft.ring, 'ring') === 'chase'"
                attributeName="stroke"
                :values="cycle(draft.ring)"
                :dur="`${draft.ring.colours.length * 0.6}s`"
                calcMode="discrete"
                repeatCount="indefinite"
              />
              <animate
                v-else-if="motion(draft.ring, 'ring') === 'spectrum'"
                attributeName="stroke"
                :values="SPECTRUM"
                dur="6s"
                repeatCount="indefinite"
              />
              <animate
                v-if="motion(draft.ring, 'ring') === 'blink'"
                attributeName="opacity"
                values="1;0.12"
                dur="0.5s"
                calcMode="discrete"
                repeatCount="indefinite"
              />
              <animate
                v-if="motion(draft.ring, 'ring') === 'chase'"
                attributeName="stroke-dashoffset"
                values="0;-60"
                dur="1.2s"
                repeatCount="indefinite"
              />
            </ellipse>
          </template>
          <ellipse v-else class="ring unlit" :cx="cup.cx" :cy="CY" rx="42" ry="53" />

          <ellipse class="cap" :cx="cup.cx" :cy="CY" rx="32" ry="42" />

          <template v-if="lit && draft">
            <rect
              class="logo-glow"
              :x="cup.cx - 15"
              :y="CY - 5"
              width="30"
              height="10"
              rx="5"
              filter="url(#glow)"
              :fill="draft.logo.colours[0]"
              :opacity="glowOpacity(draft.logo)"
            />
            <rect class="logo" :x="cup.cx - 13" :y="CY - 3.5" width="26" height="7" rx="3.5" :fill="draft.logo.colours[0]">
              <animate
                v-if="motion(draft.logo, 'logo') === 'fade'"
                attributeName="fill"
                :values="cycle(draft.logo)"
                :dur="`${draft.logo.colours.length * 1.6}s`"
                repeatCount="indefinite"
              />
              <animate
                v-else-if="motion(draft.logo, 'logo') === 'step'"
                attributeName="fill"
                :values="cycle(draft.logo)"
                :dur="`${draft.logo.colours.length * 0.6}s`"
                calcMode="discrete"
                repeatCount="indefinite"
              />
              <animate
                v-else-if="motion(draft.logo, 'logo') === 'spectrum'"
                attributeName="fill"
                :values="SPECTRUM"
                dur="4s"
                repeatCount="indefinite"
              />
              <animate
                v-if="motion(draft.logo, 'logo') === 'blink'"
                attributeName="opacity"
                values="1;0.12"
                dur="0.4s"
                calcMode="discrete"
                repeatCount="indefinite"
              />
            </rect>
          </template>
          <rect v-else class="logo unlit" :x="cup.cx - 13" :y="CY - 3.5" width="26" height="7" rx="3.5" />
        </g>

        <!-- Controls on the cups -->
        <rect class="part" x="51" y="139" width="7" height="22" rx="2.5" />
        <g class="part">
          <rect x="340" y="183" width="8" height="26" rx="3" />
          <line x1="341.5" y1="188" x2="346.5" y2="188" />
          <line x1="341.5" y1="193" x2="346.5" y2="193" />
          <line x1="341.5" y1="198" x2="346.5" y2="198" />
          <line x1="341.5" y1="203" x2="346.5" y2="203" />
        </g>

        <!-- Where each leader meets its part -->
        <g class="anchors">
          <circle class="anchor" :class="{ on: active('sound') }" cx="54" cy="150" r="2.5" />
          <circle v-if="active('sound')" class="halo" cx="54" cy="150" r="11" />
          <circle class="anchor" :class="{ on: active('lights') }" cx="322" cy="133" r="2.5" />
          <circle v-if="active('lights')" class="halo" cx="322" cy="133" r="11" />
          <circle class="anchor" cx="347" cy="196" r="2.5" />
        </g>
      </svg>

      <!-- Callouts: HTML, so they are real text and real buttons. -->
      <button
        class="callout tl"
        :class="{ on: active('sound') }"
        type="button"
        @click="app.section = 'sound'"
        @mouseenter="hovered = 'sound'"
        @mouseleave="hovered = null"
        @focus="hovered = 'sound'"
        @blur="hovered = null"
      >
        <span class="what"><span class="key num">1</span>Noise control</span>
        <span class="reading">{{ noiseReading }}</span>
      </button>
      <button
        class="callout tr"
        :class="{ on: active('lights') }"
        type="button"
        @click="app.section = 'lights'"
        @mouseenter="hovered = 'lights'"
        @mouseleave="hovered = null"
        @focus="hovered = 'lights'"
        @blur="hovered = null"
      >
        <span class="what"><span class="key num">2</span>Lights</span>
        <span class="reading">{{ lightsReading }}</span>
      </button>
      <div class="callout bm">
        <span class="what"><span class="key num">3</span>Mic</span>
        <span class="reading" :class="{ live: ready && h.micActive }">{{ micReading }}</span>
      </div>
      <div class="callout br">
        <span class="what"><span class="key num">4</span>Game/chat</span>
        <span class="reading num">{{ dialReading }}</span>
        <span class="meter" aria-hidden="true">
          <span class="meter-mark" :style="{ left: `${((dial ?? 8) / 16) * 100}%`, opacity: dial === null ? 0 : 1 }" />
        </span>
      </div>
      </div>
    </div>
    <figcaption>
      <span class="fig">Fig. 1</span>
      <span v-if="app.lightingDirty" class="draft-note">Lighting preview, not applied yet</span>
      <span v-else-if="ready">Quantum 810 Wireless, as it is now</span>
      <span v-else>Quantum 810 Wireless, not connected</span>
    </figcaption>
  </figure>
</template>

<style scoped>
.figure {
  margin: 0;
}

/* Narrow window with room: the figure stays on screen above the open
   section, so it gives up some size to leave the section space. */
@media (max-width: 839px) and (min-height: 560px) {
  .stage {
    width: min(100%, 46vh);
  }
}

.plate {
  display: flex;
  justify-content: center;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  /* Drafting film: a faint dot grid. */
  background:
    radial-gradient(circle at 1px 1px, rgb(236 232 225 / 0.06) 1px, transparent 1.2px) 0 0 / 14px 14px,
    var(--film);
  overflow: hidden;
}

/* The drawing and its callouts, at the viewBox's 4:3. */
.stage {
  position: relative;
  width: 100%;
  aspect-ratio: 4 / 3;
}

.drawing {
  position: absolute;
  inset: 4% 4% 4% 4%;
  width: 92%;
  height: 92%;
  overflow: visible;
}

.band {
  fill: var(--film-raised);
  stroke: var(--ink-2);
  stroke-width: 1.4;
  stroke-linejoin: round;
}

.seam {
  fill: none;
  stroke: var(--line-strong);
  stroke-width: 1;
  stroke-dasharray: 2 4;
}

.slider {
  fill: var(--film-raised);
  stroke: var(--ink-2);
  stroke-width: 1.2;
}

.shell {
  fill: var(--film-raised);
  stroke: var(--ink);
  stroke-width: 1.5;
}

.ring {
  fill: none;
  stroke-width: 3.5;
  transition: stroke 400ms var(--ease-out);
}

.ring.chase {
  stroke-dasharray: 22 8;
}

/* An unapplied lighting draft is drawn, not lit: dashed. */
.dirty .ring {
  stroke-dasharray: 7 5;
}

.ring.unlit {
  stroke: #2b2f36;
}

.ring-glow {
  fill: none;
  stroke-width: 9;
}

.cap {
  fill: #101215;
  stroke: var(--line-strong);
  stroke-width: 1;
}

.logo.unlit {
  fill: #2b2f36;
}

.boom-outer {
  fill: none;
  stroke: var(--ink-2);
  stroke-width: 7;
  stroke-linecap: round;
}

.boom-inner {
  fill: none;
  stroke: var(--film-raised);
  stroke-width: 4.2;
  stroke-linecap: round;
}

.capsule {
  fill: var(--film-raised);
  stroke: var(--ink);
  stroke-width: 1.4;
}

.mic-dot {
  fill: var(--ink-3);
}

.mic.live .mic-dot {
  fill: var(--ink);
}

.mic-slash {
  stroke: var(--alert);
  stroke-width: 1.6;
  stroke-linecap: round;
}

.part,
.part rect {
  fill: var(--film-raised);
  stroke: var(--ink);
  stroke-width: 1.2;
}

.part line {
  stroke: var(--ink-3);
  stroke-width: 0.8;
}

.leader {
  fill: none;
  stroke: var(--line-strong);
  stroke-width: 1;
  transition: stroke 200ms;
}

.leader.on {
  stroke: var(--violet);
  stroke-dasharray: 200;
  animation: draw 520ms var(--ease-out);
}

.anchor {
  fill: var(--ink-2);
  transition: fill 200ms;
}

.anchor.on {
  fill: var(--violet);
}

.halo {
  fill: var(--violet-soft);
  stroke: var(--violet);
  stroke-width: 1;
  animation: settle 520ms var(--ease-out);
  transform-box: fill-box;
  transform-origin: center;
}

@keyframes draw {
  from {
    stroke-dashoffset: 200;
  }
  to {
    stroke-dashoffset: 0;
  }
}

@keyframes settle {
  from {
    transform: scale(1.8);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

/* Not connected: the drawing goes to pencil. */
.offline .shell,
.offline .band,
.offline .capsule,
.offline .part,
.offline .part rect,
.offline .slider {
  stroke: var(--ink-3);
  stroke-dasharray: 3 3;
}

.offline .boom-outer {
  stroke: var(--line-strong);
}

/* Callouts sit at the ends of the leaders: x and y are the viewBox
   coordinates as percentages of the 4:3 plate, inset like the drawing. */
.callout {
  position: absolute;
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin: 0;
  padding: 4px 6px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  text-align: left;
  line-height: 1.2;
}

button.callout {
  transition: background 160ms;
}

button.callout:hover,
button.callout.on {
  background: var(--violet-soft);
}

.callout .what {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--ink-3);
}

.tr .what,
.br .what {
  flex-direction: row-reverse;
}

/* Part numbers, as on a service drawing. */
.key {
  display: inline-grid;
  place-items: center;
  width: 15px;
  height: 15px;
  border: 1px solid var(--line-strong);
  border-radius: 50%;
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  color: var(--ink-2);
}

.callout.on .key {
  border-color: var(--violet);
  color: var(--ink);
}

.callout .reading {
  font-size: 15px;
  font-weight: 600;
  color: var(--ink);
  white-space: nowrap;
}

.offline .callout .reading {
  color: var(--ink-3);
}

/* Top-left: the noise control leader ends at x 16, y 60. */
.tl {
  left: calc(4% + 0.92 * 16 / 400 * 100% - 6px);
  top: calc(4% + 0.92 * 60 / 300 * 100%);
  transform: translateY(-100%);
}

.tr {
  right: calc(4% + 0.92 * 16 / 400 * 100% - 6px);
  top: calc(4% + 0.92 * 60 / 300 * 100%);
  transform: translateY(-100%);
  align-items: flex-end;
  text-align: right;
}

.bm {
  left: calc(4% + 0.92 * 176 / 400 * 100%);
  top: calc(4% + 0.92 * 268 / 300 * 100%);
  transform: translateY(-50%);
}

.br {
  right: calc(4% + 0.92 * 16 / 400 * 100% - 6px);
  top: calc(4% + 0.92 * 244 / 300 * 100%);
  align-items: flex-end;
  text-align: right;
}

.reading.live {
  color: var(--ink);
}

.meter {
  position: relative;
  width: 56px;
  height: 2px;
  margin-top: 5px;
  background: var(--line-strong);
}

.meter-mark {
  position: absolute;
  top: -3px;
  width: 2px;
  height: 8px;
  margin-left: -1px;
  background: var(--ink);
  transition: left 300ms var(--ease-out);
}

figcaption {
  display: flex;
  gap: 10px;
  align-items: baseline;
  margin-top: 8px;
  font-size: 12px;
  color: var(--ink-3);
}

.fig {
  font-style: italic;
  font-weight: 600;
  color: var(--ink-2);
}

.draft-note {
  color: var(--violet);
}
</style>
