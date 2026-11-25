#!/usr/bin/env bash
# Prints the license texts of all third-party software compiled into the plugin: the crates
# of every release target (without build-time crates such as proc macros) and the Rust
# standard library. Required by the licenses of those dependencies (ADR 0004 of Runnel).
# Fails if a crate has no license file. Needs jq.
set -euo pipefail

cd "$(dirname "$0")/.."
name=$(sed -n 's/^name:[[:space:]]*//p' plugin.yaml)
targets=${TARGETS:-"x86_64-pc-windows-msvc x86_64-unknown-linux-gnu"}

cat <<HEADER
Third-party software notices for the Runnel plugin $name
$(printf '=%.0s' $(seq 1 $((44 + ${#name}))))

The plugin includes the following third-party software. Their license texts follow.
HEADER

# Plain text for the names (CI sets CARGO_TERM_COLOR=always, which colors cargo tree).
export CARGO_TERM_COLOR=never
metadata=$(cargo metadata --format-version 1 --locked)
own=$(jq -r '.resolve.root' <<<"$metadata")
# Union of the crates of all release targets (some are platform-specific).
for t in $targets; do
	cargo tree --locked -e normal,no-proc-macro --target "$t" --prefix none --format '{p}' | sed 's/ (.*//; s/ (\*)$//'
done | sort -u | while read -r crate version; do
	[[ -z "$crate" ]] && continue
	dir=$(jq -r --arg n "$crate" --arg v "${version#v}" \
		'.packages[] | select(.name == $n and .version == $v) | .manifest_path' <<<"$metadata" | xargs dirname)
	id=$(jq -r --arg n "$crate" --arg v "${version#v}" '.packages[] | select(.name == $n and .version == $v) | .id' <<<"$metadata")
	[[ "$id" == "$own" ]] && continue
	license=$(jq -r --arg n "$crate" --arg v "${version#v}" '.packages[] | select(.name == $n and .version == $v) | .license // "unknown"' <<<"$metadata")
	printf '\n\n--------------------------------------------------------------------------------\n'
	printf '%s %s (%s)\n' "$crate" "$version" "$license"
	printf -- '--------------------------------------------------------------------------------\n\n'
	found=0
	for f in "$dir"/LICENSE* "$dir"/LICENCE* "$dir"/COPYING* "$dir"/NOTICE*; do
		[[ -f "$f" ]] || continue
		cat "$f"
		echo
		found=1
	done
	if [[ $found -eq 0 ]]; then
		echo "ERROR: no license file found for $crate $version" >&2
		exit 1
	fi
done

sysroot=$(rustc --print sysroot)
printf '\n\n--------------------------------------------------------------------------------\n'
printf 'Rust standard library %s (MIT OR Apache-2.0), Copyright (c) The Rust Project Contributors\n' "$(rustc --version | cut -d' ' -f2)"
printf -- '--------------------------------------------------------------------------------\n\n'
cat "$sysroot/share/doc/rust/licenses/MIT.txt"
