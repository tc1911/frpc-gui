#!/bin/sh
# 在仓库根目录执行：./packaging/linux/build-deb.sh
set -eu

VERSION=${VERSION:-0.1.1}
ARCH=${ARCH:-amd64}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT="$ROOT/target/deb"
PKG="$OUT/frpc-gui_${VERSION}_${ARCH}"

test -x "$ROOT/target/release/frpc-gui" || { echo "先 cargo build --release"; exit 1; }

rm -rf "$PKG"
mkdir -p "$PKG/DEBIAN" \
         "$PKG/usr/bin" \
         "$PKG/usr/share/applications" \
         "$PKG/usr/share/icons/hicolor/512x512/apps" \
         "$PKG/usr/share/icons/hicolor/scalable/apps" \
         "$PKG/usr/share/doc/frpc-gui"

install -m755 "$ROOT/target/release/frpc-gui" "$PKG/usr/bin/frpc-gui"
install -m644 "$ROOT/packaging/linux/frpc-gui.desktop" "$PKG/usr/share/applications/frpc-gui.desktop"
install -m644 "$ROOT/assets/icon.png" "$PKG/usr/share/icons/hicolor/512x512/apps/frpc-gui.png"
install -m644 "$ROOT/assets/icon.svg" "$PKG/usr/share/icons/hicolor/scalable/apps/frpc-gui.svg"
install -m644 "$ROOT/LICENSE" "$PKG/usr/share/doc/frpc-gui/copyright"

SIZE=$(du -sk "$PKG/usr" | cut -f1)
cat > "$PKG/DEBIAN/control" <<EOF
Package: frpc-gui
Version: $VERSION
Section: net
Priority: optional
Architecture: $ARCH
Installed-Size: $SIZE
Depends: libc6, libgcc-s1, libxau6, libxcb1, libxdmcp6, libxkbcommon0, libxkbcommon-x11-0
Maintainer: tc191 <tc191_dev@163.com>
Homepage: https://github.com/tc1911/frpc-gui
Description: frpc 的图形前端
 基于 GPUI 写的 frpc 客户端，主页启停隧道，设置页填服务器与端口映射。
 守护进程常驻，关掉界面不影响已建立的隧道。
EOF

( cd "$PKG" && find usr -type f -print0 | xargs -0 md5sum > DEBIAN/md5sums )

cd "$OUT"
printf '2.0\n' > debian-binary
( cd "$PKG" && tar czf ../control.tar.gz ./DEBIAN && tar czf ../data.tar.gz ./usr )
rm -f "frpc-gui_${VERSION}_${ARCH}.deb"
ar rc "frpc-gui_${VERSION}_${ARCH}.deb" debian-binary control.tar.gz data.tar.gz
echo "生成 $OUT/frpc-gui_${VERSION}_${ARCH}.deb"
