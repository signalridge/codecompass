## ADDED Requirements

### Requirement: Retrieval eval gate CI execution MUST be enforcement mode
When retrieval-eval-gate CI checks are triggered, the workflow MUST execute the retrieval gate command without dry-run semantics so regression verdicts can fail the job.

#### Scenario: CI retrieval gate invocation is non-dry-run
- **WHEN** retrieval-related paths trigger the `retrieval-eval-gate` workflow job
- **THEN** CI MUST invoke the gate script without `--dry-run`
- **AND** a failing gate verdict MUST fail the workflow step

#### Scenario: Gate report remains available for triage
- **WHEN** the retrieval gate job runs in CI
- **THEN** the workflow MUST upload the generated gate report artifact for analysis
- **AND** this reporting behavior MUST NOT weaken fail-fast enforcement semantics
