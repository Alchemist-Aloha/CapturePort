#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
version=${CAPTUREPORT_VERSION:-0.1.0}
output_dir=${CAPTUREPORT_OUTPUT_DIR:-"$repo_root/dist"}
if [[ ! "$version" =~ ^[0-9][0-9A-Za-z.]*$ ]]; then
  printf 'RPM version must start with a digit and contain only letters, digits, and dots\n' >&2
  exit 1
fi
if ! command -v rpmbuild >/dev/null; then
  printf 'rpmbuild (rpm-build) is required to build an RPM package\n' >&2
  exit 1
fi
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

cd "$repo_root"
cargo build --locked --release --package captureport
mkdir -p "$stage/SOURCES" "$stage/SPECS" "$output_dir"
install -m755 target/release/captureport "$stage/SOURCES/captureport"
install -m644 LICENSE "$stage/SOURCES/LICENSE"
install -m644 packaging/captureport.desktop "$stage/SOURCES/captureport.desktop"
cat > "$stage/SPECS/captureport.spec" <<EOF
Name: captureport
Version: $version
Release: 1%{?dist}
Summary: Linux photo and video ingest application
License: MIT
Source0: captureport
Source1: LICENSE
Source2: captureport.desktop
Requires: libgphoto2
Requires: vulkan-loader

# Rust is built by Cargo above; retain automatic shared-library requirements.
%global debug_package %{nil}

%description
CapturePort discovers photos and videos on cameras and removable storage,
then previews and imports them safely.

%prep

%build

%install
install -Dm755 %{SOURCE0} %{buildroot}%{_bindir}/captureport
install -Dm644 %{SOURCE1} %{buildroot}%{_licensedir}/captureport/LICENSE
install -Dm644 %{SOURCE2} %{buildroot}%{_datadir}/applications/captureport.desktop

%files
%{_bindir}/captureport
%license %{_licensedir}/captureport/LICENSE
%{_datadir}/applications/captureport.desktop
EOF
rpmbuild --define "_topdir $stage" -bb "$stage/SPECS/captureport.spec"
for package in "$stage"/RPMS/*/*.rpm; do
  install -m644 "$package" "$output_dir/"
  printf 'Wrote %s/%s\n' "$output_dir" "$(basename "$package")"
done
