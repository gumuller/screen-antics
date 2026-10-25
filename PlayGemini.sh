#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
exec ./Play.sh --style remaster --asset-pack "$PWD/assets/gemini-remaster" "$@"
