## Why

The six retrieval/ranking changes were merged successfully, but follow-up hardening found a few gaps between intended guardrails and production defaults. In particular, the retrieval eval workflow still ran in dry-run mode in CI, and several post-merge polish issues (warning noise and archived spec metadata placeholders) reduced maintainability.

## What Changes

1. **Make retrieval eval gate blocking in CI**
   - Remove dry-run execution from the CI retrieval gate step so gate failures fail the workflow.
2. **Reduce post-merge warning noise**
   - Mark test-only fanout helper as test-only to eliminate dead-code warnings in normal builds.
3. **Tighten floating-point tolerance semantics**
   - Replace ultra-strict `f64::EPSILON` checks in ranking/explain paths with explicit, domain-appropriate tolerance constants.
4. **Complete archived spec Purpose sections**
   - Replace archive placeholders with concrete Purpose statements for the six newly archived capabilities.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `retrieval-eval-gate`: CI integration requirement is tightened so retrieval gate must run in enforcement mode (non-dry-run) when the gate job is triggered.

## Impact

- Affected files:
  - `.github/workflows/ci.yml`
  - `crates/cruxe-query/src/search.rs`
  - `crates/cruxe-query/src/explain_ranking.rs`
  - `openspec/specs/{policy-aware-retrieval,ranking-signal-budget-contract,edge-confidence-weighting,adaptive-query-plan,retrieval-eval-gate,context-pack-builder}/spec.md`
- Runtime/API impact:
  - No protocol/schema break.
  - CI behavior becomes stricter (intended).
- Data impact: none.
