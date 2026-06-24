# Maintainer: bowen <bowenymail@gmail.com>

pkgname=ergctl
pkgver=0.2.0
pkgrel=1
pkgdesc="Power cockpit for the ASUS ProArt P16 — CLI + TUI (battery/AC/turbo)"
arch=('x86_64')
license=('MIT')
depends=('systemd'
         # enables NVIDIA RTD3 (DynamicPowerManagement + runtime PM) — without it
         # the dGPU can't reach D3cold, so it's a hard prerequisite for dGPU sleep.
         'nvidia-laptop-power-cfg'
         # ergctl drives `cardwire set integrated|hybrid` to block/expose the dGPU.
         'cardwire')
optdepends=('tlp: deep power tunables (PCIe/USB/disk/wifi)'
            'asusctl: fan curves and keyboard control')
makedepends=('cargo')
provides=('proart-power')
conflicts=('proart-power')
replaces=('proart-power')
backup=('etc/ergctl.conf')
source=()
sha256sums=()

build() {
    cd "$startdir"
    cargo build --release --locked || cargo build --release
}

package() {
    cd "$startdir"
    install -Dm755 target/release/ergctl        "$pkgdir/usr/bin/ergctl"
    install -Dm644 systemd/ergctl.service        "$pkgdir/usr/lib/systemd/system/ergctl.service"
    install -Dm644 systemd/ergctl-resume.service "$pkgdir/usr/lib/systemd/system/ergctl-resume.service"
    install -Dm644 udev/99-ergctl.rules          "$pkgdir/usr/lib/udev/rules.d/99-ergctl.rules"
    install -Dm644 config/ergctl.conf            "$pkgdir/etc/ergctl.conf"
    install -Dm644 README.md                     "$pkgdir/usr/share/doc/$pkgname/README.md"
}

# NOTE: the package installs files only. To establish the full setup (mask
# nvidia-powerd + cardwired, trim the TLP drop-in, enable gpu-guard + audio-guard,
# passwordless sudo) run install.sh once. Minimal manual enable:
#   systemctl enable --now ergctl.service && systemctl enable ergctl-resume.service
#   systemctl mask --now nvidia-powerd cardwired
#   ergctl gpu-guard on && ergctl audio-guard on
