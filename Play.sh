#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
for pack in assets/gemini-remaster assets/hd2d assets/hd2d-v2; do
    if [[ -d "$pack/sprites" || -f "$pack/scene-art.tsv" ]]; then
        python3 tools/prepare_asset_pack.py "$pack"
    fi
done
rebuild=false
if [[ ! -x target/release/screen-antics ]]; then
    rebuild=true
else
    for source in Cargo.toml src/*.rs; do
        if [[ "$source" -nt target/release/screen-antics ]]; then rebuild=true; break; fi
    done
fi
if [[ "$rebuild" == true ]]; then
    "${CARGO:-$HOME/.cargo/bin/cargo}" build --release --offline
fi
exec ./target/release/screen-antics "$@"
