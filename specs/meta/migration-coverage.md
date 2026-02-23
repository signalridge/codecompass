# Migration Coverage: `plan/` + `plan.md` -> `specs/`

> Canonical migration audit for the planning-to-spec transition.
> Use this file to verify completeness and avoid drift between legacy plans and active specs.

## Scope

This audit checks two migration sources:

1. Legacy planning tree:
   - `plan/phase/*`
   - `plan/ops/*`
   - `plan/verify/*`
   - `plan/INDEX.md`
   - `plan/ROADMAP.md`
2. Monolithic design:
   - `plan.md`

## Canonical Ownership Model

- **Authoritative docs**:
  - `specs/meta/design.md`
  - `specs/meta/roadmap.md`
  - `specs/meta/execution-order.md`
  - `specs/meta/repo-maintenance.md`
  - `specs/meta/testing-strategy.md`
  - `specs/meta/benchmark-targets.md`
  - `specs/00x-*/` implementation specs
- **Legacy docs (removed from repo after migration)**:
  - `plan.md` — migrated and deleted
  - `plan/` — migrated and deleted

## File-Level Coverage Matrix

| Legacy File | Migrated To | Coverage | Notes |
|---|---|---|---|
| `plan/phase/00-bootstrap.md` | `specs/001-core-mvp/{spec.md,plan.md,tasks.md}` | Full | Merged with Phase 1 into spec `001` |
| `plan/phase/01-core-mvp.md` | `specs/001-core-mvp/{spec.md,plan.md,tasks.md}` | Full | Core MVP baseline preserved |
| `plan/phase/01.1-agent-protocol.md` | `specs/002-agent-protocol/{spec.md,plan.md,tasks.md}` | Full | Boundary clarification preserved |
| `plan/phase/01.5-structure-workspace.md` | `specs/003-structure-nav/*` + `specs/004-workspace-transport/*` | Full (Split) | Phase `1.5` split into `1.5a` + `1.5b` |
| `plan/phase/02-vcs-ga.md` | `specs/005-vcs-core/*` + `specs/006-vcs-ga-tooling/*` | Full (Split) | Phase `2` split into `2a` + `2b` |
| `plan/phase/02.5-call-graph.md` | `specs/007-call-graph/*` | Full | Number shifted by +1 after VCS split |
| `plan/phase/03-semantic-hybrid.md` | `specs/008-semantic-hybrid/*` | Full | Number shifted by +1 after VCS split |
| `plan/phase/04-distribution.md` | `specs/009-distribution/*` | Full | Number shifted by +1 after VCS split |
| `plan/ops/ci-security.md` | `specs/meta/repo-maintenance.md` §1-2 | Full | CI and security baseline retained |
| `plan/ops/repo-governance.md` | `specs/meta/repo-maintenance.md` §3 | Full | Governance workflow retained |
| `plan/ops/release-pipeline.md` | `specs/meta/repo-maintenance.md` §4 | Full | Release lifecycle retained |
| `plan/ops/maintenance-automation.md` | `specs/meta/repo-maintenance.md` §5 | Full | Scheduler/dependency automation retained |
| `plan/ops/cicd-coverage-matrix.md` | `specs/meta/repo-maintenance.md` §6 | Full | Coverage matrix retained |
| `plan/ops/cicd-brainstorm.md` | `specs/meta/repo-maintenance.md` §7 | Full | Decision archive retained |
| `plan/verify/testing-strategy.md` | `specs/meta/testing-strategy.md` | Full | Cross-spec strategy retained |
| `plan/verify/benchmark-targets.md` | `specs/meta/benchmark-targets.md` | Full | Quant targets retained |
| `plan/INDEX.md` | `specs/meta/INDEX.md` | Full | Canonical index moved to specs |
| `plan/ROADMAP.md` | `specs/meta/roadmap.md` + `specs/meta/execution-order.md` | Full | Roadmap + sequencing split |

## `plan.md` Section Coverage Matrix

| `plan.md` Section | Migrated To | Coverage |
|---|---|---|
| 1. Executive Decision | `specs/meta/design.md` §1 | Full |
| 2. Research Findings | `specs/meta/design.md` §2 | Full |
| 3. Product Vision | `specs/meta/design.md` §3 | Full |
| 4. Product Principles | `specs/meta/design.md` §4 | Full |
| 5. Scope and Non-goals | `specs/meta/design.md` §5 | Full |
| 5.1 VCS Mandatory Capability | `specs/meta/design.md` §5.1 + `specs/005`/`006` | Full |
| 6. Rust-first Architecture | `specs/meta/design.md` §6 | Full |
| 7. Retrieval and Ranking Strategy | `specs/meta/design.md` §7 | Full |
| 8. Feature Backlog and Algorithms | `specs/meta/design.md` §8 + `specs/meta/roadmap.md` backlog | Full |
| 9. Index Schema | `specs/meta/design.md` §9 + spec data-model docs | Full |
| 10. MCP Tool Surface | `specs/meta/design.md` §10 + per-spec contracts | Full |
| 11. CLI UX | `specs/meta/design.md` §11 | Full |
| 12. Competitive Landscape | `specs/meta/design.md` §12 | Full |
| 13. Phased Delivery Plan | `specs/meta/roadmap.md` + `specs/meta/execution-order.md` | Full |
| 14. Packaging and Distribution | `specs/009-distribution/*` + `specs/meta/repo-maintenance.md` §4 | Full |
| 15. Testing and Benchmark Plan | `specs/meta/testing-strategy.md` + `specs/meta/benchmark-targets.md` | Full |
| 16. Targets (Draft) | `specs/meta/benchmark-targets.md` | Full |
| 17. Risks and Mitigations | `specs/meta/design.md` §13 (Risk Register) | Full (Renamed) |
| 18. Security Model | `specs/meta/design.md` §14 | Full |
| 19. Schema Versioning | `specs/meta/design.md` §15 | Full |
| 20. Open Questions | `specs/meta/design.md` §16 + `specs/meta/roadmap.md` | Full |
| 21. Immediate Next Steps | `specs/meta/execution-order.md` + task plans in `specs/00x-*/tasks.md` | Full (Operationalized) |

## Intentional Structural Deltas

These are deliberate optimizations, not migration gaps:

1. **VCS split**:
   - `plan` Phase `2` became:
     - `005-vcs-core` (correctness first)
     - `006-vcs-ga-tooling` (GA tool surface)
2. **Phase 1.5 split**:
   - `plan` Phase `1.5` became:
     - `003-structure-nav`
     - `004-workspace-transport`
3. **Cross-cutting extraction**:
   - Ops/verify materials consolidated under `specs/meta/*` to reduce duplication and improve discoverability.

## Completeness Verdict

- Legacy source files reviewed: **18 / 18**
- `plan.md` H2 sections mapped: **21 / 21**
- Uncovered legacy source files: **0**
- Unmapped `plan.md` H2 sections: **0**

Migration is **complete**, with only intentional structural refactors listed above.
