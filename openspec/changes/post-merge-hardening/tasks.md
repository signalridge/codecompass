## 1. Retrieval gate enforcement in CI

- [x] 1.1 Remove `--dry-run` from retrieval gate workflow step.
- [x] 1.2 Keep retrieval eval report artifact upload intact for diagnostics.

## 2. Query crate post-merge hardening

- [x] 2.1 Mark `semantic_fanout_limits` helper as test-only to remove dead-code warnings.
- [x] 2.2 Replace ranking/explain `f64::EPSILON` comparisons with explicit tolerance constants where semantic tolerance is intended.

## 3. Archived spec metadata quality

- [x] 3.1 Replace placeholder Purpose text in the six newly archived capability specs with concrete capability purpose statements.

## 4. Verification and governance

- [x] 4.1 Run formatting and compile checks for touched crates/workspace scope.
- [x] 4.2 Run targeted retrieval/ranking/context-pack tests.
- [x] 4.3 Validate OpenSpec artifacts for `post-merge-hardening` and strict spec checks.
