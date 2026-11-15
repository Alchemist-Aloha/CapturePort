#!/usr/bin/env bash
# Build a self-contained CapturePort AppImage with linuxdeploy.
#
# libgphoto2 loads camera drivers (camlibs) and port drivers (iolibs) as
# plugins from build-time paths, so they are bundled with their dependencies
# and AppRun points CAMLIBS/IOLIBS at the bundled copies. Vulkan, Wayland, the
# font stack and glibc come from the host, as for every AppImage.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
version=${CAPTUREPORT_VERSION:-0.6.0}
output_dir=${CAPTUREPORT_OUTPUT_DIR:-"$repo_root/dist"}
arch=$(uname -m)
linuxdeploy_tag=1-alpha-20251107-1
linuxdeploy_sha256=c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d
if [[ "$arch" != x86_64 ]]; then
  printf 'The pinned linuxdeploy checksum is for x86_64 only\n' >&2
  exit 1
fi
for command in curl sha256sum pkg-config; do
  command -v "$command" >/dev/null || {
    printf 'Required command not found: %s\n' "$command" >&2
    exit 1
  }
done
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

cd "$repo_root"
cargo build --locked --release --package captureport

tool="$stage/linuxdeploy-$arch.AppImage"
curl --proto '=https' -fsSL -o "$tool" \
  "https://github.com/linuxdeploy/linuxdeploy/releases/download/$linuxdeploy_tag/linuxdeploy-$arch.AppImage"
printf '%s  %s\n' "$linuxdeploy_sha256" "$tool" | sha256sum -c --quiet
chmod +x "$tool"

appdir="$stage/CapturePort.AppDir"
libdir=$(pkg-config --variable=libdir libgphoto2)
camlibs="$libdir/libgphoto2/$(pkg-config --modversion libgphoto2)"
iolibs=$(pkg-config --variable=driverdir libgphoto2_port)
for dir in "$camlibs" "$iolibs"; do
  compgen -G "$dir/*.so" >/dev/null || {
    printf 'No libgphoto2 drivers found in %s\n' "$dir" >&2
    exit 1
  }
done
install -d "$appdir/usr/lib/libgphoto2/camlibs" "$appdir/usr/lib/libgphoto2/iolibs"
cp "$camlibs"/*.so "$appdir/usr/lib/libgphoto2/camlibs/"
cp "$iolibs"/*.so "$appdir/usr/lib/libgphoto2/iolibs/"

install -Dm644 LICENSE "$appdir/usr/share/doc/captureport/LICENSE"
install -Dm644 crates/captureport/assets/fonts/Outfit-OFL.txt \
  "$appdir/usr/share/doc/captureport/Outfit-OFL.txt"
install -Dm644 crates/captureport/assets/fonts/Spectral-OFL.txt \
  "$appdir/usr/share/doc/captureport/Spectral-OFL.txt"
install -Dm644 crates/captureport/assets/icons/LICENSE.txt \
  "$appdir/usr/share/doc/captureport/Apache-2.0-Material-Symbols.txt"

install -Dm755 /dev/stdin "$stage/AppRun" <<'EOF'
#!/bin/sh
set -eu
here=$(dirname "$(readlink -f "$0")")
# Use the bundled gphoto2 drivers; system ones may not exist or may not match.
export CAMLIBS="$here/usr/lib/libgphoto2/camlibs"
export IOLIBS="$here/usr/lib/libgphoto2/iolibs"
exec "$here/usr/bin/captureport" "$@"
EOF

mkdir -p "$output_dir"
artifact="$output_dir/captureport-${version}-${arch}.AppImage"
# Containers have no FUSE; linuxdeploy can run from its extracted payload.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$arch" LDAI_OUTPUT="$artifact" \
  "$tool" --appdir "$appdir" \
  --executable target/release/captureport \
  --desktop-file packaging/captureport.desktop \
  --icon-file crates/captureport/assets/captureport.svg \
  --custom-apprun "$stage/AppRun" \
  --deploy-deps-only "$appdir/usr/lib/libgphoto2/camlibs" \
  --deploy-deps-only "$appdir/usr/lib/libgphoto2/iolibs" \
  --output appimage
printf 'Wrote %s\n' "$artifact"
