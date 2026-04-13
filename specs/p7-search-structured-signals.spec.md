spec: task
name: "P7: mempal_search structured signals — explicit DTO fields for AAAK-extracted metadata"
tags: [feature, search, aaak, mcp, signals]
estimate: 0.7d
---

## Intent

在不改 `SearchResultDto.content` 的 raw 语义的前提下，给 `mempal_search` 响应里每条结果无条件附加 5 个显式结构化字段（`entities` / `topics` / `flags` / `emotions` / `importance_stars`），通过新增公共分析 API `crate::aaak::signals::analyze` 在 response 构造阶段提取并填充。

这是 P7 Proposal C 被实证否证后的路线修正。empirical 测量证明 `AaakCodec::encode` 不是 byte-level 压缩器（267B→381B, +42%），整个"compression"前提崩塌。修正方案：放弃 `format="aaak"` 参数轴，把 AAAK codec 内部已有的结构化信号提取器（`extract_entities`、`extract_topics`、`detect_flags`、`detect_emotions`、`infer_weight`）通过一个新的公共 analysis API 暴露，让 `mempal_search` handler 在构造 `SearchResultDto` 时直接填入结构化字段。Agent 由此可以 `result.flags.contains("DECISION")` 过滤、按 `importance_stars` 排序，无需解析任何 AAAK 语法。

设计文档：`docs/specs/2026-04-13-p7-search-structured-signals.md`。关联记忆层：`drawer_mempal_default_9f8326c6`（narrow superseding of `drawer_mempal_default_3b83fde9`），`drawer_mempal_default_e7fcd33f`（role-level framing 保留）。

## Decisions

- 暴露新的公共函数 `crate::aaak::signals::analyze(text: &str) -> AaakSignals` 作为 MCP 层的唯一入口；MCP 不直接 import codec 私有 helper
- `AaakSignals` struct 含 5 个字段：`entities: Vec<String>`、`topics: Vec<String>`、`flags: Vec<String>`、`emotions: Vec<String>`、`importance_stars: u8`
- **`importance_stars` 取值范围 2-4**：直接来自 `infer_weight(&flags)`。`infer_weight` 返回 `2` 当 flags 不含 DECISION/PIVOT/TECHNICAL（else 分支，`src/aaak/codec.rs:444-446`）、`3` 含 TECHNICAL、`4` 含 DECISION/PIVOT。**不做 `.max(1)` 后处理**——min 已经是 2
- **`flags` 字段永远 >= 1 entry**（sentinel 语义）：现有 `detect_flags` 在 `src/aaak/codec.rs:412-414` 对"无匹配"显式 fall back 到 `["CORE"]`。Agents 用 `flags.contains("DECISION")` 判断真决策，用 `flags == ["CORE"]` 判断"未分类"。P7 保留这个行为，**不改 extractor**
- **`emotions` 字段永远 >= 1 entry**：`detect_emotions` 在 `src/aaak/codec.rs:429-431` fall back 到 `["determ"]`（`DEFAULT_EMOTION` 常量）。同 flags，保留
- **`entities` 字段永远 >= 1 entry**：`analyze` 遍历 `extract_entities` 原始输出，每条通过 `default_entity_code` 映射为 3-letter code（如 `"Decision"` → `"DEC"`）；去重后若为空则 push `"UNK"`（`DEFAULT_ENTITY_CODE`）。这个转换步骤现在存在于 `encode()` 内部（`src/aaak/codec.rs:213-231`），`analyze` 复刻这个 stateless 版本
- **`topics` 字段可以为空 vec**：`extract_topics` 自己不加 default（topic 的 `"note"` default 是 `encode()` 自己加的，在 `src/aaak/codec.rs:238-242`）。signals 不做这个 wrapping，保持 raw
- `analyze` 是 infallible API（内部 extractors 对 degenerate input 已有兜底），签名不返回 `Result`
- **`src/aaak/codec.rs` 里 7 处 `fn` 升为 `pub(crate) fn`**（0 逻辑变更）：`normalize_whitespace` (line 337)、`extract_entities` (line 346)、`extract_topics` (line 370)、`detect_flags` (line 403)、`detect_emotions` (line 419)、`infer_weight` (line 436)、`default_entity_code` (line 480)
- `SearchResultDto` 扩展 5 个新字段：`entities`、`topics`、`flags`、`emotions`、`importance_stars`；**不使用 `skip_serializing_if`**——字段始终 on-wire（响应 byte size 可预测，客户端可以依赖字段存在）；仅在反序列化时用 `#[serde(default)]` 保证旧 client 兼容
- `SearchResultDto.content` 字段语义**不变**，始终是 raw drawer text，字节级保全
- `mempal_search` 无条件为每条结果调 `analyze(&result.content)` 并填充新字段，不加任何 `format` / `include_signals` 参数
- 不加 `mempal_peek_partner` 的 signal 支持（out of scope）
- 不删除 `AaakCodec` / 不 deprecate `mempal compress` / 不 deprecate `mempal wake-up --format aaak`
- `drawers` / `drawer_vectors` / `triples` 表 schema 保持 v4 不变；`analyze` 不写入任何持久化字段
- 删除 `impl From<SearchResult> for SearchResultDto` 的既有 `From` impl，handler 里显式构造 DTO

## Boundaries

### Allowed
- src/aaak/signals.rs（新增）
- src/aaak/mod.rs（添加 `pub mod signals` + re-exports）
- src/aaak/codec.rs（仅 visibility 变更，不动逻辑）
- src/mcp/tools.rs（`SearchResultDto` 加字段 + 文档注释）
- src/mcp/server.rs（`mempal_search` handler 构造 DTO + 删除旧 From impl）
- tests/search_structured_signals.rs（新增集成测试文件）

### Forbidden
- 不要修改 `drawers`、`drawer_vectors`、`triples` 等任何表 schema
- 不要 bump `CURRENT_SCHEMA_VERSION`
- 不要修改 `SearchResultDto.content` 的语义
- 不要引入 `format` / `include_signals` / 任何新请求参数到 `mempal_search`
- 不要在 `mempal_search` 路径里调用 `AaakCodec::encode`
- 不要改 `extract_entities` / `extract_topics` / `detect_flags` / `detect_emotions` / `infer_weight` 这 5 个 extractor 的算法
- 不要给 `mempal_peek_partner` / `mempal_ingest` / `mempal_kg` / 任何其他 MCP 工具加 signals 字段
- 不要修改 `mempal compress` / `mempal wake-up --format aaak` CLI 子命令
- 不要引入新的运行时依赖
- 不要破坏现有 7 个 MCP 工具的既有 schema 或行为

## Out of Scope

- `format="aaak"` 参数（整个 format 轴从 P7 移除）
- 重写 `AaakCodec` 做真正的 byte-level 压缩（独立的 research-level 工作）
- 把 `signals` 扩展到 `mempal_peek_partner`
- 把 `signals` 持久化到 `drawers` 表
- LLM-based entity / flag extractor
- longmemeval benchmark 集成
- CLI `mempal search` 输出结构化字段
- Agent 消费策略写进 MEMORY_PROTOCOL
- `mempal_ingest` dry_run 预览 signals

## Completion Criteria

Scenario: search response 携带 5 个新的 signal 字段
  Test:
    Filter: test_search_response_includes_structured_signals
    Level: integration
    Targets: src/mcp/server.rs, src/mcp/tools.rs, src/aaak/signals.rs
  Given 数据库中一条 drawer，content 为 "Decision: use Arc<Mutex<>> for state"
  When 以 `mempal_search` query "state" 调用
  Then 返回的 `SearchResultDto` 里 `entities` 字段是非空 `Vec<String>`
  And `topics` 字段是非空 `Vec<String>`
  And `flags` 字段是非空 `Vec<String>`
  And `emotions` 字段至少含 "1" 个 entry（永不为空，`detect_emotions` 对无匹配时 fallback 到 `["determ"]`）
  And `importance_stars` 字段是 `u8`，值 >= 2（最小值由 `infer_weight` else 分支决定）

Scenario: DECISION 关键字的 content 产出 DECISION flag
  Test:
    Filter: test_decision_keyword_yields_decision_flag
    Level: integration
    Targets: src/aaak/signals.rs, src/aaak/codec.rs
  Given 数据库中一条 drawer，content 明确含 "Decision: chose X over Y because Z"
  When 以 `mempal_search` 查询该 drawer
  Then 返回结果的 `flags` 字段包含 "DECISION"
  And 该结果的 `importance_stars` 值 >= 2

Scenario: importance_stars 在未检测到任何 category 关键字时为 2
  Test:
    Filter: test_importance_stars_defaults_to_2_for_uncategorized_text
    Level: unit
    Targets: src/aaak/signals.rs
  Given 一段不含 DECISION / PIVOT / TECHNICAL 等 flag 关键字的普通文本 "weather update today"
  When 调用 `aaak::signals::analyze(text)`
  Then 返回的 `AaakSignals.importance_stars` 值 == 2
  And `flags` 字段包含 "CORE" 作为 fallback sentinel
  And `flags` 字段不含 "DECISION"

Scenario: 纯 CJK content 通过 jieba 产出 CJK-only entities
  Test:
    Filter: test_pure_cjk_content_yields_jieba_derived_entities
    Level: integration
    Targets: src/aaak/signals.rs, src/aaak/codec.rs
  Given 数据库中一条 drawer，content 为纯中文 "系统决策：采用共享内存同步机制解决状态漂移问题"（不含任何 ASCII 单词，确保 ASCII 路径无法命中）
  When 以 `mempal_search` 查询该 drawer 并读出 `SearchResultDto.entities`
  Then `entities` 字段长度 >= "1"
  And `entities` 不等于 `["UNK"]`（断言 CJK jieba 分支产出了至少一个非 UNK 的 entity；因为 content 无 ASCII，结果里出现非 UNK code 等价于 jieba CJK 路径命中）

Scenario: 空 content 产出全部 sentinel 默认值
  Test:
    Filter: test_empty_content_yields_sentinel_defaults
    Level: unit
    Targets: src/aaak/signals.rs
  Given 空字符串 content ""
  When 调用 `aaak::signals::analyze("")`
  Then `AaakSignals.entities` 恰好为 `["UNK"]`（`DEFAULT_ENTITY_CODE` fallback）
  And `AaakSignals.flags` 恰好为 `["CORE"]`（`detect_flags` line 412-414 fallback）
  And `AaakSignals.emotions` 恰好为 `["determ"]`（`DEFAULT_EMOTION` fallback）
  And `AaakSignals.topics` 长度为 "0"（extract_topics 不加 default）
  And `AaakSignals.importance_stars` 值 == 2（`infer_weight(&["CORE"])` else 分支）

Scenario: whitespace-only content 产出与空 content 相同的 sentinel defaults
  Test:
    Filter: test_whitespace_content_matches_empty_sentinel_behavior
    Level: unit
    Targets: src/aaak/signals.rs
  Given whitespace-only 字符串 "   \t\n  "
  When 调用 `aaak::signals::analyze` 于该字符串
  Then 返回的 `AaakSignals.entities` 恰好为 `["UNK"]`
  And `flags` 恰好为 `["CORE"]`
  And `emotions` 恰好为 `["determ"]`
  And `topics` 长度为 "0"
  And `importance_stars` 值 == 2

Scenario: content 字段在 P7 前后字节级不变
  Test:
    Filter: test_content_field_byte_identical_to_raw
    Level: integration
    Test Double: fixture_drawer
    Targets: src/mcp/server.rs, src/mcp/tools.rs
  Given 数据库中插入一条 drawer，content 为已知原文字符串 "Decision: use Arc<Mutex<>>"
  When 以 `mempal_search` 查询该 drawer 并读出返回的 `SearchResultDto`
  Then 返回的 `content` 字段与原文完全相等（byte-level）
  And `content` 字段不以 "V1|" 开头（非 AAAK 格式）
  And `content` 字段不含 "★" 字符

Scenario: drawer_id 和 source_file 在加 signals 前后不变
  Test:
    Filter: test_drawer_id_and_source_file_unchanged_by_signals
    Level: integration
    Targets: src/mcp/server.rs
  Given 数据库中两条 drawer，各有已知的 drawer_id 和 source_file
  When 以 `mempal_search` 查询它们并读出返回
  Then 每条结果的 `drawer_id` 字段字节级等于原始 drawer_id
  And 每条结果的 `source_file` 字段字节级等于原始 source_file

Scenario: 加 signals 不会导致结果数量变化
  Test:
    Filter: test_result_count_unchanged_after_signals_wiring
    Level: integration
    Targets: src/mcp/server.rs
  Given 数据库中插入 "5" 条 drawer
  When 以 `mempal_search(top_k=5)` 查询
  Then 返回的 `results` 长度恰好为 "5"
  And 没有 drawer 因为 `analyze` 调用被丢弃

Scenario: search 对 palace.db 无副作用
  Test:
    Filter: test_search_with_signals_has_no_db_side_effects
    Level: integration
    Test Double: tempfile_palace_db
    Targets: src/mcp/server.rs, src/aaak/signals.rs
  Given tempfile palace.db 打开后的基线：drawer_count = "N"，triple_count = "M"，schema_version = 4
  When 执行 3 次任意参数的 `mempal_search` 调用
  Then `drawer_count` 仍为 "N"
  And `triple_count` 仍为 "M"
  And `schema_version` 仍为 4

Scenario: query 匹配 0 条 drawer 时 mempal_search 返回合法空 response
  Test:
    Filter: test_zero_result_query_returns_empty_response
    Level: integration
    Test Double: tempfile_palace_db
    Targets: src/mcp/server.rs
  Given tempfile palace.db 中至少有 "1" 条 drawer，但没有一条匹配 query "nonexistent_xyzqqq_impossible_match"
  When 以 `mempal_search` 用上述 query 调用
  Then 返回的 `SearchResponse.results` 长度为 "0"
  And 调用未 panic 或返回 error
  And 响应 JSON 中 `results` 字段是空数组 `[]`（而非 null）

Scenario: analyze 的 entities 字段始终包含至少一个 3-letter code
  Test:
    Filter: test_analyze_entities_never_empty_after_code_mapping
    Level: unit
    Targets: src/aaak/signals.rs, src/aaak/codec.rs
  Given 任意 content（包括空字符串、纯 whitespace、纯数字、无实体关键字的普通句子）
  When 调用 `aaak::signals::analyze(text)`
  Then `AaakSignals.entities` 长度 >= "1"
  And 第一个元素匹配正则 `^[A-Z]{3,4}$`（3-4 个大写字母，例如 "UNK" 或 "DEC"）
