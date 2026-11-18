use std::sync::atomic::{AtomicU64, Ordering};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn wal_path(test: &str) -> std::path::PathBuf {
    let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!("polydb-{test}-{}-{}", std::process::id(), seq));
    std::fs::create_dir_all(&base).unwrap();
    base.join("test.wal")
}
