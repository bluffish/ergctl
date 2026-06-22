# Maintainer: bowen <yunyang@gmail.com>

pkgname=proart-power
pkgver=0.1.0
pkgrel=1
pkgdesc="Power mode controller for the ASUS ProArt P16 (battery/AC/turbo)"
arch=('x86_64')
license=('MIT')
depends=('cardwire' 'systemd')
optdepends=('tlp: deep power tunables (PCIe/USB/disk/wifi)'
            'asusctl: fan curves and keyboard control')
makedepends=('cargo')
backup=('etc/proart-power.conf')
source=()
sha256sums=()

build() {
    cd "$startdir"
    cargo build --release --locked || cargo build --release
}

package() {
    cd "$startdir"
    install -Dm755 target/release/proart-power        "$pkgdir/usr/bin/proart-power"
    install -Dm644 systemd/proart-power.service        "$pkgdir/usr/lib/systemd/system/proart-power.service"
    install -Dm644 systemd/proart-power-resume.service "$pkgdir/usr/lib/systemd/system/proart-power-resume.service"
    install -Dm644 udev/99-proart-power.rules          "$pkgdir/usr/lib/udev/rules.d/99-proart-power.rules"
    install -Dm644 config/proart-power.conf            "$pkgdir/etc/proart-power.conf"
    install -Dm644 README.md                           "$pkgdir/usr/share/doc/$pkgname/README.md"
}

# NOTE: the package installs files only. To establish single-ownership
# (disable cardwire auto-switch, trim the TLP drop-in) and enable the services,
# run install.sh once, or do it manually:
#   systemctl enable --now proart-power.service
#   systemctl enable proart-power-resume.service
#   cardwire config battery-auto-switch false && cardwire config save
