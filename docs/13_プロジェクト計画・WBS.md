# 13 プロジェクト計画・WBS

文書ID: PMP-001｜版: 0.1.0｜状態: 提案

## 1. フェーズと退出条件
| フェーズ | 作業 | 出口ゲート |
|---|---|---|
| P0 | 要件確定、IPA 参考範囲、セキュリティ境界 | REQ/NFR 合意、リスク責任決定 |
| P1/V1 | trait、registry、WAL、worker、IPC、manifest、CLI | UT/IT/ST の Must pass + Windows 検証 |
| P2 | Graph/Vector/JSON schema/.NEMA、Catalog、認証、backup、観測性 | 負荷/セキュリティ/移行評価 |
| P3 | 独自 B+Tree/LSM、WAL 改修、MVCC、SQL parser/optimizer | 故障注入、差分・性能・互換性試験 |
| P4 | 複数エンジン整合性、クラスタリング任意 | 正常/故障時の整合性証明 |

## 2. RACI（暫定）
| 役割 | 要件 | 設計 | 実装 | UT/IT/ST | 出荷 |
|---|---|---|---|---|---|
| Product Owner | A | C | I | A | A |
| Architect | R | A | C | C | C |
| Engineer | C | R | R | R | C |
| QA | C | C | C | A/R | R |
| Security reviewer | C | C | C | R | C |

A=最終説明責任、R=実行責任、C=相談、I=通知。少人数の場合でも責任ロールを分けてレビュー記録を残す。

## 3. 変更優先度
P0: 失敗時データ破損回避。P1: 主要機能。P2: 体験/性能。P3: 高度最適化。新機能は要件定義→基本→詳細→UT/IT/ST 差分承認後に取り込む。
