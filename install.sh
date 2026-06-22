#!/usr/bin/env bash
# Live installer: build, place files, establish single-ownership, enable services.
# Run:  sudo bash install.sh
set -euo pipefail
[[ $EUID -eq 0 ]] || { echo "Run with sudo: sudo bash install.sh"; exit 1; }

DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_USER="${SUDO_USER:-root}"

echo "==> Building (as $BUILD_USER)"
sudo -u "$BUILD_USER" env -C "$DIR" cargo build --release

echo "==> Installing binary, units, udev rule"
install -Dm755 "$DIR/target/release/proart-power"        /usr/bin/proart-power
install -Dm644 "$DIR/systemd/proart-power.service"        /usr/lib/systemd/system/proart-power.service
install -Dm644 "$DIR/systemd/proart-power-resume.service" /usr/lib/systemd/system/proart-power-resume.service
install -Dm644 "$DIR/udev/99-proart-power.rules"          /usr/lib/udev/rules.d/99-proart-power.rules

if [[ -f /etc/proart-power.conf ]]; then
  echo "    keeping existing /etc/proart-power.conf (new default at .conf.new)"
  install -Dm644 "$DIR/config/proart-power.conf" /etc/proart-power.conf.new
else
  install -Dm644 "$DIR/config/proart-power.conf" /etc/proart-power.conf
fi

echo "==> Establishing single ownership of the dynamic knobs"
# 1) cardwire stops auto-switching — proart-power drives it.
cardwire config battery-auto-switch false || true
cardwire config save || true

# 2) Strip overlapping keys from the TLP drop-in (we own profile/boost/EPP now);
#    TLP keeps only the deep tunables it does well.
TLP_DROPIN=/etc/tlp.d/01-power.conf
if [[ -f "$TLP_DROPIN" ]]; then
  cp -a "$TLP_DROPIN" "${TLP_DROPIN}.bak-$(date +%s)"
fi
cat > "$TLP_DROPIN" <<'EOF'
# Deep tunables only — proart-power owns platform_profile, CPU boost, EPP,
# GPU mode and charge limit. Do not re-add those keys here (single ownership).
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

echo "==> Enabling services"
systemctl daemon-reload
udevadm control --reload
systemctl enable --now proart-power.service
systemctl enable proart-power-resume.service

echo
echo "==> Done. Current state:"
/usr/bin/proart-power status
