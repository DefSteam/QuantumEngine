<p align="center">
  <img src="quantum-linux/packaging/assets/logo-128.png" width="96" alt="QuantumEngine logo">
</p>

# QuantumEngine for Linux

Unofficial Linux control software for the **JBL Quantum 810 Wireless** headset.
You get noise cancelling, TalkThru, sidetone and RGB lighting without Windows or
JBL's QuantumENGINE app.

It includes:

- **QuantumEngine**, a desktop app that follows the headset live.
- **`quantumengined`**, a background service with a system tray menu.
- **Plasma widget**, for KDE Plasma panel and desktop control.
- **`quantumenginectl`**, a command-line tool with JSON output for scripts.
- **`jbl-quantum`**, a Rust library for anyone building their own tools.

A background service (`quantumengined`) talks to the USB dongle directly;
the app, widget and CLI talk to it over D-Bus. No Windows or vendor software needed.

## What you can do

- See battery level, microphone state and the game/chat dial position
- Switch noise control: **Off**, **ANC** or **TalkThru**
- Set sidetone (hearing your own voice): **Off**, **Low**, **Mid** or **High**
- Turn the lights on or off
- Set colours, brightness and animation for the logo and ring lighting (RGB works, but is still under development; see below)
- Equalizer: JBL headset tuning and 19 sound profiles
- Virtual 7.1 surround sound

Auto-off timer and hardware volume aren't supported over the dongle.

> **RGB lighting is still under development.** Setting colours, brightness and
> effects on the headset works, but the app's lighting editor and the headset
> drawing don't always show what the headset is doing. The headset can't report
> its lighting, so the window only knows what it last applied itself. Expect
> rough edges here, and treat what the window shows as a preview, not a
> readout. Fixes are planned.

## Supported hardware

| Headset | USB ID | Status |
|---|---|---|
| JBL Quantum 810 Wireless | `0ecb:2069` | ✅ tested |
| Other Harman / JBL Quantum dongles (for example the Quantum 800, `0ecb:203e`) | — | ⚠️ unverified, opt-in |

Only the 810 has been tested. Other models might share the protocol, share part
of it, or not share it at all, and there is no way to know without trying. So
QuantumEngine doesn't ignore them and doesn't guess either: if it finds another
Harman headset dongle, the window's **Device** tab offers to check it, safely:

1. **Check what it answers.** Reads only. Nothing is written to the headset.
   You get a plain-text report to paste into an issue, which is how a model
   moves from unverified to supported.
2. **Use this headset.** Shows what the headset reports, read-only.
3. **Allow changes.** Noise control, sidetone and lights, each off until you turn
   it on, because they send commands built for the 810.

Opening the device may need permission first. The window's **Allow access…**
button (or `quantumenginectl unverified access`) adds a udev rule for that one
device ID after you enter your password. The same steps are available as
`quantumenginectl unverified`, `unverified probe`, `unverified use on` and
`unverified allow anc sidetone lights`.

## Install

### Arch / CachyOS / EndeavourOS / Manjaro (Recommended)

Download the `.pkg.tar.zst` from the
[latest release](https://github.com/sidmax7/QuantumEngine/releases/latest) and
install it:

```sh
sudo pacman -U quantumengine-*-x86_64.pkg.tar.zst
```

Or build it from source with `makepkg`:

```sh
git clone https://github.com/sidmax7/QuantumEngine.git
cd QuantumEngine/quantum-linux/packaging
makepkg -si
```

The package installs the udev rule, desktop launcher, tray service and Plasma widget for you.
Unplug the dongle and plug it back in afterwards. Once installed, the app updates itself.

### Anywhere else

You need a Rust toolchain (1.88 or newer), Node.js with npm, and WebKitGTK 4.1
(`webkit2gtk-4.1` on Arch, `libwebkit2gtk-4.1-dev` on Debian and Ubuntu).

```sh
git clone https://github.com/sidmax7/QuantumEngine.git
cd QuantumEngine/quantum-linux
npm --prefix ui ci
npm --prefix ui run build
cargo build --release --workspace --features quantumengine/custom-protocol
sudo install -Dm755 target/release/quantumengine target/release/quantumengined \
  target/release/quantumenginectl -t /usr/local/bin/

sudo install -Dm644 udev/70-jbl-quantum.rules /etc/udev/rules.d/70-jbl-quantum.rules
sudo udevadm control --reload
```

Then replug the dongle. Now start **QuantumEngine** from your app menu, or run a command:

```sh
quantumenginectl                 # show everything
quantumenginectl anc talkthru    # switch to TalkThru
quantumenginectl lights off
```

Full install notes, command reference and troubleshooting are in
[quantum-linux/README.md](quantum-linux/README.md).

## Repository layout

```
quantum-linux/
├── crates/
│   ├── jbl-quantum/          library: device protocol and headset API
│   ├── quantumengine-dbus/   D-Bus IPC interface
│   ├── quantumengined/       background service and system tray
│   ├── quantumenginectl/     command-line tool
│   └── quantumengine/        desktop app (Tauri)
├── packaging/                PKGBUILD, desktop entries, Plasma widget
├── udev/                     device permission rule
└── ui/                       Vue 3 frontend
```

## How it works

The dongle exposes a vendor-specific HID interface. Settings are read and
written with HID feature reports, and the headset sends events when you use its
physical controls. The protocol was worked out on Linux from the device itself,
and every command the tools send has been tested on real hardware.

The firmware-update commands aren't defined anywhere in the code, so no bug in
these tools can reach them.

## Disclaimer

This is an independent project and isn't affiliated with or endorsed by JBL or
Harman International. "JBL" and "Quantum" are their trademarks. You use this
software at your own risk.

## Credits

The curves come from JBL QuantumENGINE's configuration files, as extracted by
[alessandromrc](https://github.com/alessandromrc/jbl-quantum-4-linux) and
tested by [DefSteam](https://github.com/DefSteam). Thank you both.

## License

Licensed under either the [MIT license](quantum-linux/LICENSE-MIT) or the
[Apache License 2.0](quantum-linux/LICENSE-APACHE), whichever you prefer.
