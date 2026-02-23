# Cross-Spec Execution Order

> Definitive inter-spec execution sequence.
> Intra-spec ordering is defined in each `tasks.md`.

## Spec Dependency Chain

```text
001-core-mvp
  -> 002-agent-protocol
    -> 003-structure-nav
      -> 004-workspace-transport
        -> 005-vcs-core
          -> 006-vcs-ga-tooling
            -> 007-call-graph
              -> 008-semantic-hybrid
                -> 009-distribution
```

## Cross-Spec Hardening Checklist (Before/While Implementation)

Apply these in parallel with feature development to reduce rework:

1. Enforce stable follow-up handles in retrieval outputs.
2. Enforce canonical error envelope and code registry.
3. Enforce startup compatibility checks and explicit reindex gate.
4. Keep MCP handshake non-blocking; run prewarm asynchronously.
5. Deliver semantic Track A (`rerank_only`) before Track B (`hybrid`).

## Why This Order Is Optimized

1. **Foundation-first**: all parser/index/query primitives are complete before cross-ref tooling.
2. **VCS risk split**: 005 isolates correctness-critical path; 006 adds tooling after core is stable.
3. **Feature layering**: call graph and semantic retrieval build on stable VCS semantics.
4. **Distribution last**: packaging and guides after core capability set is stable.

## Global Critical Path

```text
001 -> 002 -> 003 -> 004 -> 005 -> 006 -> 007 -> 008 -> 009
```

## Per-Spec Execution Summary

| Spec | Tasks | Phases | Task Range | Depends On | Suggested Focus |
|------|-------|--------|-----------|------------|-----------------|
| 001-core-mvp | 81 | 8 | T001-T081 | -- | Bootstrap + indexing + search baseline |
| 002-agent-protocol | 58 | 7 | T082-T139 | 001 | Agent payload and protocol optimization |
| 003-structure-nav | 56 | 7 | T140-T195 | 002 | Structure graph and context tooling |
| 004-workspace-transport | 44 | 5 | T196-T239 | 003 | Multi-workspace and transport |
| 005-vcs-core | 56 | 6 | T240-T295 | 004 | Overlay correctness core |
| 006-vcs-ga-tooling | 29 | 6 | T296-T324 | 005 | GA tooling, ref helpers, portability |
| 007-call-graph | 39 | 6 | T325-T363 | 006 | Call graph and symbol diff tooling |
| 008-semantic-hybrid | 48 | 8 | T364-T411 | 007 | Hybrid retrieval and rerank |
| 009-distribution | 39 | 6 | T412-T450 | 008 | Release/distribution/onboarding |

## Milestone Gates

| Gate | Version | Blocking Validation | Blocks Next |
|------|---------|---------------------|-------------|
| G1 | v0.1.0 | Core MVP acceptance suite | 002 |
| G2 | v0.2.0 | Agent protocol acceptance suite | 003 |
| G3 | v0.3.0-rc | Structure/navigation acceptance suite | 004 |
| G4 | v0.3.0 | Workspace/transport acceptance suite | 005 |
| G5 | v0.9.0 | VCS core correctness suite (`SC-400`..`SC-405`) | 006 |
| G6 | v1.0.0 | VCS GA tooling suite (`SC-500`..`SC-505`) | 007 |
| G7 | v1.1.0 | Call graph acceptance suite | 008 |
| G8 | v1.2.0 | Semantic/hybrid acceptance suite | 009 |
| G9 | v1.3.0 | Distribution acceptance suite | -- |

## Parallelization Guidance (Within a Spec)

- Prioritize `[P]` tasks by file independence.
- Keep correctness chains serial inside each spec's critical path.
- In VCS specs, avoid parallel writes touching same `(project, ref)` coordination paths.

## Recommended Sprint Envelope (Single Team)

| Sprint | Primary Specs | Outcome |
|--------|---------------|---------|
| S1-S2 | 001 | v0.1.0 |
| S3 | 002 | v0.2.0 |
| S4 | 003 | v0.3.0-rc |
| S5 | 004 | v0.3.0 |
| S6-S7 | 005 + 006 | v1.0.0 |
| S8 | 007 | v1.1.0 |
| S9 | 008 | v1.2.0 |
| S10 | 009 | v1.3.0 |
