use polydb::{Command, DataModel, Engine, PluginManager, ProcessEngine, PluginManifest, Response, WalEngine};
use std::sync::Arc;
use std::time::Duration;

fn usage() -> &'static str {
    "Usage:\n  polydb put <dir> <kv|document> <ns> <key> <text>\n  polydb get <dir> <kv|document> <ns> <key>\n  polydb delete <dir> <kv|document> <ns> <key>\n  polydb scan <dir> <kv|document> <ns> <prefix>\n  polydb demo <dir>\n  polydb process-demo <worker-executable> <dir>\n  polydb plugin-demo <manifest1> <manifest2>\n  polydb sql-exec <turso-worker-executable> <db-file> <SQL>\n  polydb sql-query <turso-worker-executable> <db-file> <SQL>"
}

fn run() -> polydb::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 { println!("{}", usage()); return Ok(()); }
    if args[1] == "plugin-demo" {
        if args.len() != 4 { return Err(polydb::DbError::Invalid(usage().into())); }
        let first = PluginManifest::from_file(&args[2])?;
        let second = PluginManifest::from_file(&args[3])?;
        let manager = PluginManager::new();
        manager.register("primary", Arc::new(first.spawn()?))?;
        println!("loaded {} generation={}", first.name, manager.generation("primary")?);
        let generation = manager.hot_swap_with("primary", Duration::from_secs(5), || {
            Ok(Arc::new(second.spawn()?) as Arc<dyn Engine>)
        })?;
        println!("switched {} generation={generation}", second.name);
        return Ok(());
    }
    if args[1] == "sql-exec" || args[1] == "sql-query" {
        if args.len() != 5 { return Err(polydb::DbError::Invalid(usage().into())); }
        let engine = ProcessEngine::spawn(&args[2], &[&args[3]], "turso", DataModel::Relational, &args[3])?;
        let command = if args[1] == "sql-exec" { Command::SqlExec { sql: args[4].clone() } }
            else { Command::SqlQuery { sql: args[4].clone() } };
        println!("{:?}", engine.execute(command)?);
        return Ok(());
    }
    if args[1] == "demo" {
        if args.len() != 3 { return Err(polydb::DbError::Invalid(usage().into())); }
        let path = format!("{}/kv.wal", args[2]);
        let manager = PluginManager::new();
        manager.register("kv", Arc::new(WalEngine::open(&path, DataModel::KeyValue)?))?;
        manager.execute("kv", Command::Put {namespace:"demo".into(), key:"hello".into(), value:b"world".to_vec()})?;
        println!("before swap: {:?}", manager.execute("kv", Command::Get {namespace:"demo".into(), key:"hello".into()})?);
        let next = manager.hot_swap_with("kv", Duration::from_secs(5), || {
            Ok(Arc::new(WalEngine::open(&path, DataModel::KeyValue)?) as Arc<dyn Engine>)
        })?;
        println!("after swap generation={next}: {:?}", manager.execute("kv", Command::Get {namespace:"demo".into(), key:"hello".into()})?);
        return Ok(());
    }
    if args[1] == "process-demo" {
        if args.len() != 4 { return Err(polydb::DbError::Invalid(usage().into())); }
        let path = format!("{}/external-kv.wal", args[3]);
        let manager = PluginManager::new();
        manager.register("kv", Arc::new(ProcessEngine::spawn(&args[2], &["kv", &path], "process-kv", DataModel::KeyValue, &path)?))?;
        manager.execute("kv", Command::Put { namespace: "external".into(), key: "hello".into(), value: b"plugin".to_vec() })?;
        println!("{:?}", manager.execute("kv", Command::Get {namespace: "external".into(), key: "hello".into()})?);
        return Ok(());
    }
    if args.len() < 4 { return Err(polydb::DbError::Invalid(usage().into())); }
    let model = DataModel::parse(&args[3]).ok_or_else(|| polydb::DbError::Invalid("unknown model".into()))?;
    if !matches!(model, DataModel::KeyValue | DataModel::Document) {
        return Err(polydb::DbError::Unsupported("local storage supports kv/document only".into()));
    }
    let engine = WalEngine::open(format!("{}/{}.wal", args[2], model.token()), model)?;
    let req = match args[1].as_str() {
        "put" if args.len() == 7 => Command::Put { namespace: args[4].clone(), key: args[5].clone(), value: args[6].as_bytes().to_vec() },
        "get" if args.len() == 6 => Command::Get { namespace: args[4].clone(), key: args[5].clone() },
        "delete" if args.len() == 6 => Command::Delete { namespace: args[4].clone(), key: args[5].clone() },
        "scan" if args.len() == 6 => Command::Scan { namespace: args[4].clone(), prefix: args[5].clone() },
        _ => return Err(polydb::DbError::Invalid(usage().into())),
    };
    match engine.execute(req)? {
        Response::Value(Some(bytes)) => println!("{}", String::from_utf8_lossy(&bytes)),
        Response::Value(None) => println!("NOT_FOUND"),
        other => println!("{other:?}"),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
}
