# Maintainer: bowen <yunyang@gmail.com>

pkgname=ergctl
pkgver=0.2.0
pkgrel=1
pkgdesc="Power cockpit for the ASUS ProArt P16 — CLI + TUI (battery/AC/turbo)"
arch=('x86_64')
license=('MIT')
depends=('cardwire' 'systemd')
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

# NOTE: the package installs files only. To establish single-ownership
# (disable cardwire auto-switch + nvidia block, trim the TLP drop-in) and enable
# the services, run install.sh once, or do it manually:
#   systemctl enable --now ergctl.service
#   systemctl enable ergctl-resume.service
#   cardwire config battery-auto-switch false && cardwire config experimental-nvidia-block false && cardwire config save
