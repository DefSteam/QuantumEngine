---
version: 1
slug: "ui-src-app-vue"
primary_target: "ui/src/App.vue"
related_targets: []
---

## Scope

The QuantumEngine desktop window (Tauri 2 + Vue, WebKitGTK), default 1100×740 (landscape), minimum 360×520. Mode: Operate. Opened occasionally for lighting, EQ, updates and details; quick toggles live in the tray and Plasma widget. User brief: "modern, yet beautiful and dark".

## Direction contract

THESIS: The headset is the dashboard: a live technical figure whose numbered callouts read its state. Refuses the vendor-app sidebar with a product render and neon cards.
OWN-WORLD: Graphite drafting film (#0e0f12), warm ink white, hairline grey, Archivo at two widths. Chrome is ink only; the only colour is the headset's real lighting inside Fig. 1, plus one reserved violet (the logo's) ringing what's under attention.
STORY: See the headset's condition at a glance, pick a numbered part, change it, watch the figure confirm.
FIRST VIEWPORT: Huge battery figure top-left over status; Fig. 1 headset below with leaders to noise, mic, dial, lights; numbered section tabs; selected panel.
FORM: Service Manual Figure, position 6 of 7; seed 3fb446bb.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Raises carried from the round

- Battery owns one enormous figure; other labels small (type specimen).
- One section at a time, no long scroll (vertical feed).
- Colour quarantined to the figure (lexicon plate).
- Attention ringed in one reserved violet (star atlas).
- Every row ends in its reading, right-aligned (cassette j-card).
- Fixed-width figures that never reflow (bitmap specimen).

## Decisions made while building

- Part numbers key the figure: callouts are numbered 1–4 (noise control, lights, microphone, game/chat dial). The tabs stay unnumbered, because EQ and Device have no part on the headset; the violet attention ring on the callout marks which part the open section controls.
- Alert red (#ff7c69) is the one exception to ink-only chrome, used only for failures and a low battery (≤15%), because state comes first.
- Narrow windows with room (under 840px wide, at least 560px high) pin the overview and scroll only the open section, so the figure stays in view while you edit.

## Signature interaction

Editing lighting repaints the figure live as a dashed "not applied" draft; Apply turns it solid once the headset confirms.
