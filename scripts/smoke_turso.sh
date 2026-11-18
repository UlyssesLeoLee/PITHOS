#!/usr/bin/env sh
# Manual IT-007/ST-005. Run from repository root. Uses disposable test data only.
set -eu
cargo build --manifest-path adapters/turso-worker/Cargo.toml
cargo build --bin polydb
mkdir -p data
WORKER=./adapters/turso-worker/target/debug/polydb-turso-worker
DB=./data/turso-smoke.db
./target/debug/polydb sql-exec "$WORKER" "$DB" "CREATE TABLE IF NOT EXISTS smoke (id INTEGER PRIMARY KEY, message TEXT)"
./target/debug/polydb sql-exec "$WORKER" "$DB" "INSERT OR REPLACE INTO smoke (id, message) VALUES (1, 'worker-ok')"
./target/debug/polydb sql-query "$WORKER" "$DB" "SELECT message FROM smoke WHERE id=1"
echo "Verify manually: the SQL result includes worker-ok; record evidence in QA workbook."
