# Maintainer: 1mesles1 <https://github.com/1mesles1>
# Contributor: measles

pkgname=rink
pkgver=0.3.5
pkgrel=1
pkgdesc="A console-based FB2 reader with library management, bookmarks, footnotes and i18n support"
arch=('x86_64' 'aarch64')
url="https://github.com/measles/rink"
license=('GPL3')
depends=('glibc' 'gcc-libs')
makedepends=('cargo' 'rust')
source=("$pkgname-$pkgver.tar.gz::https://github.com/1mesles1/rink/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo build --release --locked
}

package() {
    cd "$srcdir/$pkgname-$pkgver"
    install -Dm755 target/release/$pkgname "$pkgdir/usr/bin/$pkgname"
    
    # Установка man-страницы (если есть)
    # install -Dm644 man/$pkgname.1 "$pkgdir/usr/share/man/man1/$pkgname.1"
    
    # Установка документации
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
