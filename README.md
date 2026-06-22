# proart-power

A small power-mode controller for the **ASUS ProArt P16** (Ryzen AI 9 HX 370 +
Radeon 890M + RTX 4070). It is the **single owner** of the dynamic power state, so
nothing competes to react to plug/unplug events.

## Why

The stock setup had three things all reacting to power events — cardwire's internal
auto-switch, TLP, and ad-hoc scripts — which fought each other (a manual
`cardwire set` silently disarmed cardwire's own auto-switch, etc.). `proart-power`
replaces that with one trigger → one decision → one coherent apply.

## Model

```
udev (AC plug/unplug) ─┐
boot (multi-user)      ├─► proart-power.service ─► `proart-power apply`
resume (suspend.target)┘                              │
                                                      ├─ reads override (auto|turbo)
                                                      ├─ reads AC online
                                                      └─ applies ONE state:
                                                         platform_profile, CPU boost,
                                                         EPP, charge limit, cardwire GPU
```

- **proart-power** owns: GPU mode (via cardwire), `platform_profile`, CPU boost, EPP, charge limit.
- **TLP** is kept for deep tunables only: PCIe ASPM, USB autosuspend, disk, wifi, audio.
- **cardwire** is used purely as the GPU-block mechanism (its own `battery_auto_switch` is turned **off**).
- **asusctl** stays for fans / keyboard.

## Usage

```
proart-power auto      # automatic battery/AC switching (default)
proart-power turbo     # force max performance + dGPU, even on battery (sticky until 'auto')
proart-power status    # show mode + live power state
```

## Install

```
sudo bash install.sh
```

Builds with `cargo`, installs the binary + systemd units + udev rule, turns off
cardwire's auto-switch, trims the TLP drop-in to deep-tunables-only, and enables
the services. Config lives at `/etc/proart-power.conf`.

## Uninstall

```
sudo systemctl disable --now proart-power.service proart-power-resume.service
sudo make uninstall
```

## Config

`/etc/proart-power.conf` — one `key = value` block per state (`battery`, `ac`,
`turbo`) plus `charge_limit`. See the shipped default for all keys.
