# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(A desktop window, not a website: a Tauri 2 shell (`crates/quantumengine`)
around a Vue 3 interface (`ui/`), rendered by the system WebKitGTK 4.1. Target
desktop: KDE Plasma 6 on Arch/CachyOS, Wayland or X11.)

## Users

People who own a JBL Quantum 810 Wireless and game on Linux (Arch, CachyOS,
Manjaro), many of whom left Windows and the vendor's QuantumENGINE app behind.
The maintainer is one of them. They expect the headset's features to simply
work on Linux, without Windows or vendor software.

## Product Purpose

Control the headset from Linux: noise control (Off / ANC / TalkThru),
sidetone, lights and lighting effects, a PC-side equalizer, and device status
(battery, mic, game/chat dial, firmware). Success is never needing Windows for
this headset.

## Positioning

The only way to drive this headset on Linux, built on a protocol that was
reverse-engineered from the device, with every command checked on real
hardware. It is unofficial and not affiliated with JBL or Harman; nothing may
suggest otherwise.

## Operating Context

- A background service (`quantumengined`; `quantumengine --tray` still starts
  it) owns the dongle, the EQ, update checks and the tray icon. The window,
  the Plasma widget and `quantumenginectl` are all clients of it over D-Bus,
  and start it when it isn't running.
- Quick toggles (noise control, lights, EQ on/off) usually happen from the
  tray or the Plasma widget. The window is opened now and then for lighting,
  the EQ, update notices and device details, then closed.
- Typical scene: a gaming desk, often during or between game sessions.

## Capabilities and Constraints

- The headset can be absent (no dongle), inaccessible (udev rule missing),
  off/out of range, or ready. The background service can also be down.
- One write at a time: controls lock while a change is pending, until the
  headset confirms it.
- The headset cannot report its lighting; the app shows what it last applied.
  Two zones (logo, ring); both earcups always match. Up to MAX_SLOTS colours
  per zone, 10 known effects, brightness 0–100.
- The game/chat dial and mic boom are physical and read-only.
- The EQ runs on the PC (PipeWire) and works with the headset off: on/off, one
  of 19 profiles, headset tuning Off / JBL stereo / JBL surround. Profiles were
  extracted by alessandromrc; that credit must stay visible.
- Updates: daily GitHub check, install via pacman with a password prompt.
- Not available on this headset over the dongle: spatial sound, auto-off,
  charging state, volume. Never show controls for them.
- Default window 440×800, minimum 360×520, resizable; two columns from 840px
  wide.

## Brand Commitments

- Name: QuantumEngine. App icon/logo: a violet neon "Q" ring around Tux on
  black (`crates/quantumengined/icon.png`, `packaging/assets/`).
- Voice: plain, precise, no hype (as in the README).
- Must not imitate JBL's branding or QuantumENGINE's interface.

## Evidence on Hand

Real device data shapes only (battery %, firmware strings, serial, paired host
name). No screenshots, testimonials or user counts exist; none may be invented.

## Product Principles

1. State first: the headset's real condition is readable at a glance before any
   control.
2. Truthful controls: never show a state the device has not confirmed, and say
   when something is remembered rather than read.
3. Linux-native: sits comfortably on a Plasma desktop beside the tray and
   widget; no vendor-app mimicry.
4. Gentle with the hardware: one write at a time, nothing fires on its own.
