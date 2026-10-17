#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
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
