# Example runbook — no production data

1. `cargo build --bins`
2. `cargo run --bin polydb -- demo ./data`
3. `cargo run --bin polydb -- put ./data document sample doc-001 '{"id":"doc-001","tags":["rust","db"]}'`
4. `cargo run --bin polydb -- get ./data document sample doc-001`
5. In separate clean test folder, create deliberate corruption by appending a partial record to a copied WAL. `polydb get` must fail with `Corrupt` (never test this on valuable data).
6. For Turso, compile `adapters/turso-worker`, run `sql-exec`, then `sql-query`.
7. Record actual results under `qa/IPA_品質・試験管理.xlsx`.

This is not an SDK load test, not a crash-consistency certification, and not an official IPA audit.
