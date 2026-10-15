---
name: QuantumEngine
description: A Linux control window for the JBL Quantum 810 Wireless, drawn as a live service-manual figure on graphite drafting film.
colors:
  ground: "#0e0f12"
  film: "#15171b"
  film-raised: "#1b1e23"
  line: "#2a2e35"
  line-strong: "#3d424b"
  ink: "#ece8e1"
  ink-2: "#b0b3ba"
  ink-3: "#868b94"
  violet: "#a46bff"
  violet-soft: "rgb(164 107 255 / 0.16)"
  alert: "#ff7c69"
typography:
  display:
    fontFamily: "Archivo Variable, Noto Sans, system-ui, sans-serif"
    fontSize: "84px"
    fontWeight: 250
    lineHeight: 0.86
    letterSpacing: "-0.035em"
    fontVariation: "\"wdth\" 112"
    fontFeature: "\"tnum\""
  reading:
    fontFamily: "Archivo Variable, Noto Sans, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.2
    fontVariation: "\"wdth\" 100"
  title:
    fontFamily: "Archivo Variable, Noto Sans, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.45
    fontVariation: "\"wdth\" 100"
  body:
    fontFamily: "Archivo Variable, Noto Sans, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
    fontVariation: "\"wdth\" 100"
  detail:
    fontFamily: "Archivo Variable, Noto Sans, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "Archivo Variable, Noto Sans, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
  part-key:
    fontFamily: "Archivo Variable, Noto Sans, system-ui, sans-serif"
    fontSize: "10px"
    fontWeight: 600
    lineHeight: 1
    fontFeature: "\"tnum\""
rounded:
  sm: "6px"
  md: "10px"
  pill: "13px"
  full: "50%"
spacing:
  gutter: "16px"
  field-inner: "10px"
  section: "22px"
  field-stack: "26px"
  columns: "40px"
components:
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ground}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "36px"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "0 14px"
    height: "36px"
  button-secondary-hover:
    backgroundColor: "{colors.film-raised}"
  segmented-option:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "32px"
  segmented-option-on:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ground}"
  switch:
    backgroundColor: "{colors.ground}"
    rounded: "{rounded.pill}"
    width: "44px"
    height: "26px"
  switch-on:
    backgroundColor: "{colors.ink}"
  select:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "0 34px 0 11px"
    height: "34px"
  tab:
    backgroundColor: "transparent"
    textColor: "{colors.ink-3}"
    padding: "0 12px"
    height: "40px"
  tab-on:
    textColor: "{colors.ink}"
  callout-on:
    backgroundColor: "{colors.violet-soft}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "4px 6px"
  plate:
    backgroundColor: "{colors.film}"
    rounded: "{rounded.md}"
  strip:
    backgroundColor: "{colors.film-raised}"
    rounded: "{rounded.md}"
    padding: "12px 14px"
  toast:
    backgroundColor: "{colors.film-raised}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "11px 10px 11px 12px"
---

# Design System: QuantumEngine

## Overview

**Creative North Star: "The Service Manual Figure"**

The window is a page from a technical manual, printed in warm ink on graphite drafting film. Its centre is Fig. 1, a line drawing of the headset whose numbered callouts read the device's live state. Everything around the figure is quiet, ink-only chrome: one enormous battery numeral, a status line, unnumbered section tabs and one open section at a time. The headset's own lighting is the only colour in the window, and it lives inside the figure.

The density is that of a desktop utility opened for a minute and closed again. It is calm and exact, with no hype. Readings sit right-aligned at the end of their row, numerals are tabular and never reflow, and every control shows only what the headset has confirmed. Pending and draft states are drawn, not filled: a sweeping violet rule, a ringing knob, a dashed lighting ring.

The world rejects the vendor-app sidebar with a product render and neon cards. It must never imitate JBL's or QuantumENGINE's branding.

**Key Characteristics:**
- Graphite film ground with warm ink type; chrome carries no hue.
- One reserved violet (the logo's) marks attention: focus, the selected part, a hovered preview, a pending write.
- Alert red appears only for failures and a low battery (15% or less).
- Archivo Variable at two widths: 112 for the battery figure, 100 for everything else.
- Fig. 1 is a live technical drawing: leaders, anchors, numbered part keys, an italic figure caption.
- Flat surfaces separated by hairlines; elevation only for the floating toast.

## Colors

A near-monochrome drafting palette: cool graphite surfaces, warm off-white ink in three strengths, one violet for attention and one red for trouble.

### Primary
- **Logo Violet** (`violet`): the attention colour, borrowed from the app icon's neon "Q" ring. It draws the focus outline (2px, 2px offset), the active leader line and anchor halo on Fig. 1, the ring on the active part key, the pending sweep under a segmented option, the ring on a pending switch knob, the pending profile mark, the EQ preview curve and the "Lighting preview, not applied yet" caption. **Violet Wash** (`violet-soft`) is the hover and active fill behind a callout or EQ profile row, and the text selection colour.

### Secondary
- **Signal Coral** (`alert`): failures and a battery at 15% or less. It colours the battery figure and label when low, inline error lines with their alert icon, the toast icon and the mic-muted slash on Fig. 1. It has no other use.

### Neutral
- **Graphite Ground** (`ground`): the window background, the sticky tab bar and action bar, and the inset ground of segmented controls, switches and selects.
- **Drafting Film** (`film`): the Fig. 1 plate and the EQ plot, the surfaces that carry drawings. The plate adds a faint dot grid (1px ink dots at 6% on a 14px pitch).
- **Raised Film** (`film-raised`): the bodies of parts in the drawing, the update strip, toasts and hover fills on secondary buttons and segmented options.
- **Hairline** (`line`): borders of the plate, plot and segmented track; dividers under the tabs, between list rows and between lighting zones.
- **Strong Hairline** (`line-strong`): borders of interactive controls (secondary button, switch, select, toast, strip), leader lines at rest, part-key circles, scrollbar thumbs and the unfilled part of a range track.
- **Ink** (`ink`): primary text, readings, the filled primary button, the selected segment, the on switch, the tab underline, the EQ curve, the earcup outline.
- **Ink 2** (`ink-2`): secondary readings (13px values at a row's end), unselected segment labels, the battery unit and part-key digits.
- **Ink 3** (`ink-3`): labels, hints, captions, unselected tabs, list terms and the offline "pencil" strokes on Fig. 1.

### Named Rules
**The Quarantined Colour Rule.** Hue belongs to the headset. The lighting colours appear only inside Fig. 1 and in the colour swatches that edit them; the chrome around them is ink on graphite.

**The One Violet Rule.** Violet means "this has your attention" and nothing else: focus, the selected or hovered part, a preview or a pending write. It is never decoration, never a brand fill, never a button colour.

**The Alert Only Rule.** Coral is reserved for failures and a low battery. State comes first, so it is the single exception to ink-only chrome.

## Typography

**Display Font:** Archivo Variable at width 112 (with Noto Sans, system-ui)
**Body Font:** Archivo Variable at width 100 (with Noto Sans, system-ui)

**Character:** One grotesque at two widths. The wide, very light battery numeral reads like a specimen figure; the regular width does all the work at small sizes, its weight steps (400, 500, 600) carrying the hierarchy rather than size.

### Hierarchy
- **Display** (250, 84px, line-height 0.86, -0.035em, width 112, tabular): the battery percentage only. In a narrow window with height to spare it scales as `clamp(56px, 9vh, 84px)`. Its "%" unit sits top-right at 20px, weight 400, in ink 2.
- **Reading** (600, 15px, 1.2): the live value in each Fig. 1 callout ("ANC", "On · Fade", "Game +3").
- **Title** (600, 14px): field headings, the status title, the selected tab and the update headline.
- **Body** (400, 14px, 1.45): default text; buttons use weight 500 (secondary, segmented, tabs) or 600 (primary, selected).
- **Detail** (400, 13px): readings at a row's end, list terms and values, inline errors, toast messages.
- **Label** (400, 12px): hints under controls, status detail (max 26ch), callout part names, captions, EQ group names, credits.
- **Part key** (600, 10px, tabular): the digit inside a 15–17px hairline circle.

### Named Rules
**The Fixed Figures Rule.** Every number that changes (battery, readings, dial, EQ axis labels) is set with tabular numerals so it never shifts its neighbours.

**The One Giant Rule.** Only the battery is large. Everything else stays between 12px and 15px; the hierarchy comes from weight and ink strength.

**The Sentence Case Rule.** Labels, tabs, headings and buttons are in sentence case at their normal tracking. There are no all-caps labels.

## Layout

A single column of 560px maximum, centred, with a 16px gutter and 20px top padding. The overview (update strip, status header, Fig. 1) stacks with 22px gaps, then the work area: a sticky tab bar with a hairline below it, then the open section with 22px top and 28px bottom padding. Only one section is open at a time; there is no long scrolling page.

- **Narrow with room** (under 840px wide, at least 560px tall): the layout fills the viewport height. The overview stays put, its gaps tighten to 14px, Fig. 1 shrinks to `min(100%, 46vh)`, and only the open section scrolls.
- **Wide** (840px and up): two columns (`minmax(360px, 1fr)` and `minmax(400px, 1fr)`) with a 40px gap and a 1080px maximum. The overview is sticky on the left at 28px from the top; the work sits on the right.
- **Short windows** fall back to normal page scrolling.

Inside a section, a field is a head row (title left, reading right-aligned, 26px minimum height), its control, then a hint, with 10px between them and 26px between fields. Lighting zones and device rows are separated by hairlines rather than gaps. Action bars (Apply, Revert) are sticky at the bottom, bleed to the gutter edges and sit on the ground colour above a hairline.

**The Reading At The Margin Rule.** Every row ends in its reading, right-aligned: field readings, device list values, the status line and the right-hand callouts.

## Elevation & Depth

The system is flat. Depth comes from tonal steps (ground, film, raised film) and hairline borders, not shadows. The one floating surface, the toast, is the one thing that gets a cast shadow. Small functional shadows remain: the live status dot's 3px ink halo, the range thumb's 1px lift, and the switch knob's violet ring while pending.

### Shadow Vocabulary
- **Float** (`box-shadow: 0 10px 30px -8px rgb(0 0 0 / 0.6)`): toasts only, as they hover above the window's content.
- **Live halo** (`box-shadow: 0 0 0 3px rgb(236 232 225 / 0.12)`): the connected status dot.
- **Pending ring** (`box-shadow: 0 0 0 3px var(--violet)`, pulsing to 1px): a switch knob waiting for the headset to confirm.

### Named Rules
**The Hairline Over Shadow Rule.** Surfaces are separated by a 1px line in `line` or `line-strong`, never by a drop shadow. Only something that floats above the page casts one.

## Shapes

Gently rounded rectangles in two sizes: 10px for containers (the Fig. 1 plate, the EQ plot, the update strip, toasts) and 6px for controls (buttons, select, segmented options, callout hover fills). The segmented track's radius is the inner radius plus its 3px padding (9px), so the corners nest. The switch is a full pill (13px on 26px). Circles mark things you pick or that identify parts: colour swatches (30px), the add/remove swatch buttons, profile radio marks, part keys and anchors.

Strokes matter as much as fills. Hairlines are 1px. Fig. 1 uses 1–1.5px outlines and dashes with meaning: a dashed ring is an unapplied lighting draft, dashed outlines mean the headset is not connected ("pencil"), a dotted seam is construction detail. The add-colour button uses a dashed circle because it is a place for something not yet there.

## Components

### Buttons
Quiet and certain: one filled button per panel, everything else a line or a link.
- **Shape:** gently rounded (6px), 36px tall (32px inside the update strip).
- **Primary:** ink fill with ground-coloured text, weight 600, 16px side padding. It is the action the panel exists for (Apply to headset, Install). Hover lightens to pure white; disabled drops to 40% opacity.
- **Secondary:** transparent with a strong hairline border, ink text, weight 500, 14px padding. Hover raises the border to ink 3 and fills with raised film.
- **Link button:** plain ink text with a strong-hairline underline at a 3px offset; the underline turns ink on hover. Used for secondary choices (Revert, What's new).
- **Focus:** 2px violet outline at a 2px offset, everywhere.

### Segmented Control
- **Style:** a ground-coloured track with a hairline border and 3px padding; options share the width equally, 32px tall, ink 2 text at weight 500.
- **Selected:** filled with ink, ground text, weight 600. Hover on an unselected option fills it with raised film.
- **Pending:** the option being applied shows a 2px violet rule sweeping under its label (900ms, alternating) until the headset confirms. While disabled, the selected fill dims to ink 2.

### Switch
- **Style:** 44 by 26px pill on the ground colour with a strong hairline border; an 18px ink 3 knob.
- **On:** track and border fill with ink; the knob turns ground-coloured and slides 18px (260ms ease-out).
- **Pending:** the knob travels to the requested side and waits there, ringed in pulsing violet.

### Inputs / Fields
- **Select:** 34px, ground background, strong hairline border, 6px radius, a hand-drawn chevron from two 1.5px ink 2 strokes. Hover raises the border to ink 3.
- **Range:** 3px track, ink fill up to the value over a strong hairline remainder, 16px ink thumb. Focus rings the thumb in violet.
- **Colour swatch:** a 30px circle of the colour with an inner 18% white hairline and an outer hairline; hover scales it to 1.08.
- **Panel field:** title (with an optional part key) left, reading right, control below, 12px ink 3 hint underneath.

### Navigation
- **Tabs:** unnumbered text tabs (Sound, Lights, EQ, Device), 40px tall, ink 3 at weight 500; hover goes to ink. The selected tab is ink at weight 600 with a 2px ink underline sitting on the bar's hairline. The bar is sticky on the ground colour. Arrow keys move between tabs.

### Lists
- **Device rows:** term (13px ink 3) left, value (13px ink) right, 9px vertical padding, hairlines between rows and above the first.
- **EQ profiles:** an auto-fill grid of 34px rows, each with a 14px circular mark. The selected profile fills its mark with an ink dot and its label goes to ink 600; hover washes the row in violet; a pending mark pulses violet.

### Notices
- **Update strip:** raised film, strong hairline, 10px radius; a 600 headline and a 12px ink 2 subline, with a primary button and a link button to the right. The quiet variant (a newer service is already running) drops the fill.
- **Inline error:** 13px coral text after a 15px alert icon, with no box.
- **Toast:** raised film, strong hairline, 10px radius, the Float shadow, a coral alert icon, a 13px message and a 24px dismiss button. Pinned to the bottom of the window at the gutter, it rises 8px as it fades in.

### Fig. 1 (Signature Component)
A 4:3 technical drawing of the headset on the drafting-film plate. Outlines are ink and ink 2 over raised-film bodies. Earcup rings and logos glow in the real lighting colours, animated roughly as the chosen effect runs on the headset; with the lights off they are drawn in a dark unlit grey. Four callouts sit at the ends of hairline leaders, each with a numbered part key (1 Noise control, 2 Lights, 3 Mic, 4 Game/chat), a 12px ink 3 name and a 15px reading. Callouts 1 and 2 are real buttons that open their section. The part under attention (hovered, or the one the open section controls) gets a violet leader drawn in over 520ms, a violet anchor with a settling halo, a violet ring on its key and a violet wash behind its callout. An italic "Fig. 1" caption in ink 2 at weight 600 is followed by a 12px ink 3 caption that states what the drawing shows.

- **Draft:** editing lighting repaints the figure live with dashed rings and a violet caption, "Lighting preview, not applied yet". Apply turns the rings solid once the headset confirms.
- **Offline:** outlines go to dashed ink 3 "pencil", readings to ink 3 and dashes.

### Motion
Ease-out (`cubic-bezier(0.16, 1, 0.3, 1)`) everywhere. State colour changes take 160–200ms, the switch knob 260ms, panels enter over 220ms rising 6px and leave in 90ms, and leaders draw in over 520ms. Pending indicators loop at 900ms. Reduced motion collapses all transitions to 1ms, and the figure shows lighting effects as steady colour.

## Do's and Don'ts

### Do:
- **Do** keep chrome in ink on graphite (`ground`, `film`, `film-raised`, `ink`, `ink-2`, `ink-3`); the lighting colours live only inside Fig. 1 and its swatches.
- **Do** use violet only for attention: focus (2px outline, 2px offset), the selected or hovered part, previews and pending writes.
- **Do** end every row with its reading, right-aligned, in tabular numerals.
- **Do** show pending and draft states as drawing (a sweeping rule, a ringing knob, a dashed stroke) and switch to the solid, filled state only when the headset confirms.
- **Do** separate surfaces with 1px hairlines and tonal steps; give a cast shadow only to a floating toast.
- **Do** keep one filled ink button per panel, for the action the panel exists for.
- **Do** number a setting with a part key only when it has a matching callout on Fig. 1.

### Don't:
- **Don't** use violet as a fill for buttons, headings or decoration; it would stop meaning "attention".
- **Don't** use coral for anything but failures and a battery at 15% or less.
- **Don't** add a second large figure; only the battery is set at display size.
- **Don't** set labels in all caps or add small tracked-out labels above headings; sentence case at 12px ink 3 is the label voice.
- **Don't** build the vendor-app layout: no sidebar, no product render, no neon cards, nothing that imitates JBL or QuantumENGINE branding.
- **Don't** show controls the headset can't support over the dongle, or a state it hasn't confirmed.
