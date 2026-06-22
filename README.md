# ergctl

A power **cockpit** for the **ASUS ProArt P16** (Ryzen AI 9 HX 370 + Radeon 890M +
RTX 4070). It is the **single owner** of the dynamic power state — nothing else
competes to react to plug/unplug events — and ships a TUI to see and drive it all.

## Why

The stock setup had three things all reacting to power events — cardwire's internal
auto-switch, TLP, and ad-hoc scripts — which fought each other (a manual
`cardwire set` silently disarmed cardwire's own auto-switch; TLP's built-in
defaults stomped the platform profile; `nvidia-powerd` and cardwire's experimental
nvidia block both kept the dGPU awake). `ergctl` replaces that with one trigger →
one decision → one coherent apply.

## Model

```
udev (AC plug/unplug) ─┐
boot (multi-user)      ├─► ergctl.service ─► `ergctl apply`
resume (suspend.target)┘                        │
                                                ├─ reads override (auto|turbo)
                                                ├─ reads AC online
                                                └─ applies ONE state:
                                                   platform_profile, CPU boost, EPP,
                                                   charge limit, cardwire GPU, nvidia-powerd
```

- **ergctl** owns: GPU mode (cardwire), `platform_profile`, CPU boost, EPP, charge
  limit, and `nvidia-powerd` (stopped when the dGPU is integrated so it can RTD3).
- **TLP** kept for deep tunables only: PCIe ASPM, USB autosuspend, disk, wifi, audio.
  Its built-in EPP/profile/boost defaults are explicitly disabled.
- **cardwire** is the GPU-block mechanism; its `battery_auto_switch` and
  `experimental_nvidia_block` are both **off** (the latter wedges NVIDIA RTD3).
- **asusctl** stays for fans / keyboard.

## TUI

`ergctl` with no arguments (in a terminal) opens the cockpit: live battery + SoC
wattage with a sparkline, CPU/GPU/battery/system panels, and one-key control:

```
[a]uto  [t]urbo   [p]rofile  [b]oost  [g]pu   [ ] charge   -/= bright   [q]uit
```

## CLI

```
ergctl            # open the TUI (when interactive)
ergctl auto       # automatic battery/AC switching (default)
ergctl turbo      # force max performance + dGPU, even on battery (sticky until 'auto')
ergctl status     # print mode + live power state
ergctl apply      # re-apply for current override/power source (used by the service)
```

Mutating commands and the TUI self-elevate via `sudo`.

## Install

```
sudo bash install.sh
```

Builds with `cargo`, migrates any previous `proart-power` install, installs the
binary + systemd units + udev rule, sets cardwire/TLP single-ownership, and
enables the services. Config lives at `/etc/ergctl.conf`.

## Uninstall

```
sudo systemctl disable --now ergctl.service ergctl-resume.service
sudo make uninstall
```

## Config

`/etc/ergctl.conf` — one `key = value` block per state (`battery`, `ac`, `turbo`)
plus `charge_limit`. See the shipped default for all keys.
