# 14 運用設計・Runbook

文書ID: OPR-001｜版: 0.1.0｜状態: 提案

## 1. 事前条件
Rust stable + cargo、ローカルファイル書込み権限。V1 はネットワークサービスではなく CLI 組込ライブラリ。データディレクトリに秘密情報を直接置かない。1 WAL file に同時アクセスするアプリプロセスは 1 個に制限する。

## 2. Windows / Linux 基本操作
```
cargo build --bins
cargo test --all-targets
cargo run --bin polydb -- demo ./data
cargo run --bin polydb -- put ./data kv app key001 value001
cargo run --bin polydb -- get ./data kv app key001
cargo run --bin polydb -- put ./data document app doc001 '{"name":"Alice"}'
```
Windows PowerShell では `./target/debug/polydb-worker` を `./target/debug/polydb-worker.exe` とする。

## 3. 起動停止・監視
V1 Worker は stdio プロセスとして親が起動する。親切断で Worker は EOF 退出する。監視・自動再起動・PID 管理は OS supervisor の範囲外。error stderr、exit code と出力を保存して原因分析する。

## 4. 操作ガード
切替前にオペレータが旧 plugin と候補 manifest の `protocol/model/dataset` を確認し、想定外の外部プロセス書込みがないことを確かめる。切替失敗後は generation 不変を検証。破損時は復旧前にファイルコピーを作成する。

## 5. トラブルシュート
`Corrupt`: 再実行で書込みを続けず、破損ファイルを保全。`Busy`: 排空を妨げた処理を分析。`Plugin`: 子プロセスの stderr と binary path を確認。`Incompatible`: model/protocol/storage identity の一致を確認。
