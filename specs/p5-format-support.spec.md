spec: task
name: "P5: Slack + Codex 格式支持"
tags: [feature, ingest, formats]
estimate: 1d
---

## Intent

扩展 mempal-ingest 支持更多对话格式。当前支持 3 种（Claude JSONL、ChatGPT JSON、
纯文本），MemPalace 支持 6 种。最有价值的两个缺失格式是 Slack JSON export 和
Codex CLI JSONL。

## Decisions

- Slack JSON export：解析 `export/channel/messages.json`，提取 user + text + ts
- Codex CLI JSONL：解析 event_msg 类型记录，跳过 synthetic context
- 格式检测：现有 `detect_format()` 扩展，通过文件结构自动识别
- 归一化输出：统一为 `> User: ...\nAssistant: ...` 的标准对话格式
- 分块策略：按 QA 对分块（与现有策略一致）

## Boundaries

### Allowed
- crates/mempal-ingest/src/（新增格式检测 + 归一化模块）
- crates/mempal-ingest/tests/（格式测试）

### Forbidden
- 不要改变现有 3 种格式的行为
- 不要依赖外部 NLP 库做格式检测

## Out of Scope

- Slack channel 消息（只支持 DM）
- Discord / Telegram 格式
- 实时 Slack API 集成

## Completion Criteria

Scenario: Slack DM JSON 格式识别和导入
  Test: test_ingest_slack_dm
  Given Slack export 目录包含 messages.json
  When mempal ingest --wing team --format slack
  Then 正确提取 user 和 text
  And 按对话对分块

Scenario: Codex CLI JSONL 格式识别和导入
  Test: test_ingest_codex_jsonl
  Given Codex CLI 的 JSONL 日志文件
  When mempal ingest --wing project
  Then 只提取 event_msg 类型记录
  And 跳过 synthetic context 记录

Scenario: 格式自动检测
  Test: test_format_auto_detect
  Given 不指定 --format 参数
  When 传入 Slack export 目录
  Then 自动识别为 slack 格式
