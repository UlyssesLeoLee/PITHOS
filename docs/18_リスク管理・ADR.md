# 18 リスク管理・アーキテクチャ決定記録

文書ID: ADR-001｜版: 0.1.0｜状態: 未承認

## 1. ADR
| ADR | 決定案 | 根拠 | 不採用案 |
|---|---|---|---|
| ADR-001 | Rust stable + std-only core | 長期可搬性・独立構築 | すべてを巨大依存へ |
| ADR-002 | Turso は独立 process adapter | 更新影響・障害隔離 | SQL Engine を core 固定 |
| ADR-003 | 同一 dataset の routing-only hot swap | data migration と区別 | 任意 path の無検証切替 |
| ADR-004 | fail-closed WAL replay | サイレント破損防止 | 自動欠損切除 |
| ADR-005 | Graph/Vector/Cloud を後続 phase | Correctness first | 初版一括実装 |
| ADR-006 | 文書管理は IPA 共通フレームを参考 | 日本の工程分類とレビュー統一 | 「IPA 認証済」を無根拠に主張 |

## 2. リスク登録簿
| ID | リスク | 重大度 | 検知/対策 | Owner | 状態 |
|---|---|---|---|---|---|
| RSK-001 | 同一 WAL 複数プロセス書込破損 | Critical | OS lock/単一 writer enforcement | Architect | Open |
| RSK-002 | SQL injection / raw SQL | Critical | prepared binding / RBAC | Security | Open |
| RSK-003 | panic 時 swap admission 閉塞 | High | catch_unwind + IT-005（abort は対象外） | Engineer | Mitigated / untested |
| RSK-004 | Worker ハングで処理停止 | High | deadline, cancellation, supervisor | Engineer | Open |
| RSK-005 | 自動バックアップなし | High | snapshot + recovery test | Ops | Open |
| RSK-006 | Turso API version 変化 | Medium | version pin + compatibility CI | Engineer | Open |
| RSK-007 | 大きい WAL で再起動遅延 | High | compaction / LSM | Engineer | Open |
| RSK-008 | 失敗時 ACK 消失の不確定書込 | High | idempotency token / dedup | Architect | Open |
| RSK-009 | 未認証 plugin executable | Critical | signature, ACL, sandbox | Security | Open |
| RSK-010 | 用語上の「IPA 準拠」誤解 | Medium | documentation disclaimer | PM | Mitigated |

## 3. 主要なトレードオフ
V1 は各書込みで `sync_all` を行うため耐久化を優先し throughput は低くなり得る。CRC32 は誤り検出であって攻撃者に対する真正性保証ではない。in-memory index は読込高速だがサイズに比例した RAM と restart の再生負荷がある。すべての制限を実測/受入で再判定する。
