mod common;
use polydb::{Command, DataModel, Engine, Response, WalEngine};

#[test]
fn ut_001_put_get_delete_and_restart() {
    let path = common::wal_path("ut001");
    {
        let store = WalEngine::open(&path, DataModel::KeyValue).unwrap();
        assert_eq!(store.execute(Command::Get {namespace:"n".into(), key:"k".into()}).unwrap(), Response::Value(None));
        store.execute(Command::Put {namespace:"n".into(),key:"k".into(), value:b"hello".to_vec()}).unwrap();
        assert_eq!(store.execute(Command::Get {namespace:"n".into(),key:"k".into()}).unwrap(),Response::Value(Some(b"hello".to_vec())));
    }
    let restored = WalEngine::open(&path, DataModel::KeyValue).unwrap();
    assert_eq!(restored.execute(Command::Get {namespace:"n".into(),key:"k".into()}).unwrap(),Response::Value(Some(b"hello".to_vec())));
    restored.execute(Command::Delete {namespace:"n".into(),key:"k".into()}).unwrap();
    drop(restored);
    let restored = WalEngine::open(&path, DataModel::KeyValue).unwrap();
    assert_eq!(restored.execute(Command::Get {namespace:"n".into(),key:"k".into()}).unwrap(),Response::Value(None));
}

#[test]
fn ut_002_namespaces_and_prefix_scans() {
    let path = common::wal_path("ut002");
    let store = WalEngine::open(path, DataModel::Document).unwrap();
    for (namespace,key) in [("a","alpha"),("a","alpine"),("a","zebra"),("b","alpha")] {
        store.execute(Command::Put{namespace:namespace.into(),key:key.into(),value:b"{}".to_vec()}).unwrap();
    }
    match store.execute(Command::Scan {namespace:"a".into(),prefix:"al".into()}).unwrap() {
        Response::Items(items) => assert_eq!(items.len(),2),
        _=>panic!("wrong reply"),
    }
}

#[test]
fn ut_003_reject_empty_namespace_and_oversize() {
    let path = common::wal_path("ut003");
    let store = WalEngine::open(path, DataModel::KeyValue).unwrap();
    assert!(store.execute(Command::Put {namespace:"".into(),key:"k".into(),value:vec![]}).is_err());
    assert!(store.execute(Command::Put {namespace:"a".into(),key:"k".into(),value:vec![0;4*1024*1024]}).is_err());
}
