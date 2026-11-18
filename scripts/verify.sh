#!/usr/bin/env sh
set -eu
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
printf '\nPASSED: core CI gates. Turso adapter is a separate build.\n'
