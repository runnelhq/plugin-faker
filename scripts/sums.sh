#!/usr/bin/env bash
# Writes dist/SHA256SUMS for the release archives in dist/.
set -euo pipefail

cd "$(dirname "$0")/../dist"
sha256sum ./*.zip | sed 's| \./| |' >SHA256SUMS
cat SHA256SUMS
