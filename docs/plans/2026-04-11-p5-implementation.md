# P5 Implementation Plan: MemPalace-Inspired Improvements

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement 5 features borrowed from MemPalace analysis, improving mempal's wake-up intelligence, knowledge graph usability, data quality, agent self-awareness, and format coverage.

**Architecture:** Build on existing mempal infrastructure (schema v3, 7 MCP tools, model2vec embedder). P5 features are independent — no circular dependencies — but share a logical grouping.

**Specs:** All specs at `specs/p5-*.spec.md`, lint Quality 100%.

**Estimated total:** 3.5 days

---

## Dependency Graph

```
p5-wake-up-importance (1d)  ── schema v4, standalone
       │
       ▼
p5-kg-timeline-stats (0.5d) ── extends mempal_kg, standalone
       │
       ▼
p5-semantic-dedup (0.5d)    ── extends mempal_ingest, needs embedder
       │
       ▼
p5-agent-diary (0.5d)       ── protocol only, standalone
       │
       ▼
p5-format-support (1d)      ── extends mempal-ingest, standalone
```

No hard dependencies between tasks — ordering is by priority, not technical necessity.
Tasks 2+3 can be parallelized. Task 4 is zero-code (protocol + docs only).

---

## Task 1: L1 Importance-Ranked Wake-Up (P0)

**Spec:** `specs/p5-wake-up-importance.spec.md`

- [ ] **Step 1:** Schema v4 migration: `ALTER TABLE drawers ADD COLUMN importance INTEGER DEFAULT 0`. Bump `CURRENT_SCHEMA_VERSION` to 4 in `db.rs`.

- [ ] **Step 2:** Update `Drawer` struct in `types.rs` to include `importance: i32` field. Update all `Drawer` constructors across codebase.

- [ ] **Step 3:** Update `insert_drawer()` in `db.rs` to persist `importance`. Update `recent_drawers()` → rename to `top_drawers()`, sort by `importance DESC, CAST(added_at AS INTEGER) DESC`.

- [ ] **Step 4:** Update `mempal_ingest` MCP tool: `IngestRequest` gains optional `importance: Option<i32>` field. Default to 0 if omitted.

- [ ] **Step 5:** Update CLI `wake-up` command to use `top_drawers()` instead of `recent_drawers()`. Add `--top-k N` argument (default 5).

- [ ] **Step 6:** Write tests: `test_wake_up_importance_order`, `test_ingest_with_importance`, `test_default_importance`, `test_schema_v4_migration`.

- [ ] **Step 7:** Run `cargo test --workspace && cargo clippy --workspace -- -D warnings`.

- [ ] **Step 8:** Commit: `feat: add importance-ranked wake-up (schema v4)`

---

## Task 2: KG Timeline and Stats (P1)

**Spec:** `specs/p5-kg-timeline-stats.spec.md`

- [ ] **Step 1:** Add `timeline_for_entity()` method to `Database`: query triples where subject=entity OR object=entity, order by `valid_from ASC`.

- [ ] **Step 2:** Add `triple_stats()` method to `Database`: return total, active (valid_to IS NULL), expired, distinct entities, most common predicates (top 5).

- [ ] **Step 3:** Update `mempal_kg` MCP tool handler: add `"timeline"` and `"stats"` action branches. Timeline requires `subject` param. Stats requires no params.

- [ ] **Step 4:** Update `KgResponse` in `tools.rs` to carry optional `stats` field for stats action.

- [ ] **Step 5:** Add CLI subcommands: `mempal kg timeline <entity>` and `mempal kg stats`.

- [ ] **Step 6:** Write tests: `test_kg_timeline`, `test_kg_stats`.

- [ ] **Step 7:** Commit: `feat: add KG timeline and stats to mempal_kg`

---

## Task 3: Semantic Dedup Detection (P1)

**Spec:** `specs/p5-semantic-dedup.spec.md`

- [ ] **Step 1:** Add `dedup_threshold: Option<f64>` to `EmbedConfig` in `config.rs`. Default: `0.85`.

- [ ] **Step 2:** In MCP `mempal_ingest` handler (server.rs): after embedding the new content, before inserting, search for top-1 similar existing drawer using `search_by_vector()` with top_k=1.

- [ ] **Step 3:** If similarity >= threshold, add `duplicate_warning` to `IngestResponse`. Warning includes: similar drawer_id, similarity score, content preview (first 100 chars).

- [ ] **Step 4:** Update `IngestResponse` in `tools.rs` to include optional `duplicate_warning: Option<DuplicateWarning>`.

- [ ] **Step 5:** Update CLI ingest output to print warning when duplicate detected.

- [ ] **Step 6:** Handle edge case: skip dedup check when drawer_vectors table is empty or doesn't exist (first-ever ingest).

- [ ] **Step 7:** Write tests: `test_ingest_duplicate_warning`, `test_ingest_no_duplicate_warning`, `test_ingest_first_drawer_no_check`.

- [ ] **Step 8:** Commit: `feat: add semantic duplicate detection on ingest`

---

## Task 4: Agent Diary Convention (P2)

**Spec:** `specs/p5-agent-diary.spec.md`

- [ ] **Step 1:** Add Rule 5a to MEMORY_PROTOCOL in `protocol.rs`:
  ```
  5a. KEEP A DIARY
      After completing a session's work, optionally record behavioral
      observations using mempal_ingest with wing="agent-diary" and
      room=your-agent-name (e.g. "claude", "codex"). Prefix entries with
      OBSERVATION:, LESSON:, or PATTERN: to categorize. Diary entries
      help future sessions of any agent learn from past behavioral patterns.
  ```

- [ ] **Step 2:** Update `docs/usage.md` with Agent Diary section explaining the convention.

- [ ] **Step 3:** Write test: `test_protocol_contains_diary_rule` (assert MEMORY_PROTOCOL contains "DIARY").

- [ ] **Step 4:** Commit: `feat: add agent diary convention (protocol rule 5a)`

---

## Task 5: Slack + Codex Format Support (P2)

**Spec:** `specs/p5-format-support.spec.md`

- [ ] **Step 1:** Study Slack export format: `export/<channel>/messages.json` — array of `{user, text, ts}` objects.

- [ ] **Step 2:** Implement `normalize_slack()` in mempal-ingest: parse messages, group consecutive same-user messages, format as `> User: ...\nAssistant: ...`.

- [ ] **Step 3:** Study Codex CLI JSONL format: lines with `{type: "event_msg", ...}` — extract only these, skip synthetic context.

- [ ] **Step 4:** Implement `normalize_codex()` in mempal-ingest.

- [ ] **Step 5:** Extend `detect_format()` to recognize Slack directories (contains `messages.json` with `ts` field) and Codex JSONL (lines containing `event_msg`).

- [ ] **Step 6:** Write tests with fixture data: `test_ingest_slack_dm`, `test_ingest_codex_jsonl`, `test_format_auto_detect`.

- [ ] **Step 7:** Commit: `feat: add Slack DM and Codex CLI format support`

---

## Verification Checklist

After all 5 tasks, verify:

- [ ] `cargo test --workspace --all-features` — all pass
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
- [ ] `cargo fmt --all --check` — clean
- [ ] `mempal status` — shows schema_version: 4
- [ ] `mempal wake-up` — returns importance-sorted drawers
- [ ] `mempal kg timeline Kai` — returns chronological triples
- [ ] `mempal kg stats` — returns KG statistics
- [ ] MCP `mempal_ingest` with duplicate content — returns warning
- [ ] `mempal search "lesson" --wing agent-diary` — diary searchable
- [ ] Protocol text contains Rule 5a (DIARY)

---

## Parallelization Opportunities

```
Sequential:
  Task 1 (schema v4) → Tasks 2+3 (can parallel) → Task 4 → Task 5

Parallel pairs:
  Task 2 (KG) + Task 3 (dedup) — no shared code
  Task 4 (diary) + Task 5 (formats) — no shared code
```

If using subagent-driven development, dispatch Tasks 2+3 as parallel agents after Task 1 completes.
