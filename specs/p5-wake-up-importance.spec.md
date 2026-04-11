spec: task
name: "P5: L1 重要性排序的 wake-up"
tags: [feature, wake-up, layers]
estimate: 1d
---

## Intent

当 agent 启动新 session 时，`mempal wake-up` 应该返回**最重要的**记忆，而不是最近的。
当前 wake-up 按时间倒序取最近 5 个 drawer，但最近的可能是琐碎的 status snapshot，
而关键的架构决策可能在几天前。

借鉴 MemPalace 的 L0-L1 分层思想，但用更简单的方式实现：给 drawer 加 importance 评分，
wake-up 按 importance 排序。不做完整的 4 层系统——L2/L3 已被 `mempal search` 覆盖。

## Decisions

- drawers 表加 `importance INTEGER DEFAULT 0` 列（schema v4 migration）
- importance 范围 0-5（对应 AAAK 的 ★ 到 ★★★★★）
- `mempal_ingest` 新增可选 `importance` 字段，默认 0
- `mempal wake-up` 改为按 `importance DESC, added_at DESC` 排序
- CLI `mempal wake-up --top-k N` 控制返回数量（默认 5）
- MCP `mempal_status` 的 scopes 部分不变
- identity.txt 整合到 wake-up 的 L0 输出（已有读取逻辑，确保显示）

## Boundaries

### Allowed
- crates/mempal-core/src/db.rs（migration + 排序改动）
- crates/mempal-core/src/types.rs（Drawer 加 importance 字段）
- crates/mempal-cli/src/main.rs（wake-up 排序 + --top-k）
- crates/mempal-mcp/src/server.rs（ingest 支持 importance）
- crates/mempal-mcp/src/tools.rs（IngestRequest 加 importance）

### Forbidden
- 不要实现完整的 4 层系统（L2/L3 已被 search 覆盖）
- 不要用 LLM 自动评估 importance（由 agent 或用户显式指定）

## Out of Scope

- L2/L3 层实现（已有 search 覆盖）
- 自动 importance 推断
- wake-up 的 AAAK 格式改动（已有 --format aaak）

## Completion Criteria

Scenario: wake-up 按重要性排序
  Test: test_wake_up_importance_order
  Given 数据库有 importance=5 的旧 drawer 和 importance=0 的新 drawer
  When 执行 mempal wake-up
  Then importance=5 的 drawer 排在前面
  And importance=0 的新 drawer 排在后面

Scenario: ingest 可指定 importance
  Test: test_ingest_with_importance
  Given 通过 MCP mempal_ingest 提交 importance=4 的内容
  When 查询该 drawer
  Then importance 字段为 4

Scenario: 默认 importance 为 0
  Test: test_default_importance
  Given 通过 mempal_ingest 提交内容不指定 importance
  When 查询该 drawer
  Then importance 字段为 0

Scenario: schema 迁移不丢数据
  Test: test_schema_v4_migration
  Given 一个 v3 的数据库有已有 drawer
  When Database::open 执行迁移
  Then 现有 drawer 的 importance 为 0
  And schema_version 为 4
