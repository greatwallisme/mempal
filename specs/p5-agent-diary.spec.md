spec: task
name: "P5: Agent Diary Convention"
tags: [feature, protocol, convention]
estimate: 0.5d
---

## Intent

让 agent 能记录跨 session 的行为模式、学到的教训、和自我观察。这和决策记忆不同：
决策记忆记录项目事实，日记记录 agent 自己的行为。

MemPalace 用专门的 diary_write/diary_read 工具实现。mempal 用 convention 实现：
通过 taxonomy + MEMORY_PROTOCOL 新规则，不加新工具。

## Decisions

- 不加新 MCP 工具——用现有 mempal_ingest + 固定 wing/room convention
- 日记 convention：`wing = "agent-diary"`, `room = agent 名字`（如 "claude", "codex"）
- MEMORY_PROTOCOL 新增 Rule 5a: KEEP A DIARY
- 日记内容格式：`OBSERVATION:`, `LESSON:`, `PATTERN:` 前缀标记类型
- 默认 taxonomy 条目在 `mempal init` 时不创建（agent 首次写日记时自动路由）
- `mempal search --wing agent-diary` 可查看所有 agent 的日记

## Boundaries

### Allowed
- crates/mempal-core/src/protocol.rs（新增 Rule 5a）
- docs/usage.md（文档化 diary convention）

### Forbidden
- 不要加新 MCP 工具
- 不要改 ingest 逻辑
- 不要强制 agent 写日记——只是提供 convention

## Out of Scope

- 日记自动摘要
- 跨 agent 日记合并

## Completion Criteria

Scenario: 协议包含 diary 规则
  Test: test_protocol_contains_diary_rule
  Given mempal_status 返回的 memory_protocol
  When 检查协议文本
  Then 包含 "KEEP A DIARY" 或 "DIARY" 相关规则
  And 包含 wing="agent-diary" 的使用说明

Scenario: agent 能通过现有工具写日记
  Test: test_diary_via_ingest
  Given agent 调用 mempal_ingest wing="agent-diary" room="claude"
  When 内容为 "LESSON: always check repo docs before writing infrastructure"
  Then drawer 成功创建在 agent-diary/claude scope 下

Scenario: 日记可搜索
  Test: test_diary_searchable
  Given agent-diary wing 有日记内容
  When 搜索 "lesson infrastructure" --wing agent-diary
  Then 返回匹配的日记 drawer
