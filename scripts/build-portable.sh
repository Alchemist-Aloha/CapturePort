#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
version=${CAPTUREPORT_VERSION:-0.1.0}
output_dir=${CAPTUREPORT_OUTPUT_DIR:-"$repo_root/dist"}
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

cd "$repo_root"
cargo build --locked --release --package captureport
install -Dm755 target/release/captureport "$stage/captureport/bin/captureport"
install -Dm644 LICENSE "$stage/captureport/LICENSE"
install -Dm644 crates/captureport/assets/fonts/OFL.txt "$stage/captureport/OFL.txt"
install -Dm644 README.md "$stage/captureport/README.md"
install -Dm755 /dev/stdin "$stage/captureport/run-captureport" <<'EOF'
#!/usr/bin/env sh
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$base/bin/captureport" "$@"
EOF
mkdir -p "$output_dir"
artifact="$output_dir/captureport-${version}-$(uname -m)-portable.tar.gz"
tar -C "$stage" -czf "$artifact" captureport
printf 'Wrote %s\n' "$artifact"
