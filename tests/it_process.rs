mod common;
use polydb::{Command, DataModel, Engine, ProcessEngine, Response};

#[test]
fn it_004_process_plugin_roundtrip() {
    let executable = env!("CARGO_BIN_EXE_polydb-worker");
    let path = common::wal_path("it004");
    let path_string = path.to_string_lossy().to_string();
    let engine = ProcessEngine::spawn(executable, &["kv", &path_string], "worker-kv", DataModel::KeyValue, &path_string).unwrap();
    engine.execute(Command::Put {namespace:"n".into(),key:"x".into(),value:vec![0,10,13,255]}).unwrap();
    assert_eq!(engine.execute(Command::Get {namespace:"n".into(),key:"x".into()}).unwrap(),Response::Value(Some(vec![0,10,13,255])));
}
