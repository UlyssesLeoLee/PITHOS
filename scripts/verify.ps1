$ErrorActionPreference = 'Stop'
cargo fmt --all
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test --all-targets
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --release
exit $LASTEXITCODE
