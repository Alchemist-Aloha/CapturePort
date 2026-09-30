#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
version=${CAPTUREPORT_VERSION:-0.2.0}
output_dir=${CAPTUREPORT_OUTPUT_DIR:-"$repo_root/dist"}
package_root=$(mktemp -d)
metadata_root=$(mktemp -d)
trap 'rm -rf "$package_root" "$metadata_root"' EXIT

cd "$repo_root"
cargo build --locked --release --package captureport
install -Dm755 target/release/captureport "$package_root/usr/bin/captureport"
install -Dm644 LICENSE "$package_root/usr/share/doc/captureport/copyright"
install -Dm644 crates/captureport/assets/fonts/Outfit-OFL.txt \
  "$package_root/usr/share/doc/captureport/Outfit-OFL.txt"
install -Dm644 crates/captureport/assets/fonts/Spectral-OFL.txt \
  "$package_root/usr/share/doc/captureport/Spectral-OFL.txt"
install -Dm644 crates/captureport/assets/icons/LICENSE.txt \
  "$package_root/usr/share/doc/captureport/Apache-2.0-Material-Symbols.txt"
install -Dm644 packaging/captureport.desktop \
  "$package_root/usr/share/applications/captureport.desktop"
mkdir -p "$metadata_root/debian" "$package_root/DEBIAN"
cp packaging/debian/control "$metadata_root/debian/control"
if ! command -v dpkg-shlibdeps >/dev/null; then
  printf 'dpkg-shlibdeps (dpkg-dev) is required to calculate package dependencies\n' >&2
  exit 1
fi
dependencies=$(cd "$metadata_root" && dpkg-shlibdeps -O -e"$repo_root/target/release/captureport" | sed -n 's/^shlibs:Depends=//p')
if [[ -z "$dependencies" ]]; then
  printf 'Could not calculate shared-library dependencies\n' >&2
  exit 1
fi
architecture=$(dpkg --print-architecture 2>/dev/null || printf '%s' amd64)
cat > "$package_root/DEBIAN/control" <<EOF
Package: captureport
Version: $version
Section: graphics
Priority: optional
Architecture: $architecture
Maintainer: CapturePort contributors <maintainers@example.invalid>
Depends: $dependencies
Description: Linux photo and video ingest application
 CapturePort discovers photos and videos on cameras and removable storage,
 then previews and imports them safely.
EOF
mkdir -p "$output_dir"
artifact="$output_dir/captureport_${version}_${architecture}.deb"
dpkg-deb --build --root-owner-group "$package_root" "$artifact"
printf 'Wrote %s\n' "$artifact"
