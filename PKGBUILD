# Maintainer: 1mesles1 <https://github.com/1mesles1>
# Contributor: measles

pkgname=rinx
pkgver=0.4.8
pkgrel=1
pkgdesc="A console-based FB2 reader with library management, bookmarks, footnotes and i18n support"
arch=('x86_64' 'aarch64')
url="https://github.com/measles/rinx"
license=('GPL3')
depends=('glibc' 'gcc-libs' 'zstd')
makedepends=('cargo' 'rust')
source=("$pkgname-$pkgver.tar.gz::https://github.com/1mesles1/rinx/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/$pkgname-$pkgver"
    export ZSTD_SYS_USE_PKG_CONFIG=1
    cargo build --release
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
