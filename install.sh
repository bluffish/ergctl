#!/usr/bin/env bash
# Live installer: build, place files, establish single-ownership, enable services.
# Also migrates an older proart-power install to ergctl.
# Run:  sudo bash install.sh
set -euo pipefail
[[ $EUID -eq 0 ]] || { echo "Run with sudo: sudo bash install.sh"; exit 1; }

DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_USER="${SUDO_USER:-root}"

# Prerequisite: NVIDIA RTD3 must be enabled or the dGPU can never reach D3cold.
# That's provided by the nvidia-laptop-power-cfg package (modprobe + udev).
if ! pacman -Q nvidia-laptop-power-cfg >/dev/null 2>&1; then
  echo "WARNING: 'nvidia-laptop-power-cfg' is not installed."
  echo "         It enables NVIDIA RTD3 (DynamicPowerManagement + runtime PM);"
  echo "         without it the dGPU will NOT reach D3cold no matter what ergctl does."
  echo "         Install it first (g14 repo: pacman -S nvidia-laptop-power-cfg), then re-run."
  echo
fi

echo "==> Building ergctl (as $BUILD_USER)"
sudo -u "$BUILD_USER" env -C "$DIR" cargo build --release

echo "==> Migrating any previous proart-power install"
systemctl disable --now proart-power.service proart-power-resume.service 2>/dev/null || true
rm -vf /usr/bin/proart-power \
       /usr/lib/systemd/system/proart-power.service \
       /usr/lib/systemd/system/proart-power-resume.service \
       /usr/lib/udev/rules.d/99-proart-power.rules \
       /etc/proart-power.conf.new 2>/dev/null || true
# carry the old config over to the new name if present and ergctl has none yet
if [[ -f /etc/proart-power.conf && ! -f /etc/ergctl.conf ]]; then
  mv -v /etc/proart-power.conf /etc/ergctl.conf
else
  rm -vf /etc/proart-power.conf 2>/dev/null || true
fi
rm -rf /run/proart-power 2>/dev/null || true

echo "==> Installing binary, units, udev rule"
install -Dm755 "$DIR/target/release/ergctl"        /usr/bin/ergctl
install -Dm644 "$DIR/systemd/ergctl.service"        /usr/lib/systemd/system/ergctl.service
install -Dm644 "$DIR/systemd/ergctl-resume.service" /usr/lib/systemd/system/ergctl-resume.service
install -Dm644 "$DIR/udev/99-ergctl.rules"          /usr/lib/udev/rules.d/99-ergctl.rules

if [[ -f /etc/ergctl.conf ]]; then
  echo "    keeping existing /etc/ergctl.conf (new default at .conf.new)"
  install -Dm644 "$DIR/config/ergctl.conf" /etc/ergctl.conf.new
else
  install -Dm644 "$DIR/config/ergctl.conf" /etc/ergctl.conf
fi

echo "==> Establishing single ownership of the dynamic knobs"
# Disable TLP's built-in EPP/profile/boost defaults (empty value = leave alone;
#    see the "use PARAMETER=\"\"" note in /etc/tlp.conf). TLP keeps deep tunables.
TLP_DROPIN=/etc/tlp.d/01-power.conf
if [[ -f "$TLP_DROPIN" ]]; then
  cp -a "$TLP_DROPIN" "${TLP_DROPIN}.bak-$(date +%s)"
fi
cat > "$TLP_DROPIN" <<'EOF'
# ergctl owns platform_profile, CPU boost, EPP, GPU mode and charge limit.
CPU_ENERGY_PERF_POLICY_ON_AC=""
CPU_ENERGY_PERF_POLICY_ON_BAT=""
PLATFORM_PROFILE_ON_AC=""
PLATFORM_PROFILE_ON_BAT=""
CPU_BOOST_ON_AC=""
CPU_BOOST_ON_BAT=""

# --- Deep tunables TLP keeps ---
CPU_SCALING_GOVERNOR_ON_AC=powersave
CPU_SCALING_GOVERNOR_ON_BAT=powersave
PCIE_ASPM_ON_AC=default
PCIE_ASPM_ON_BAT=powersupersave
RUNTIME_PM_ON_AC=auto
RUNTIME_PM_ON_BAT=auto
WIFI_PWR_ON_AC=off
WIFI_PWR_ON_BAT=on
USB_AUTOSUSPEND=1
AHCI_RUNTIME_PM_ON_BAT=auto
SOUND_POWER_SAVE_ON_AC=0
SOUND_POWER_SAVE_ON_BAT=1
EOF
systemctl restart tlp || true

echo "==> Masking nvidia-powerd + cardwired"
# nvidia-powerd (Dynamic Boost) keeps the dGPU's runtime PM from reaching D3cold.
# cardwire is retired: ergctl no longer drives it, and its Integrated-mode block
# traps the dGPU at D0. Both masked so they can't fight RTD3.
systemctl mask --now nvidia-powerd cardwired 2>/dev/null || true

echo "==> Enabling GPU guard (Electron/Chromium default to iGPU)"
/usr/bin/ergctl gpu-guard on || true

echo "==> Enabling audio guard (unbind dGPU HDMI audio that pins the GPU at D0)"
/usr/bin/ergctl audio-guard on || true

echo "==> Allowing passwordless sudo for ergctl (scoped to the binary)"
SUDOERS=/etc/sudoers.d/ergctl
printf '%s ALL=(root) NOPASSWD: /usr/bin/ergctl\n' "$BUILD_USER" > "$SUDOERS"
chown root:root "$SUDOERS"; chmod 0440 "$SUDOERS"
visudo -cf "$SUDOERS" >/dev/null || { rm -f "$SUDOERS"; echo "  (sudoers check failed, skipped)"; }

echo "==> Enabling services"
systemctl daemon-reload
udevadm control --reload
systemctl enable --now ergctl.service
systemctl enable ergctl-resume.service

echo
echo "==> Done. Current state:"
/usr/bin/ergctl status
echo
echo "Launch the cockpit with:  ergctl    (or: sudo ergctl tui)"
