---
id: benchmark-targets
title: "Benchmark Targets"
status: active
scope: cross-cutting (all specs)
refs:
  - "design.md"
  - "testing-strategy.md"
  - "repo-maintenance.md"
migrated_from: plan/verify/benchmark-targets.md
---

# Benchmark Targets

> Quantitative acceptance criteria used by milestone gates.

## Latency Targets

| Operation | Target | Condition | Spec Owner |
|-----------|--------|-----------|-----------|
| Symbol lookup (warm) | p95 < 300ms | Single repo, <10k files | 001-core-mvp |
| Symbol lookup (cold) | p95 < 2000ms | First query after startup | 001-core-mvp |
| `search_code` (warm) | p95 < 500ms | Lexical federated retrieval | 002-agent-protocol |
| `get_file_outline` | p95 < 50ms | SQLite-only query path | 002-agent-protocol |
| `get_code_context` | p95 < 800ms | Token budget fitting | 003-structure-nav |
| Branch overlay query | p95 < 500ms | Base + overlay merge | 005-vcs-core |
| `diff_context` | p95 < 1000ms | Medium branch (~50 changed files) | 006-vcs-ga-tooling |

## Precision Targets

| Metric | Target | Intent | Spec Owner |
|--------|--------|--------|-----------|
| `locate_symbol` top-1 precision | >= 90% | Symbol lookup | 001-core-mvp |
| `locate_symbol` top-3 precision | >= 97% | Symbol lookup | 001-core-mvp |
| `search_code` top-3 precision | >= 75% | Symbol intent | 002-agent-protocol |
| `search_code` top-5 precision | >= 60% | NL intent (>=100 stratified queries) | 008-semantic-hybrid |
| Hybrid vs lexical MRR uplift | >= 15% | NL intent (same stratified set) | 008-semantic-hybrid |
| Branch result correctness | 100% | VCS mode | 005-vcs-core |
| Rerank failure query impact | 0 | Fail-soft guarantee | 008-semantic-hybrid |

## Indexing/Sync Speed Targets

| Operation | Target | Condition | Spec Owner |
|-----------|--------|-----------|-----------|
| Full bootstrap | < 60s | 5k-file repo | 001-core-mvp |
| Incremental sync | < 5s | 10 changed files | 005-vcs-core |
| Branch overlay bootstrap | < 15s | 50 changed files from merge-base | 005-vcs-core |
| Branch overlay bootstrap (large) | < 30s | 100 changed files from merge-base | 005-vcs-core |
| Prewarm duration | < 3s | <10k files | 002-agent-protocol |

## Resource Usage Targets

| Resource | Target | Condition |
|----------|--------|-----------|
| Tantivy index size | < 3x source size | Typical code repo |
| SQLite DB size | < 50MB | 10k files |
| Memory (`serve-mcp` idle) | < 100MB | Single project |
| Memory (`serve-mcp` active) | < 500MB | Concurrent query load |

## Spec Ownership Summary

| Spec | Targets Owned | Validation Gate |
|------|---------------|-----------------|
| 001-core-mvp | core lookup latency/precision, bootstrap speed, resource baseline | G1 |
| 002-agent-protocol | response latency and prewarm behavior | G2 |
| 003-structure-nav | context assembly latency | G3 |
| 004-workspace-transport | no new numeric target; inherits system constraints | G4 |
| 005-vcs-core | overlay latency/sync speed/branch correctness | G5 |
| 006-vcs-ga-tooling | diff/ref tooling latency and GA workflow reliability | G6 |
| 007-call-graph | no new hard latency target; inherits existing constraints | G7 |
| 008-semantic-hybrid | natural-language relevance improvement | G8 |
| 009-distribution | no new runtime targets; verifies distribution operability | G9 |

## Measurement Method

- Latency: tracing spans + benchmark harness
- Precision/relevance: labeled query suite on fixture repos
- NL semantic benchmark suite should be stratified by language (Rust/TypeScript/Python/Go, >= 20 queries each)
- Sync speed: fixture branch operations with fixed change sets
- Resource usage: `doctor --stats` + CI sampling

## Regression Policy

- Latency regression > 20% on benchmark suite blocks merge
- Precision drop > 5% on relevance suite blocks merge
- Resource regressions are release-readiness blockers when sustained
