use polydb::PluginManifest;
#[test]
fn ut_004_invalid_manifest_is_rejected() {
    let path = std::env::temp_dir().join(format!("polydb-manifest-{}.txt",std::process::id()));
    std::fs::write(&path, "model=kv\nmodel=document\n").unwrap();
    assert!(PluginManifest::from_file(&path).is_err());
    let _ = std::fs::remove_file(&path);
}
