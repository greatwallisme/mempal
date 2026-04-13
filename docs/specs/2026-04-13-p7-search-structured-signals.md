# P7 — `mempal_search` structured signals — 设计文档

**日期**：2026-04-13
**状态**：Draft — awaiting review
**前置**：
- `docs/specs/2026-04-08-mempal-design.md`（mempal v1 总体设计）
- `docs/specs/2026-04-13-cowork-peek-and-decide.md`（P6，为 P7 提供 MCP ClientInfo 捕获、tests/ 目录约定等基础）

**关联 drawer（mempal 记忆层）**：
- `drawer_mempal_default_e7fcd33f` (Codex, role-level): AAAK 应该是 "agent-facing output-side working-memory interface"
- `drawer_mempal_default_9f8326c6` (narrow superseding): P7 Proposal C 取消；AAAK codec 的 "compression" 子命题被实证否证；role-level 定位保留
- `drawer_agent_diary_claude_a9fee141` (CHECK-BEFORE-EXTRAPOLATE lesson)

**关联 spec**：`specs/p7-search-structured-signals.spec.md`（已创建，lint quality 100%）

## 一句话定位

> **不动 `SearchResultDto.content` 的 raw 语义，给 `mempal_search` 响应里每一条 `SearchResultDto` 无条件附加 5 个显式结构化字段（`entities` / `topics` / `flags` / `emotions` / `importance_stars`），通过一个新的 `crate::aaak::signals::analyze` 公共分析 API 在 response 构造阶段计算。**

这是 P7 Proposal C 被实证否证后的**路线修正**。之前的 Proposal C 试图通过 `format="aaak"` 参数改变 `content` 字段的语义（raw → compressed view），empirical 测量证明 AAAK codec 不是 byte-level 压缩器（267B→381B, +42%），整个"token 节省"前提崩塌。修正路径是**放弃 format 参数轴，直接把结构化 metadata 作为独立显式字段**加到 DTO——agent 从此可以 `result.flags.contains("DECISION")` 过滤、按 `result.importance_stars` 排序、按 `result.entities` 精确匹配，**无需解析任何 AAAK 语法**。

## 动机与背景

### Proposal C 为什么失败

2026-04-13 的 P7 brainstorming round 提出的方案 C 是：给 `mempal_search` 加 `format: Option<String>` 参数，`format="aaak"` 时调用 `AaakCodec::default().encode(...)` 把每条 drawer 的 `content` 替换成 AAAK document 字符串，假设这能给 agent 省 context tokens。

Codex partner review 和我的复测都证明这个前提是错的：

| 输入 | AAAK encode 输出 | 变化 |
|---|---|---|
| 267B 决策文本（Arc<Mutex<Option<String>>>...）| 381B | **+42%** |
| 113B 短文本（Codex 的独立测量）| 213B | **+88%** |

根本原因在 `src/aaak/codec.rs:243` 和 `src/aaak/model.rs:83-116`：`AaakCodec::encode` 把完整原文塞进 `zettel.quote`，`Display` 再把它原样输出到 document 里，**并额外附加** header (V1|wing|room|date|source) + entity codes + topics + stars + emotions + flags。输出一定比输入长——这不是压缩器，是**带结构化标注的包装器**。

`docs/aaak-dialect.md:330` 其实早就明确写过：

> 最诚实的定位：AAAK 是面向 AI 的速记索引格式。**它的核心价值不在于"无损压缩"**，而在于"跨模型可读的高效上下文摘要"。

P0 时代的项目文档和 P7 brainstorming 的假设之间存在张力——assumption "AAAK 压缩" 从 docs 到 drawer 被无意识传递下来，直到 empirical 测量才被识别。完整归因见 `drawer_mempal_default_9f8326c6`。

### 修正路径：显式结构化字段

既然 AAAK 不压缩，那 agent consumption 的正确做法是**把 AAAK codec 内部已经提取出来的结构化信号**（entities, topics, flags, emotions, weight）**直接暴露到 DTO 层**，而不是让 agent 拿到 AAAK document 再反过来 parse。

- Agent 消费路径：`result.flags.contains("DECISION") && result.importance_stars >= 3`
- 对比被取消的 Proposal C：`parse_aaak_grammar(result.content).find_zettel_with_flag("DECISION")`

**explicit fields > implicit grammar** 是 API 设计的基础常识。P5 已经做过一次同模式的决策：把 drawer 的 `importance` 从隐式 wake-up 排序升级为显式字段，P7 是**同一 pattern 推广到 search result**。

## 核心洞察：signals 是 codec 的副产品

`AaakCodec::encode` 内部已经调用了 5 个提取函数计算结构化信号：

| 位置 | 函数 | 输出 |
|---|---|---|
| `src/aaak/codec.rs:346` | `extract_entities` | `Vec<String>` 原始实体候选字符串（ASCII 或 CJK 名称，尚未映射为 3-letter code；code 映射由 `default_entity_code` 完成，在 `analyze` 里显式调用）|
| `src/aaak/codec.rs:370` | `extract_topics` | `Vec<String>` 话题关键词 |
| `src/aaak/codec.rs:403` | `detect_flags` | `Vec<String>` 类别标签 (DECISION/CORE/PIVOT/...) |
| `src/aaak/codec.rs:419` | `detect_emotions` | `Vec<String>` 情感编码 |
| `src/aaak/codec.rs:436` | `infer_weight` | `u8` 基于 flags 推导的重要性权重 |

这些函数目前是 module-level `fn`（私有），只被 `encode()` 内部调用。**P7 的工程动作是把它们抬到一个公共 analysis API 背后**，让 MCP 层可以在不接触 AAAK 语法的前提下消费它们。

## 架构决策记录

| 决策 | 选择 | 理由 |
|---|---|---|
| 暴露形式 | 在 `SearchResultDto` 加 5 个显式字段，非 opt-in | Agent 直接消费，无需 parse AAAK；无新契约轴 |
| `content` 语义 | **不变**，始终是 raw drawer text | 避免重演 Proposal C 的契约漂移；drawer_id + source_file + content 三者共同构成权威引用 |
| API 抽象层 | 新增 `src/aaak/signals.rs` 导出 `AaakSignals` + `pub fn analyze` | MCP 层不能 import `codec.rs` 私有 fn；需要明确的公共入口 |
| 私有函数可见性 | 将 **5 个** extractor (`extract_entities` / `extract_topics` / `detect_flags` / `detect_emotions` / `infer_weight`) + `normalize_whitespace` + `default_entity_code` 共 **7 处** 从 `fn` 升为 `pub(crate) fn` | 最小侵入；不改逻辑，只改 visibility |
| `importance_stars` 范围 | **2-4**（直接来自 `infer_weight(&flags)`，else 分支返回 2；TECHNICAL 返回 3；DECISION/PIVOT 返回 4。**不做 `.max(1)` 后处理**——min 已经是 2）| 直接反映 `infer_weight` 在 `src/aaak/codec.rs:436-446` 的实际行为，避免引入"0 或 1 是特殊值"的错觉 |
| 是否加新 MCP 参数 | **不加**。无条件附加，不需要 `format` / `include_signals` / 等任何 flag | 简化契约面；Codex tightening 的核心要求 |
| 是否 deprecate 既有 CLI（`mempal compress` / `wake-up --format aaak`）| **不 deprecate**，保留为 legacy | 保守；避免同时改两件事 |
| 持久化 | 无。每次 search 都现算 | 遵守 P0 约束 `drawer_d1e88c52`："no AAAK persistence, output-side only"；符合 on-demand 原则 |
| 是否触 `drawers` 表 schema | 不触 | `importance_stars` 来自 signal 推导，不来自 DB 列 |
| 向后兼容策略 | Pure additive。旧 client 忽略新字段即可 | JSON extension 的标准 pattern |

## 数据流

**当前路径**（P6 完成状态）：

```
MCP client → mempal_search(query, wing?, room?, top_k?)
  ↓
resolve_route → RouteDecision
  ↓
search_with_vector (BM25 + vector + RRF + tunnel hints)
  ↓
Vec<SearchResult>
  ↓
results.into_iter().map(SearchResultDto::from)
  ↓
Json(SearchResponse { results })
```

**P7 之后**：

```
MCP client → mempal_search(query, wing?, room?, top_k?)
  ↓
resolve_route                                        ← unchanged
  ↓
search_with_vector                                   ← unchanged
  ↓
Vec<SearchResult>                                    ← unchanged
  ↓
for each result:
    let signals = aaak::signals::analyze(&result.content);
    build SearchResultDto {
        content: result.content,   // ← RAW, 未变
        drawer_id, source_file, wing, room, similarity, route, tunnel_hints, // ← 未变
        entities: signals.entities,                  // ← NEW
        topics: signals.topics,                      // ← NEW
        flags: signals.flags,                        // ← NEW
        emotions: signals.emotions,                  // ← NEW
        importance_stars: signals.importance_stars,  // ← NEW
    }
  ↓
Json(SearchResponse { results })
```

**三个关键不变量**：

1. `content` **字节级不变**——调用前后完全相同，包括空白符和 trailing newlines
2. `drawer_id`、`source_file`、`wing`、`room`、`similarity` 五个引用/定位字段**字节级不变**
3. `results.len()` 不变——没有 drawer 因为 analyze 被 drop

## 新公共 API：`crate::aaak::signals`

**新文件 `src/aaak/signals.rs`**：

```rust
//! Public analysis layer extracted from AaakCodec internals.
//!
//! Provides structured signal extraction (entities, topics, flags, emotions,
//! importance) from arbitrary text, without going through the full AAAK
//! encoding pipeline. Used by `mempal_search` to attach structured metadata
//! to `SearchResultDto` instances.

use serde::{Deserialize, Serialize};

/// Structured signals extracted from a piece of text by the AAAK analysis
/// primitives. Mirrors the fields produced by `AaakCodec` internally, but
/// without the AAAK document format wrapping.
///
/// IMPORTANT sentinel semantics (matching existing extractor behavior —
/// P7 does NOT change extractor algorithms):
/// - `entities`: always has >= 1 entry. If no entities detected, contains
///   `["UNK"]` (the `DEFAULT_ENTITY_CODE` sentinel).
/// - `flags`: always has >= 1 entry. If no flag keywords matched,
///   contains `["CORE"]` (the sentinel for "uncategorized"). Agents
///   filter real decisions via `flags.contains("DECISION")`, and detect
///   "no explicit category" via `flags == ["CORE"]`.
/// - `emotions`: always has >= 1 entry. Defaults to `["determ"]`
///   (the `DEFAULT_EMOTION` sentinel).
/// - `topics`: can be empty. `extract_topics` does not add a default.
/// - `importance_stars`: always 2, 3, or 4. Direct output of
///   `infer_weight(&flags)`, whose else branch returns 2. No
///   post-processing floor applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AaakSignals {
    pub entities: Vec<String>,
    pub topics: Vec<String>,
    pub flags: Vec<String>,
    pub emotions: Vec<String>,
    pub importance_stars: u8,
}

/// Analyze a piece of text and extract structured signals.
///
/// This is the public entry point that `mempal_search` calls per result.
/// It does NOT produce an AAAK-formatted document; it only extracts the
/// five structured signal fields. The caller is free to attach them as
/// explicit fields on whatever DTO they construct.
///
/// This function is intentionally infallible: the underlying extractors
/// already have explicit fallback behavior for degenerate inputs (empty
/// flags → `["CORE"]`, empty emotions → `["determ"]`, empty entities
/// after code mapping → `["UNK"]`), and Rust's `&str` guarantees valid
/// UTF-8 at the type level.
pub fn analyze(text: &str) -> AaakSignals {
    use std::collections::BTreeSet;

    let normalized = super::codec::normalize_whitespace(text);

    // Entities: raw extraction → 3-letter code mapping → dedup → UNK fallback.
    // Mirrors encode()'s behavior at src/aaak/codec.rs:213-231 but stateless
    // (no custom entity_map, uses only default_entity_code).
    let mut entity_codes: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    for entity in super::codec::extract_entities(&normalized) {
        let code = super::codec::default_entity_code(&entity);
        if seen.insert(code.clone()) {
            entity_codes.push(code);
        }
    }
    if entity_codes.is_empty() {
        entity_codes.push("UNK".to_string());
    }

    // Flags already contain "CORE" fallback when no flag keyword matched
    // (see detect_flags src/aaak/codec.rs:412-414). We pass it through
    // to infer_weight directly — no .max(1) post-processing.
    let flags = super::codec::detect_flags(&normalized);
    let importance_stars = super::codec::infer_weight(&flags);

    AaakSignals {
        entities: entity_codes,
        topics: super::codec::extract_topics(&normalized),  // may be empty
        emotions: super::codec::detect_emotions(&normalized),  // always >= 1
        importance_stars,
        flags,
    }
}
```

**`src/aaak/codec.rs`**：**7 处** `fn` → `pub(crate) fn` 的可见性变更：

- `normalize_whitespace` (line 337)
- `extract_entities` (line 346)
- `extract_topics` (line 370)
- `detect_flags` (line 403)
- `detect_emotions` (line 419)
- `infer_weight` (line 436)
- `default_entity_code` (line 480) — 用于 `signals::analyze` 的 stateless 3-letter code 映射

**没有**逻辑变更。纯 visibility 调整。

**`src/aaak/mod.rs`**：

```rust
#![warn(clippy::all)]

mod codec;
mod model;
mod parse;
pub mod signals;   // ← new
mod spec;

pub use codec::AaakCodec;
pub use model::{...};
pub use signals::{AaakSignals, analyze};  // ← new
pub use spec::generate_spec;
```

## `SearchResultDto` 扩展

**`src/mcp/tools.rs`** 的 `SearchResultDto` 扩展后（5 个新字段，doc comments 说明来源）：

```rust
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SearchResultDto {
    // ---- existing fields (all unchanged) ----
    pub drawer_id: String,
    pub content: String,         // RAW drawer text, never compressed or transformed
    pub wing: String,
    pub room: Option<String>,
    pub source_file: String,
    pub similarity: f32,
    pub route: RouteDecisionDto,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tunnel_hints: Vec<String>,

    // ---- NEW (P7): structured signals, always populated ----
    /// 3-letter entity codes (e.g. "DEC", "ARC", "MUT") extracted from `content`
    /// and mapped through `default_entity_code`. **Always has >= 1 entry** —
    /// when no entities are detected, contains `["UNK"]`. Enables agents to
    /// filter/group results without parsing AAAK grammar.
    #[serde(default)]
    pub entities: Vec<String>,

    /// Topic keywords (snake_case) extracted from `content`. Can be empty
    /// when no topics were detected (extract_topics does not add a default).
    #[serde(default)]
    pub topics: Vec<String>,

    /// Category flags: DECISION, CORE, PIVOT, ORIGIN, TECHNICAL, SENSITIVE.
    /// **Always has >= 1 entry** — when no flag keywords matched, contains
    /// `["CORE"]` (the "uncategorized" sentinel). Agents filter real
    /// decisions via `flags.contains("DECISION")` and detect the
    /// uncategorized case via `flags == ["CORE"]`.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Emotion codes (mindset tags) extracted from `content`. **Always has
    /// >= 1 entry** — when no emotions detected, contains `["determ"]`
    /// (the `DEFAULT_EMOTION` sentinel).
    #[serde(default)]
    pub emotions: Vec<String>,

    /// Importance rating, always **2, 3, or 4**. Direct output of
    /// `infer_weight(&flags)`. `4` when flags contain DECISION or PIVOT,
    /// `3` when TECHNICAL, `2` otherwise (including the CORE fallback case).
    #[serde(default = "default_importance_stars")]
    pub importance_stars: u8,
}

fn default_importance_stars() -> u8 { 2 }
```

**Serde 契约**：`#[serde(default)]` 仅用于反序列化兼容（老客户端发送不含新字段的请求时，服务端用默认值填入）。**不使用 `skip_serializing_if`**——服务端响应始终 emit 这 5 个字段，客户端可以依赖字段永远存在（非 null, 非 undefined）。这避免了"`always populated` vs `omit when empty`"的契约矛盾。

## MCP handler wiring

**`src/mcp/server.rs`** 的 `mempal_search` 尾段改动（~15 LoC）：

```rust
// existing:
let results = search_with_vector(&db, &request.query, &query_vector, route, request.top_k.unwrap_or(10))
    .map_err(|error| ErrorData::internal_error(format!("search failed: {error}"), None))?;

// P7: attach structured signals per result before serializing
let dtos = results
    .into_iter()
    .map(|result| {
        let signals = crate::aaak::analyze(&result.content);
        SearchResultDto {
            drawer_id: result.drawer_id,
            content: result.content,
            wing: result.wing,
            room: result.room,
            source_file: result.source_file,
            similarity: result.similarity,
            route: result.route.into(),
            tunnel_hints: result.tunnel_hints,
            entities: signals.entities,
            topics: signals.topics,
            flags: signals.flags,
            emotions: signals.emotions,
            importance_stars: signals.importance_stars,
        }
    })
    .collect();

Ok(Json(SearchResponse { results: dtos }))
```

**注意**：`SearchResultDto::from(SearchResult)` 这个既有的 `From` impl 需要删除或调整——它目前忽略新字段。P7 实施时两个选择：

- (a) 删除 `impl From<SearchResult> for SearchResultDto`，直接在 handler 里构造（上面的风格）
- (b) 保留 `From` impl 返回 signals-empty 版本，handler 接管填充 signals

我倾向 (a)，因为 `From` 语义在这里变成"部分信息丢失的转换"，不如显式写清楚。

## 使用场景

### 场景 1：agent 过滤 DECISION

Agent 问："我们在 mempal-core 里做过哪些架构决策？"

P6 之前：Agent 调 `mempal_search(query="mempal-core architectural decision")`，拿到 top-10 raw drawer，需要逐条扫 content 判断哪些是决策、哪些是探索性笔记。

P7 之后：Agent 调同样的 search，在响应端做 `results.filter(r => r.flags.includes("DECISION"))`，一次性拿到只是决策的子集。无 parsing，无 LLM 判断。

### 场景 2：agent 按 importance 排序

Agent 问："给我最重要的 P5 决策"。

P6 之前：Agent 靠 `similarity` 分数排序——但 similarity 衡量的是 query 匹配度，不是决策重要性。

P7 之后：Agent 可以用 `.sort_by_key(r => -r.importance_stars)` 或 `.filter(r => r.importance_stars >= 4)`，按项目内部的重要性刻度排序。

### 场景 3：agent 按 entity 精确匹配

Agent 已知某个 drawer 提到了某个特定的 entity code（比如 "MCP"），想找所有相关讨论。

P6 之前：Agent 只能靠 vector similarity 模糊匹配。

P7 之后：Agent 可以先发 `mempal_search(query="MCP")` 拿一批 candidate，然后 `.filter(r => r.entities.includes("MCP"))` 精确过滤。

## 使用场景走查

| 场景 | 之前 | P7 之后 |
|---|---|---|
| 只要 DECISION 类 drawer | 需要 LLM 逐条判断 content | `r.flags.includes("DECISION")` 直接过滤 |
| 按重要性排序 | 只能按 similarity（query 匹配度） | `sort_by(r => r.importance_stars)` |
| 按 entity code 精确匹配 | 只能 vector 近似 | `r.entities.includes("MCP")` |
| 消费 AAAK 语法 | 需要 parse BNF | 不需要 |
| 原文引用（quote drawer content）| `r.content` | `r.content` 不变 |

## 测试 plan

新文件 `tests/search_structured_signals.rs`。所有测试 hermetic，使用真 `Database` + 真 embedder（`tempfile`）或 mock search pipeline。

| # | Scenario | 证明什么 |
|---|---|---|
| S1 | search 响应包含 5 个 signal 字段，非 null | pure existence check |
| S2 | DECISION 关键字的 content → `flags` 包含 `"DECISION"` 且 `importance_stars >= 2` | flags 正确提取 + infer_weight 正确 |
| S3 | 无 category 关键字的普通文本 → `importance_stars == 2` 且 `flags == ["CORE"]` | sentinel default + infer_weight else 分支 |
| S4 | **纯 CJK** content（无任何 ASCII 实体）→ `entities != ["UNK"]` | 证明 jieba CJK 分支真正产出了非 UNK 的 entity code；如果 fixture 含 ASCII，测试会被 ASCII 路径"意外通过"，无法证明 jieba 路径健康 |
| S5 | 空 content → `entities == ["UNK"]`, `flags == ["CORE"]`, `emotions == ["determ"]`, `topics == []`, `importance_stars == 2` | degenerate input + sentinel defaults |
| S6 | 纯 whitespace content → 同 S5 | degenerate input 2 |
| S7 | **Invariant**: search 前后 `r.content == drawer.content`（byte-level） | raw 保全 |
| S8 | **Invariant**: search 前后 `drawer_id` / `source_file` 不变 | 引用一致 |
| S9 | **Invariant**: search 结果数量在"加了 signals"和"没加 signals"两个模式下相等 | 无 drop |
| S10 | **Invariant**: palace.db schema_version 在 search 前后都是 4，`drawer_count` 不变 | no DB side effects（沿用 P6 的 real-snapshot 模式） |
| S11 | 0-result query 返回合法空 `SearchResponse`（`results == []`，非 null） | empty-path edge case |
| S12 | `analyze` 的 `entities` 字段始终 >= 1 entry（正则 `^[A-Z]{3,4}$`） | 3-letter code invariant |

**Happy paths**：S1, S2, S4
**Exception/invariant/sentinel paths**：S3, S5, S6, S7, S8, S9, S10, S11, S12

9 个异常/不变量，3 个 happy path — 满足 `lint` 的 "exception ≥ happy" 规则（远超）。

## 代码量估算

| 文件 | 改动 | LoC |
|---|---|---|
| `src/aaak/codec.rs` | **7 处** `fn` → `pub(crate) fn`（见 Part 1 列表）| +7（减 7 加 7，净 +0）|
| `src/aaak/signals.rs` (new) | `AaakSignals` + `analyze` + rustdoc | +45 |
| `src/aaak/mod.rs` | `pub mod signals` + re-exports | +3 |
| `src/mcp/tools.rs` | `SearchResultDto` 加 5 字段 + rustdoc | +35 |
| `src/mcp/server.rs` | `mempal_search` handler 组装 DTO（删 `From` impl 或改写）| +20 |
| `tests/search_structured_signals.rs` (new) | 12 个 scenario，hermetic | +220 |
| `docs/specs/2026-04-13-p7-search-structured-signals.md` (this file) | 设计文档 | ~400 lines |
| `specs/p7-search-structured-signals.spec.md` | agent-spec 合约（已创建，lint 100%） | ~200 lines |
| `docs/plans/2026-04-13-p7-implementation.md` (writing-plans 阶段) | 实施计划 | ~280 lines |

**Rust 代码 delta**：~103 生产 + ~220 测试 = **~323 LoC**。比 P6 略小。

**0 schema migration，0 新 runtime dep，0 既有测试需要修改**（只要 SearchResultDto 的 From impl 不破坏现有行为）。

## 风险和限制

| 风险 | 缓解 |
|---|---|
| 5 个新字段 serializer overhead 对每次 search 有成本 | `analyze()` 是纯字符串处理，测量一下 p99 latency 应该 <1ms per drawer；top_k=10 下总开销 <10ms。如果实测有问题加一个 `include_signals: bool` 参数 opt-out（不建议，目前无证据需要）|
| AAAK codec 和 `signals::analyze` 共用 extractors；codec.rs 未来如果改 extractor 语义，signals 会跟着变 | 这是正确的耦合方向——两者本来就应该语义一致。通过单元测试锁 extractor 输出 |
| `signals.rs` 引用 `super::codec::*` 私有实现细节 | `pub(crate)` 限定保证外部 crate 看不到。内部耦合是可接受的 |
| 新字段如果 wire 大小过大（比如 50 个 entity codes）| 本次刻意**不使用** `skip_serializing_if`（保证契约一致性），接受"每个 response 都含这 5 个字段"的稳定开销；如果未来 p99 wire size 成为问题，可以单独 spec 讨论 per-field top-k 裁剪 |
| Agent 消费新字段需要更新 tool description | `src/mcp/server.rs` 的 `mempal_search` tool description 字符串会顺手更新，explain 新字段 |
| 中文 content 依赖 jieba——但 jieba 已经是 P5 既有依赖 | 无新依赖，无新风险 |

## 明确不做（Out of scope）

- ❌ `format="aaak"` 参数——整个 format 轴从 P7 设计里移除
- ❌ 把 `content` 变成 AAAK document 或任何非 raw 形式
- ❌ 重写 AaakCodec 做真正的压缩（path a）——那是独立的 research-level 工作
- ❌ 给 `mempal_peek_partner` 加 signal 字段——peek 的职责是 raw live session，不需要结构化分析
- ❌ Deprecate `mempal compress` / `mempal wake-up --format aaak` CLI subcommands——保留为 legacy
- ❌ 往 `drawers` 表加 entity/flag/topic 列——违反 "no persistence" P0 约束
- ❌ longmemeval benchmark 作为 P7 acceptance criterion——benchmark 框架不适合，且 P7 的验证轴是"结构化字段正确性"而非"retrieval 质量"
- ❌ CLI `mempal search` 输出里显示 signals——CLI 用户看 raw content + 相似度就够了
- ❌ Agent 消费策略的协议化指引（"怎么用 flags 过滤"之类）——写在 tool description 里就够，不进 MEMORY_PROTOCOL

## Follow-up

P7 合入后：

1. **观察 agent 使用 pattern**：看 Claude Code / Codex 是否真的用 `flags` / `importance_stars` 过滤。可以通过 session jsonl 看调用方是否 ref 了新字段。如果**没人用**，下一步是考虑删除 AAAK codec 整个 subsystem（signals 可以脱离 codec 独立存在）
2. **如果使用 pattern 显著**：考虑把 signals 扩展到 `mempal_peek_partner`（目前刻意不做）、`mempal_ingest` 的 response（dry_run 预览 signals 帮助 agent 决定是否要 ingest）
3. **如果 signals 的 extractor 准确率成为问题**：单独开 P7a spec 讨论是否用 LLM-based extractor 替代 heuristic。目前无证据需要
4. **`importance_stars` 和 drawer 表的 `importance` 字段的关系**：drawer 表已有 `importance i32` 列（P5），用户可以手动设置；`importance_stars` 来自 signal 推导。两者是独立的，但未来可能需要一个"真实 importance"合成逻辑（P7a）

## 开放问题（非阻塞）

1. `SearchResultDto::from(SearchResult)` 删还是改？
   **初步答案**：删。从 handler 里直接构造，把 "signals 来自哪里" 显式化
2. 如果 drawer 的 `importance` (DB column, P5) ≠ `importance_stars` (P7 signal)，返回哪个？
   **初步答案**：两者都返回。`importance_stars` 来自 content 启发式；DB `importance` 来自用户或 agent 显式设置。不合并，各有语义。可以考虑加一个 `importance_db: Option<i32>` 字段暴露 DB 值
3. 要不要把 `signals.rs` 的 `analyze` 也从 CLI 的 `mempal compress` 路径中走一遍？
   **初步答案**：不。`mempal compress` 是 legacy path，不动
