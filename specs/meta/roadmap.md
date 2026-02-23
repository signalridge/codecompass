# CodeCompass Development Roadmap

> Canonical phase/version roadmap.
> Design details live in [design.md](design.md); this file focuses on sequencing and release gates.

## Executive Summary

CodeCompass ships as a Rust-first, zero-external-service code navigation engine.
Delivery is optimized around two principles:

1. **Correctness before breadth** (especially VCS correctness)
2. **Additive capability layering** (tooling after stable foundations)

## Version Mapping

| Phase | Version | Release Type | Gate Description |
|-------|---------|-------------|-----------------|
| 0+1 | v0.1.0 | Alpha | Core indexing/search/locate + MCP baseline |
| 1.1 | v0.2.0 | Alpha | Agent protocol optimization (`detail_level`, outline, health) |
| 1.5a | v0.3.0-rc | Beta-prep | Structure/navigation and token-budget context |
| 1.5b | v0.3.0 | Beta | Multi-workspace and HTTP transport |
| 2a | v0.9.0 | Beta | VCS core correctness (overlay + merge + sync + recovery) |
| 2b | v1.0.0 | **GA** | VCS GA tooling + portability + full GA validation |
| 2.5 | v1.1.0 | Feature | Call graph analysis |
| 3 | v1.2.0 | Feature | Semantic/hybrid retrieval and rerank |
| 4 | v1.3.0 | Distribution | Packaging, templates, guides, release polish |

**Key rule**: `v1.0.0` is reached only after both `005-vcs-core` and
`006-vcs-ga-tooling` gates pass.

## Phase Dependency Graph

```text
001-core-mvp
  -> 002-agent-protocol
    -> 003-structure-nav
      -> 004-workspace-transport
        -> 005-vcs-core
          -> 006-vcs-ga-tooling (v1.0.0 GA)
            -> 007-call-graph
              -> 008-semantic-hybrid
                -> 009-distribution
```

## Phase Summary

| Phase | Version | Spec | Key Deliverable | Gate |
|-------|---------|------|-----------------|------|
| 0+1 | v0.1.0 | [001-core-mvp](../001-core-mvp/) | `init/index/search/locate` baseline | G1 |
| 1.1 | v0.2.0 | [002-agent-protocol](../002-agent-protocol/) | `detail_level`, `get_file_outline`, health/prewarm | G2 |
| 1.5a | v0.3.0-rc | [003-structure-nav](../003-structure-nav/) | hierarchy/related/context tools | G3 |
| 1.5b | v0.3.0 | [004-workspace-transport](../004-workspace-transport/) | workspace routing + HTTP transport | G4 |
| 2a | v0.9.0 | [005-vcs-core](../005-vcs-core/) | branch overlay correctness core | G5 |
| 2b | v1.0.0 | [006-vcs-ga-tooling](../006-vcs-ga-tooling/) | diff/ref/ranking/ref-switch/export-import tooling | G6 (GA) |
| 2.5 | v1.1.0 | [007-call-graph](../007-call-graph/) | `get_call_graph` + symbol comparison | G7 |
| 3 | v1.2.0 | [008-semantic-hybrid](../008-semantic-hybrid/) | adaptive hybrid semantic + privacy-gated rerank | G8 |
| 4 | v1.3.0 | [009-distribution](../009-distribution/) | cross-platform release and onboarding assets | G9 |

## Ops Integration Timeline

| Ops Area | Integrate With | Trigger |
|----------|---------------|---------|
| CI/Security | Early 001 | Repo bootstrap |
| Repo Governance | Early 001 | First collaborative PRs |
| Release Pipeline | Late 001 / 002 | First publishable binaries |
| Maintenance Automation | 004+ | Dependency graph growth |

See [repo-maintenance.md](repo-maintenance.md) for operational details.

## Immediate Hardening Priorities (Implementation Smoothness)

These are cross-spec priorities to reduce implementation risk and agent/runtime friction.

| Priority | Item | Primary Specs |
|---|---|---|
| H1 | Stable follow-up handles (`symbol_id`, `symbol_stable_id`, `result_id`) across all retrieval outputs | 001, 003, 005, 006, 007 |
| H2 | Canonical error registry and envelope (`error.code`, `error.message`, `error.data`) | meta + all contracts |
| H3 | Startup compatibility + explicit reindex gate (`index_incompatible`) | 001, 002, 004 |
| H4 | Non-blocking MCP startup (handshake first, async prewarm) | 002 |
| H5 | Semantic complexity split (`off` / `rerank_only` / `hybrid`) with Track A first | 008 |
| H6 | Fail-soft rerank + strict external privacy gates | 008 |
| H7 | Deterministic `suggested_next_actions` in low-confidence/truncated responses | 001, 003, 007, 008 |

## Verification Plans

| Plan | File | Scope |
|------|------|-------|
| Testing Strategy | [testing-strategy.md](testing-strategy.md) | Unit, integration, E2E, relevance |
| Benchmark Targets | [benchmark-targets.md](benchmark-targets.md) | Latency, precision, sync speed, resources |

## Backlog (Unscheduled)

| Feature | Design Reference | Notes |
|---------|------------------|-------|
| `search_similar_symbol` | [design.md §10.1](design.md#101-candidate-v15v2-mcp-tools-high-value) | Candidate tool |
| Result diversification | [design.md §7.4](design.md#74-augment-inspired-search-behaviors-to-adopt-local-first) | Reduce duplicate-heavy top-k |
| Identifier-aware rewrite | [design.md §7.4](design.md#74-augment-inspired-search-behaviors-to-adopt-local-first) | Query normalization enhancement |
| Overlay eviction policy | [design.md §9.1](design.md#91-branch-aware-indexing-strategy-default-no-full-reindex) | Lifecycle optimization |
| Segment force-merge | [design.md §9.1](design.md#91-branch-aware-indexing-strategy-default-no-full-reindex) | Index maintenance |
| `codecompass migrate-index` | [design.md §15](design.md#15-schema-versioning-and-migration-plan) | Schema migration command |
| Local file watcher | [design.md §9.7](design.md#97-index-update-timing-and-trigger-policy-authoritative) | Event-driven sync |
| Periodic reconcile trigger | [design.md §9.7](design.md#97-index-update-timing-and-trigger-policy-authoritative) | Low-frequency consistency pass |
| Docker image for CI | 009 extension | Optional distribution channel |
| npm wrapper | 009 extension | Optional install channel |

## Resolved Open Questions

1. v1 language scope: Rust, TypeScript, Python, Go
2. Tokenizer scope: `code_camel`, `code_snake`, `code_dotted`, `code_path`
3. SQLite driver: `rusqlite`
