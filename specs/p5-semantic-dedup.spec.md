spec: task
name: "P5: 语义去重检测"
tags: [feature, ingest, quality]
estimate: 0.5d
---

## Intent

mempal 当前用 drawer_id（内容哈希）做精确去重，但内容略有不同的记忆会重复存储。
例如两个 session 存了类似的决策记录，用词不同但语义相同。

借鉴 MemPalace 的 check_duplicate（threshold-based 相似度检查），在 ingest 时
检测语义重复，返回 warning 而非拒绝写入——由 agent 决定是否继续。

## Decisions

- `mempal_ingest` 在写入前用向量搜索检查 top-1 相似度
- 相似度阈值：0.85（可通过 config 调整）
- 超过阈值时：仍然写入，但响应中附加 `duplicate_warning` 字段
- duplicate_warning 包含：相似 drawer_id、相似度分数、内容摘要
- CLI `mempal ingest` 超过阈值时打印 warning
- 不做自动合并或拒绝——agent/用户自己决定

## Boundaries

### Allowed
- crates/mempal-mcp/src/server.rs（ingest 添加去重检查）
- crates/mempal-mcp/src/tools.rs（IngestResponse 加 duplicate_warning）
- crates/mempal-core/src/config.rs（可选 dedup_threshold 配置）

### Forbidden
- 不要自动拒绝写入（warning only）
- 不要自动合并重复内容
- 不要在没有向量表时做检查（新数据库第一次写入跳过）

## Out of Scope

- 自动合并策略
- 历史 drawer 的去重清理

## Completion Criteria

Scenario: 检测到语义重复时返回 warning
  Test: test_ingest_duplicate_warning
  Given 数据库已有内容 "decided to use SQLite for single-file portability"
  When ingest "chose SQLite because of single-file backup and portability"
  Then 写入成功
  And 响应包含 duplicate_warning
  And duplicate_warning 包含已有 drawer 的 id 和相似度

Scenario: 不相似内容无 warning
  Test: test_ingest_no_duplicate_warning
  Given 数据库已有内容 "decided to use SQLite"
  When ingest "AAAK compression uses jieba for Chinese"
  Then 写入成功
  And 响应不包含 duplicate_warning

Scenario: 空数据库第一次写入无 warning
  Test: test_ingest_first_drawer_no_check
  Given 空数据库
  When ingest 任意内容
  Then 写入成功
  And 响应不包含 duplicate_warning
