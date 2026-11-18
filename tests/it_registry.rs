mod common;
use polydb::{Command, DataModel, DbError, Engine, PluginManager, Response, WalEngine};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn it_001_swap_preserves_dataset_and_increments_generation() {
    let path = common::wal_path("it001");
    let manager = PluginManager::new();
    manager.register("data",Arc::new(WalEngine::open(&path,DataModel::KeyValue).unwrap())).unwrap();
    manager.execute("data",Command::Put{namespace:"n".into(),key:"k".into(),value:b"one".to_vec()}).unwrap();
    let generation = manager.hot_swap_with("data",Duration::from_secs(2),|| {
        Ok(Arc::new(WalEngine::open(&path,DataModel::KeyValue)?) as Arc<dyn Engine>)
    }).unwrap();
    assert_eq!(generation,2);
    assert_eq!(manager.execute("data",Command::Get{namespace:"n".into(),key:"k".into()}).unwrap(),Response::Value(Some(b"one".to_vec())));
}

#[test]
fn it_002_failed_swap_retains_old_engine() {
    let path = common::wal_path("it002");
    let different = common::wal_path("it002-other");
    let manager = PluginManager::new();
    manager.register("data",Arc::new(WalEngine::open(&path,DataModel::KeyValue).unwrap())).unwrap();
    let rejected = manager.hot_swap_with("data",Duration::from_secs(2),|| {
        Ok(Arc::new(WalEngine::open(&different,DataModel::KeyValue)?) as Arc<dyn Engine>)
    });
    assert!(matches!(rejected,Err(DbError::Incompatible(_))));
    assert_eq!(manager.generation("data").unwrap(),1);
    assert_eq!(manager.execute("data",Command::Ping).unwrap(),Response::Ack);
}

#[test]
fn it_003_duplicate_route_is_rejected() {
    let path = common::wal_path("it003");
    let manager = PluginManager::new();
    manager.register("a",Arc::new(WalEngine::open(&path,DataModel::KeyValue).unwrap())).unwrap();
    assert!(manager.register("a",Arc::new(WalEngine::open(&path,DataModel::KeyValue).unwrap())).is_err());
}

#[test]
fn it_005_loader_panic_does_not_block_route() {
    let path = common::wal_path("it005");
    let manager = PluginManager::new();
    manager.register("data",Arc::new(WalEngine::open(&path,DataModel::KeyValue).unwrap())).unwrap();
    let result = manager.hot_swap_with("data",Duration::from_secs(1), || -> polydb::Result<Arc<dyn Engine>> {
        panic!("malicious candidate loader")
    });
    assert!(result.is_err());
    assert_eq!(manager.generation("data").unwrap(),1);
    assert_eq!(manager.execute("data",Command::Ping).unwrap(),Response::Ack);
}
