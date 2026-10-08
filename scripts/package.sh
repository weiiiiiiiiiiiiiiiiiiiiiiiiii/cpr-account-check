#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
plugin_cli="${PLUGIN_CLI:-cpr-plugin}"
plugin_target="${1:-x86_64-unknown-linux-gnu}"
case "$plugin_target" in
  x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|aarch64-apple-darwin) ;;
  *) echo "不支持的插件目标平台：$plugin_target" >&2; exit 1 ;;
esac
rustup target add "$plugin_target"
npm --prefix frontend ci --legacy-peer-deps
npm --prefix frontend run build
cargo build --manifest-path backend/Cargo.toml --release --locked --target "$plugin_target"
plugin_build_dir="${CARGO_TARGET_DIR:-backend/target}"
"$plugin_cli" package --manifest plugin.json \
  --binary "$plugin_build_dir/$plugin_target/release/cpr-account-check" \
  --target "$plugin_target" --resource-map web=frontend/dist --output-dir dist
