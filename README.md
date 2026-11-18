# PolyDB (working title) — Rust 多模型数据库微内核 / IPA 工程文档包

> **工程阶段：V1 研究基线；未经工程验收，不可用于生产数据。**
> 此包依据 IPA《共通フレーム2013》《非機能要求グレード2018》等公开资料组织设计和测试产物；**不是 IPA 官方模板、认证或合规证明**。

## 1. 这份交付包含什么

- **22 份日文工程文档**（`docs/*.md`）：文档管理、要件定义、基本/详细设计、数据/接口、插件/多模型设计、非功能、安全、灾备、UT/IT/ST、WBS、运维、迁移、验收、追踪、ADR/风险、CI、UAT、对抗式审核。
- **质量与测试管理工作簿**（`qa/IPA_品質・試験管理.xlsx`）：要求、测试、非功能、风险、验收门槛、追踪矩阵和公式看板。
- **标准库 Rust 源码**（`src/`）：`Engine` Trait、动态路由、排空式热切换、KV/Document WAL、CRC32、独立进程 Worker、版本化二进制 IPC、manifest、CLI。
- **Turso 0.8.3 实验适配器源码**（`adapters/turso-worker/`）：与核心解耦、单独构建和运行。
- **测试代码**（`tests/`）与 GitHub Actions 工作流及校验脚本。

### 能力边界（非常重要）

| 模块 | V1 状态 | 说明 |
|---|---|---|
| 关系型 | Turso adapter 源码（experimental） | 需单独下载依赖/编译/验证；不作生产保证 |
| KV | 实现代码 | Append-only WAL / 单实例写入 |
| Document | 实现代码 | opaque bytes 存储，**不校验 JSON** |
| Plugin hot-swap | 实现代码 | 只支持相同 model、protocol、storage identity 的路由切换；非在线数据迁移 |
| Process Plugin | 实现代码 | stdio IPC，同步调用；无 deadline/sandbox |
| Manifest | 实现代码 | `key=value` 受信配置，非签名分发 |
| Graph / Vector / .NEMA | 设计规划 | 未实现 |
| 自研事务存储 / MVCC / distributed ACID | 路线图 | 未实现 |
| 备份、加密、权限控制 | 设计规划 | 未实现 |

## 2. 快速开始（Rust stable，Windows/Linux）

在本目录执行：

```sh
cargo build --bins
cargo test --all-targets
cargo run --bin polydb -- demo ./data
cargo run --bin polydb -- put ./data kv app user:1 alice
cargo run --bin polydb -- get ./data kv app user:1
cargo run --bin polydb -- put ./data document app doc:1 '{"message":"hello"}'
cargo run --bin polydb -- scan ./data kv app user:
```

可选基准测试（只输出实际测量结果，不预设目标性能）：

```sh
cargo run --release --bin polydb-bench -- ./data 1000
```

模拟独立进程插件：

```sh
cargo run --bin polydb -- process-demo ./target/debug/polydb-worker ./data
```

Windows PowerShell 把 worker 路径末尾改为 `polydb-worker.exe`。

## 3. 配置插件并运行热替换

先 `cargo build --bins`，将 `target/debug` 添加到 PATH（Windows 使用 `target\debug`），创建 `data` 文件夹，并核对示例：

```
examples/manifests/kv-v1.plugin
examples/manifests/kv-v2.plugin
```

两个 manifest 的 `dataset` 都是 `data/kv.wal`；`polydb plugin-demo` 会在请求排空后新建候选并验证，成功则 generation++，失败保持旧版本：

```sh
cargo run --bin polydb -- plugin-demo examples/manifests/kv-v1.plugin examples/manifests/kv-v2.plugin
```

**警告**：manifest 指定 `executable` 后会以当前用户的 OS 权限启动它。仅加载受信管理员审核过的文件。

## 4. Turso SQL Adapter（独立构建）

本体没有第三方依赖。Turso 使用固定的 `turso = 0.8.3`，需可访问 crates.io 的开发机执行：

```sh
cargo build --manifest-path adapters/turso-worker/Cargo.toml
mkdir -p data
cargo run --bin polydb -- sql-exec ./adapters/turso-worker/target/debug/polydb-turso-worker ./data/demo.db 'CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, name TEXT)'
cargo run --bin polydb -- sql-query ./adapters/turso-worker/target/debug/polydb-turso-worker ./data/demo.db 'SELECT * FROM items'
```

Windows 可执行文件名添加 `.exe`。上述 SQL 是受信本地管理用途，**不支持参数绑定或外部输入拼接**。Turso 源码和 API 参考：
- https://github.com/tursodatabase/turso
- https://docs.rs/crate/turso/0.8.3

## 5. 测试与验证

Linux/macOS：`sh scripts/verify.sh`。Windows PowerShell：`./scripts/verify.ps1`。也可仅 `cargo test --all-targets`。

已包含 UT、IT、ST 的**测试代码**，但交付生成环境没有 Rust 工具链，也无法获取 crates.io 依赖，故**未声称编译/测试通过**；Excel 中全部实测结果初始为 `未実施`。运行后请把真实 CI 日志链接写回试验工作簿。

## 6. 安全与恢复警告

- **不支持**单个 WAL 文件的多进程并发写入；不能代替 SQLite/Turso 的成熟事务能力。
- **不支持**热插拔时把不同物理数据库的内容自动迁移。
- WAL 发生 CRC/末尾截断时**拒绝启动**，不会自动修复。需停止进程并按 `docs/09_障害回復・バックアップ設計.md` 从已验证备份恢复。
- `sync_all` 失败时数据状态可能未决，不可把没有 ACK 简单等同于没有持久化。
- 无网络鉴权、访问控制、加密、插件签名、进程资源限额、SQL 参数绑定、worker deadline；**不得公开服务、存储敏感生产数据**。

## 7. 后续路线

V2：完整插件 ABI / WASM sandbox、Graph/Vector、Catalog、auth、备份、.NEMA adapter、事件订阅。
V3：自研 Rust Page/WAL/B+Tree/LSM/MVCC 与差分测试。
V4：有明确一致性协议的跨引擎协作和混合查询优化。

**交付意图**：先用 Turso 完成关系型功能，同时保证未来把 SQL/storage 插件切换为自研引擎而不重写客户端。当前可直接开始在真实 Rust 开发环境构建和审查 V1。
