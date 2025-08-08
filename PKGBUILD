# Maintainer: Your Name <abert036@uottawa.ca>

_pkgname=sergw
pkgname=sergw-git
pkgver=r0.0.0
pkgrel=1
pkgdesc="Simple Serial to TCP Gateway (serial port to TCP bridge)"
arch=('x86_64' 'aarch64' 'armv7h' 'armv6h' 'i686')
url="https://github.com/seofernando25/sergw"
license=('GPL-3.0-or-later')
depends=()
makedepends=('git' 'rust' 'cargo')
provides=("${_pkgname}")
conflicts=("${_pkgname}")
source=("git+https://github.com/seofernando25/sergw.git")
sha256sums=('SKIP')

pkgver() {
  cd "${srcdir}/${_pkgname}" || exit 1
  printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

prepare() {
  cd "${srcdir}/${_pkgname}"
}

build() {
  cd "${srcdir}/${_pkgname}"
  cargo build --release
}

check() {
  cd "${srcdir}/${_pkgname}"
  cargo test --release
}

package() {
  cd "${srcdir}/${_pkgname}"
  install -Dm755 "target/release/${_pkgname}" "${pkgdir}/usr/bin/${_pkgname}"
  install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${_pkgname}/LICENSE"
  install -Dm644 README.md "${pkgdir}/usr/share/doc/${_pkgname}/README.md"
}
