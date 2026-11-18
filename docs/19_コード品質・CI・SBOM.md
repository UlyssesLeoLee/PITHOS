# 19 コード品質・CI・依存管理

文書ID: QAP-001｜版: 0.1.0｜状態: CI 定義同梱・結果未確認

## 1. CI 定義
`.github/workflows/ci.yml`：stable toolchain、rustfmt、Clippy、UT/IT/ST、release build。Turso worker は独立 job で `continue-on-error` の研究枠。開発の品質 gate は core pass、Turso pass は別に記録する。

## 2. 品質ポリシー
- データ完全性を壊す変更はテスト不足なら merge 禁止。
- `unsafe` は原則不使用。将来 FFI 導入時に unsafe 監査。
- WAL format version、IPC protocol version、dataset identity の変更は設計と互換性表を必ず改定。
- fuzz/property testing、Miri、cargo-audit、cargo-deny、SBOM 自動生成は V2 gate への追加予定。

## 3. OSS 依存
core: Rust 標準ライブラリのみ。Turso worker: `turso = 0.8.3`, `tokio = 1`。間接依存/ライセンスの正確な SBOM はネットワーク構築で lock 生成後に取得する。Turso OSS 本体の詳細なライセンス・feature は導入時点で再確認。

## 4. 構成管理
ソース、文書、試験結果、dependency lock、hash、変更要求は同一 commit に紐付ける。`Cargo.lock` は実際に解決してからコミットする。未実行環境でビルド成功の事実を記載しない。
