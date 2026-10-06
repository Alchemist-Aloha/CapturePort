#!/usr/bin/env sh
set -eu

if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
  printf 'CapturePort release binaries support Linux x86_64 only.\n' >&2
  exit 1
fi
for command in curl sha256sum awk mktemp install; do
  command -v "$command" >/dev/null 2>&1 || {
    printf 'Required command not found: %s\n' "$command" >&2
    exit 1
  }
done

stage=$(mktemp -d)
pending=
trap 'rm -rf "$stage"; if [ -n "$pending" ]; then rm -f "$pending"; fi' EXIT
trap 'exit 1' HUP INT TERM

# Resolve latest once so a release published mid-install cannot mix assets.
release_url=$(curl --proto '=https' --proto-redir '=https' -fsSL \
  -w '%{url_effective}' -o /dev/null \
  https://github.com/Alchemist-Aloha/CapturePort/releases/latest)
case "$release_url" in
  https://github.com/Alchemist-Aloha/CapturePort/releases/tag/*) ;;
  *) printf 'Could not resolve the latest GitHub release.\n' >&2; exit 1 ;;
esac
base="https://github.com/Alchemist-Aloha/CapturePort/releases/download/${release_url##*/}"
asset=captureport-linux-x86_64
curl --proto '=https' --proto-redir '=https' -fsSL "$base/SHA256SUMS" -o "$stage/SHA256SUMS"
curl --proto '=https' --proto-redir '=https' -fsSL "$base/$asset" -o "$stage/$asset"
awk -v asset="$asset" '$2 == asset { print }' "$stage/SHA256SUMS" > "$stage/binary.sha256"
if [ ! -s "$stage/binary.sha256" ]; then
  printf 'Release checksum for %s is missing.\n' "$asset" >&2
  exit 1
fi
(cd "$stage" && sha256sum -c binary.sha256)

bin_dir="$HOME/.local/bin"
mkdir -p "$bin_dir"
# Publish on the destination filesystem without truncating a running binary.
pending=$(mktemp "$bin_dir/.captureport.XXXXXX")
install -m755 "$stage/$asset" "$pending"
mv -fT "$pending" "$bin_dir/captureport"
pending=
printf 'Installed %s/captureport\n' "$bin_dir"
case ":${PATH:-}:" in
  *":$bin_dir:"*) ;;
  *) printf 'Add to your shell profile: export PATH="$HOME/.local/bin:$PATH"\n' ;;
esac
