spec: task
name: "P5: KG timeline 和 stats"
tags: [feature, knowledge-graph]
estimate: 0.5d
---

## Intent

mempal_kg 当前支持 add/query/invalidate，但缺少 timeline 和 stats。
timeline 让 agent 能回答"Kai 这个月做了什么"，stats 让 agent 了解 KG 的规模和健康状况。

借鉴 MemPalace 的 kg_timeline 和 kg_stats，但用更简单的 action 模式实现。

## Decisions

- mempal_kg 新增 action="timeline"：按 valid_from 排序返回某实体的所有三元组
- mempal_kg 新增 action="stats"：返回实体数、三元组数、活跃数、过期数、最常见 predicate
- CLI `mempal kg timeline <entity>` 和 `mempal kg stats`
- timeline 输入：subject（必填），返回该实体作为 subject 或 object 的所有三元组，按时间排序
- stats 不需要输入参数

## Boundaries

### Allowed
- crates/mempal-core/src/db.rs（新查询方法）
- crates/mempal-mcp/src/server.rs（kg 新 action）
- crates/mempal-cli/src/main.rs（kg 新子命令）

### Forbidden
- 不要实现矛盾检测（MemPalace 有但复杂度高，v1 不做）
- 不要做自动三元组抽取

## Out of Scope

- 矛盾检测（contradiction detection）
- 图可视化

## Completion Criteria

Scenario: timeline 返回时间排序的三元组
  Test: test_kg_timeline
  Given 实体 "Kai" 有 3 个三元组（不同 valid_from）
  When 调用 mempal_kg action="timeline" subject="Kai"
  Then 返回 3 个三元组按 valid_from 升序排列
  And 包含 Kai 作为 subject 和 object 的三元组

Scenario: stats 返回正确统计
  Test: test_kg_stats
  Given 数据库有 5 个三元组（3 活跃 + 2 过期）
  When 调用 mempal_kg action="stats"
  Then 返回 total=5, active=3, expired=2
  And 返回实体数和最常见 predicate
