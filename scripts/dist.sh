#!/usr/bin/env bash
# Builds the release archive of the plugin for one system:
# dist/runnel-plugin-<name>_<version>_<os>_<arch>.zip with the folder <name>/ (plugin.yaml,
# README.md, bin/<name>[.exe], LICENSE, THIRD_PARTY_NOTICES) – docs/dev/PLUGINS.md of Runnel,
# "Namen und Veröffentlichung". The plugin.yaml in the archive names only the program of its
# system. Usage: scripts/dist.sh <rust target>, e.g. x86_64-unknown-linux-gnu or
# x86_64-pc-windows-msvc (on Windows); scripts/sums.sh then writes dist/SHA256SUMS.
# THIRD_PARTY_NOTICES comes from dist/THIRD_PARTY_NOTICES if it exists (the release builds it
# once), else from scripts/third-party-notices.sh.
set -euo pipefail

cd "$(dirname "$0")/.."
target=${1:?usage: scripts/dist.sh <rust target>}
name=$(sed -n 's/^name:[[:space:]]*//p' plugin.yaml)
version=$(sed -n 's/^version:[[:space:]]*//p' plugin.yaml)
case "$target" in
x86_64-pc-windows-*) os=windows exe=$name.exe ;;
x86_64-unknown-linux-*) os=linux exe=$name ;;
*) echo "unsupported target $target" >&2 && exit 2 ;;
esac

mkdir -p dist
[[ -f dist/THIRD_PARTY_NOTICES ]] || scripts/third-party-notices.sh >dist/THIRD_PARTY_NOTICES
cargo build --locked --release --target "$target"
stage=dist/stage/${os}_amd64/$name
rm -rf "dist/stage/${os}_amd64"
mkdir -p "$stage/bin"
cp "target/$target/release/$exe" "$stage/bin/$exe"
# executable only for this system, so the archive needs nothing else
sed '/^executable:/,/^[^[:space:]]/{/^executable:/!{/^[[:space:]]/d}}' plugin.yaml |
	sed "s|^executable:.*|executable: bin/$exe|" >"$stage/plugin.yaml"
cp LICENSE README.md dist/THIRD_PARTY_NOTICES "$stage/"
archive=runnel-plugin-${name}_${version}_${os}_amd64.zip
rm -f "dist/$archive"
if command -v zip >/dev/null; then
	(cd "dist/stage/${os}_amd64" && zip -qrX "../../$archive" "$name")
else
	(cd "dist/stage/${os}_amd64" && 7z a -tzip -bso0 "../../$archive" "$name")
fi
rm -rf "dist/stage/${os}_amd64"
echo "dist/$archive"
