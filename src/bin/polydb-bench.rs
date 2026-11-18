//! Manual ST-P001 benchmark: measured results only, no throughput promises.
//! Usage: cargo run --release --bin polydb-bench -- <data-dir> <count>
use polydb::{Command, DataModel, Engine, WalEngine};
use std::time::{Duration, Instant};

fn percentile(values: &[Duration], pct: f64) -> f64 {
    if values.is_empty() { return 0.0; }
    let index = ((values.len() - 1) as f64 * pct).round() as usize;
    values[index].as_secs_f64() * 1000.0
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 { eprintln!("Usage: polydb-bench <data-dir> <count>"); std::process::exit(2); }
    let count: usize = args[2].parse()?;
    if count == 0 || count > 100_000 { return Err("count must be 1..100000".into()); }
    let file = format!("{}/bench.wal", args[1]);
    let store = WalEngine::open(&file, DataModel::KeyValue)?;
    let mut put_samples = Vec::with_capacity(count);
    let mut get_samples = Vec::with_capacity(count);
    let overall = Instant::now();
    for i in 0..count {
        let start = Instant::now();
        store.execute(Command::Put {
            namespace: "benchmark".into(), key: format!("key-{i}"), value: vec![42; 128]
        })?;
        put_samples.push(start.elapsed());
    }
    for i in 0..count {
        let start = Instant::now();
        let result = store.execute(Command::Get {
            namespace: "benchmark".into(), key: format!("key-{i}")
        })?;
        if result != polydb::Response::Value(Some(vec![42; 128])) {
            return Err("verification mismatch".into());
        }
        get_samples.push(start.elapsed());
    }
    let total = overall.elapsed().as_secs_f64();
    put_samples.sort();
    get_samples.sort();
    println!("operations={}, elapsed_s={total:.3}, op_per_s={:.1}",2*count, (2*count) as f64/total);
    println!("PUT p50_ms={:.3} p95_ms={:.3} p99_ms={:.3}",
        percentile(&put_samples,0.50),percentile(&put_samples,0.95),percentile(&put_samples,0.99));
    println!("GET p50_ms={:.3} p95_ms={:.3} p99_ms={:.3}",
        percentile(&get_samples,0.50),percentile(&get_samples,0.95),percentile(&get_samples,0.99));
    Ok(())
}
