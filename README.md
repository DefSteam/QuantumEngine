<p align="center">
  <img src="quantum-linux/packaging/assets/logo-128.png" width="96" alt="QuantumEngine logo">
</p>

# QuantumEngine for Linux

Unofficial Linux control software for the **JBL Quantum 810 Wireless** headset.
Control noise cancelling, TalkThru, sidetone, lighting and equalizer profiles
without Windows or JBL's QuantumENGINE app.

It includes:

- **QuantumEngine**, a responsive desktop app with a live headset illustration,
  device status, sound controls, lighting editor and equalizer.
- **`quantumengined`**, a background service with a system tray menu.
- **Plasma widget**, with a layout that adapts to horizontal and vertical panels.
- **`quantumenginectl`**, a command-line tool with JSON output for scripts.
- **`jbl-quantum`**, a Rust library for anyone building their own tools.

A background service (`quantumengined`) talks to the USB dongle directly;
the app, widget and CLI talk to it over D-Bus. No Windows or vendor software needed.

## What you can do

- See battery level, microphone state and the game/chat dial position
- Switch noise control: **Off**, **ANC** or **TalkThru**
- Set sidetone (hearing your own voice): **Off**, **Low**, **Mid** or **High**
- Turn the lights on or off
- Set colours, brightness and animation for the logo and ring lighting
- Equalizer: visualize the active curve, choose JBL headset tuning and 19 sound profiles
- Virtual 7.1 surround sound

The desktop layout adapts to the window shape: wide windows place the headset
overview beside the controls, while compact layouts stack the content. The EQ
view keeps its curve and enable control visible alongside the headset on wide
windows, with presets in their own section.

The dongle doesn't expose the headset's hardware volume or auto-off timer. Use
the desktop's system mixer for playback volume. If the mixer slider moves but
playback loudness doesn't change, see the [WirePlumber volume workaround](quantum-linux/README.md#system-volume-slider-moves-but-loudness-does-not).

## Supported hardware

| Headset | USB ID | Status |
|---|---|---|
| JBL Quantum 810 Wireless | `0ecb:2069` | ✅ tested |
| Other JBL Quantum models | — | ❌ not supported |

Other models may use a different protocol, so the tools refuse to talk to them
rather than guess. If you own one and want to help add it, open an issue.

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

To build and install the current checkout directly, run the included scripts:

```sh
cd QuantumEngine/quantum-linux
./build.sh
./install.sh
```

The package installs the udev rule, desktop launcher, tray service and Plasma
widget. Unplug the dongle and plug it back in after installation. Once
installed, the app can check for and install new releases.

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

QuantumEngine was created by [sidmax7](https://github.com/sidmax7). The EQ
profiles come from JBL QuantumENGINE's configuration files, extracted by
[alessandromrc](https://github.com/alessandromrc/jbl-quantum-4-linux) and tested
by [DefSteam](https://github.com/DefSteam). DefSteam also contributed UI and
application improvements.

## License

Licensed under either the [MIT license](quantum-linux/LICENSE-MIT) or the
[Apache License 2.0](quantum-linux/LICENSE-APACHE), whichever you prefer.
