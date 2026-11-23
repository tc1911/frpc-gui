#!/bin/sh
# 在仓库根目录执行：./packaging/linux/build-rpm.sh
set -eu

VERSION=${VERSION:-0.1.0}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TOP=${TOP:-/tmp/frpc-gui-rpmbuild}

mkdir -p "$TOP"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# 直接从 tag 导出源码，避免把 target 也塞进去
( cd "$ROOT" && git archive --format=tar.gz \
    --prefix="frpc-gui-$VERSION/" \
    -o "$TOP/SOURCES/frpc-gui-$VERSION.tar.gz" "v$VERSION" )

cp "$ROOT/packaging/linux/frpc-gui.spec" "$TOP/SPECS/"

rpmbuild -bb --nodeps \
    --define "_topdir $TOP" \
    "$TOP/SPECS/frpc-gui.spec" | tail -5

find "$TOP/RPMS" -name '*.rpm' -exec ls -lh {} \;
