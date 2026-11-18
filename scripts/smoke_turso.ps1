$ErrorActionPreference = 'Stop'
cargo build --manifest-path adapters/turso-worker/Cargo.toml
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --bin polydb
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
New-Item -ItemType Directory -Path ./data -Force | Out-Null
$w = './adapters/turso-worker/target/debug/polydb-turso-worker.exe'
$db = './data/turso-smoke.db'
& ./target/debug/polydb.exe sql-exec $w $db "CREATE TABLE IF NOT EXISTS smoke (id INTEGER PRIMARY KEY, message TEXT)"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& ./target/debug/polydb.exe sql-exec $w $db "INSERT OR REPLACE INTO smoke (id, message) VALUES (1, 'worker-ok')"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& ./target/debug/polydb.exe sql-query $w $db "SELECT message FROM smoke WHERE id=1"
exit $LASTEXITCODE
