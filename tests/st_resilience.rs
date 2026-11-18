mod common;
use polydb::{Command, DataModel, Engine, PluginManager, Response, WalEngine};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn st_001_torn_wal_tail_fails_closed() {
    let path = common::wal_path("st001");
    {
        let engine = WalEngine::open(&path,DataModel::KeyValue).unwrap();
        engine.execute(Command::Put{namespace:"n".into(),key:"k".into(),value:b"v".to_vec()}).unwrap();
    }
    use std::io::Write;
    std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"PDB1").unwrap();
    assert!(WalEngine::open(&path,DataModel::KeyValue).is_err());
}

#[test]
fn st_002_concurrent_writes_are_replayed() {
    let path = common::wal_path("st002");
    let manager = Arc::new(PluginManager::new());
    manager.register("kv", Arc::new(WalEngine::open(&path,DataModel::KeyValue).unwrap())).unwrap();
    let mut threads = Vec::new();
    for thread in 0..4 {
        let m = Arc::clone(&manager);
        threads.push(std::thread::spawn(move || {
            for index in 0..25 {
                m.execute("kv",Command::Put{namespace:"load".into(),key:format!("{thread}-{index}"),value:b"ok".to_vec()}).unwrap();
            }
        }));
    }
    for thread in threads { thread.join().unwrap(); }
    drop(manager);
    let store = WalEngine::open(&path,DataModel::KeyValue).unwrap();
    match store.execute(Command::Scan {namespace:"load".into(),prefix:String::new()}).unwrap() {
        Response::Items(items) => assert_eq!(items.len(),100),
        _ => panic!("expected items"),
    }
}

#[test]
fn st_003_timeout_preserves_route() {
    struct SlowEngine(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Engine for SlowEngine {
        fn descriptor(&self) -> polydb::Descriptor {
            polydb::Descriptor{name:"slow".into(),model:DataModel::KeyValue,protocol:1,storage_id:"same".into()}
        }
        fn execute(&self,c:Command) -> polydb::Result<Response> {
            if !matches!(c,Command::Ping) {
                self.0.store(true,std::sync::atomic::Ordering::Release);
                std::thread::sleep(Duration::from_millis(300));
            }
            Ok(Response::Ack)
        }
    }
    let manager = Arc::new(PluginManager::new());
    let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    manager.register("slow",Arc::new(SlowEngine(Arc::clone(&started)))).unwrap();
    let m=Arc::clone(&manager);
    let t=std::thread::spawn(move || m.execute("slow",Command::Get{namespace:"a".into(),key:"b".into()}));
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !started.load(std::sync::atomic::Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline, "slow operation did not start");
        std::thread::yield_now();
    }
    let result=manager.hot_swap_with("slow",Duration::from_millis(20),||Ok(Arc::new(SlowEngine(Arc::clone(&started)))));
    assert!(result.is_err());
    t.join().unwrap().unwrap();
    assert_eq!(manager.generation("slow").unwrap(),1);
    assert_eq!(manager.execute("slow",Command::Ping).unwrap(),Response::Ack);
}
