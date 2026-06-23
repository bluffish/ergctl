# ergctl

Linux power management for the ASUS ProArt P16 (Ryzen AI 9 HX 370 + Radeon 890M
iGPU + RTX 4070 dGPU). Keeps the dGPU asleep on battery, switches power profiles
automatically on AC/battery, and exposes a TUI and a Waybar module.

## Features

- **Auto power states** — applies a coherent profile on every plug/unplug, boot,
  and resume: ACPI platform profile, CPU boost, EPP, and battery charge limit.
- **dGPU sleep** — keeps the NVIDIA dGPU in D3cold via native RTD3 plus two guards:
  - `gpu-guard` points GL/EGL/Vulkan at the iGPU so apps don't wake the dGPU.
  - `audio-guard` removes the dGPU HDMI-audio function that otherwise pins it at D0.
- **TUI cockpit** — live power draw, CPU/GPU/battery panels, one-key controls.
- **Waybar module** — a pill showing power draw + dGPU state (red when awake).
- **CLI** — scriptable; drives the systemd units on boot/resume/power events.

## Install

### Requirements

- **`nvidia-laptop-power-cfg`** (in the [`g14`](https://github.com/Frogging-Family/community-db)
  repo) — enables NVIDIA RTD3 (DynamicPowerManagement + runtime PM). **Required**:
  without it the dGPU can't reach D3cold, so nothing here can put it to sleep.
- Optional: `tlp` (deep tunables), `asusctl` (fans/keyboard).

```sh
git clone https://github.com/bluffish/ergctl
cd ergctl
sudo bash install.sh
```

Builds with `cargo`, installs the binary + systemd units + udev rule, and wires up
the guards. An Arch `PKGBUILD` is included.

## Usage

```sh
ergctl              # open the TUI (when run in a terminal)
ergctl auto         # automatic battery/AC switching (default)
ergctl turbo        # force max performance, even on battery
ergctl status       # print current mode and live power state
ergctl waybar       # emit Waybar JSON (power draw + dGPU state)
ergctl gpu-guard   {on|off|status}
ergctl audio-guard {on|off|status}
```

Mutating commands self-elevate via `sudo`.

## Configuration

`/etc/ergctl.conf` — one block per state (`battery`, `ac`, `turbo`) for the
platform profile, CPU boost, and EPP, plus the charge limit.

## Waybar / Hyprland

See `contrib/waybar/` for the module definition and styling.

## License

MIT
