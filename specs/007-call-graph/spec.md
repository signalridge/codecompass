# Feature Specification: Call Graph Analysis

**Feature Branch**: `007-call-graph`
**Created**: 2026-02-23
**Status**: Draft
**Phase**: 2.5 | **Version**: v1.1.0
**Depends On**: 006-vcs-ga-tooling
**Input**: User description: "Call edge extraction via tree-sitter, get_call_graph MCP tool, compare_symbol_between_commits tool, suggest_followup_queries tool"

## User Scenarios & Testing

### User Story 1 - Extract Call Edges from Indexed Code (Priority: P1)

A developer has an indexed repository and wants to understand the call relationships
between functions. The system, during indexing, uses tree-sitter to match function
and method call sites in each source file, then creates `edge_type = 'calls'` entries
in the `symbol_edges` table. Direct calls are marked as `static` confidence, while
method calls where the receiver type is ambiguous are marked as `heuristic`. Cross-file
resolution is performed best-effort by qualified name matching against the symbol index.

**Why this priority**: Call edges are the data foundation for the call graph tool
and for cross-reference analysis. Without extracted edges, no graph queries are possible.

**Independent Test**: Index a fixture repository with known call chains, then query
`symbol_edges` for `edge_type = 'calls'` and verify the expected caller-callee pairs
exist with correct confidence levels.

**Acceptance Scenarios**:

1. **Given** a Rust file containing `fn a() { b(); }` and `fn b() {}`, **When** indexing
   completes, **Then** `symbol_edges` contains an entry with `from_symbol = a`,
   `to_symbol = b`, `edge_type = 'calls'`, `confidence = 'static'`.
2. **Given** a Python file containing `obj.process()` where `obj`'s type is not
   statically resolvable, **When** indexing completes, **Then** `symbol_edges` contains
   an entry with `confidence = 'heuristic'` for the method call.
3. **Given** a call from `file_a.rs::handler()` to `file_b.rs::validate()`, **When**
   indexing completes, **Then** cross-file resolution matches the callee by qualified
   name and creates the edge with correct `to_symbol_id`.
4. **Given** a call to a function not in the indexed codebase (e.g., stdlib or external
   crate), **When** indexing completes, **Then** the edge is created with
   `to_symbol_id = NULL` and `to_name = "external::function_name"`.
5. **Given** a method call via trait object `dyn Handler`, **When** indexing completes,
   **Then** the edge is marked `confidence = 'heuristic'` since the concrete
   implementation is unknown at parse time.

---

### User Story 2 - Query the Call Graph for a Symbol (Priority: P1)

A developer or AI agent uses the `get_call_graph` MCP tool to understand what calls
a given symbol and what it calls. The tool returns a graph of callers and callees
up to a configurable depth, scoped to a specific ref. This enables impact analysis
("what would break if I change this function?") and code comprehension ("what does
this function depend on?").

**Why this priority**: This is the primary user-facing value of call graph extraction.
Agents need this to reason about code changes and their impacts.

**Independent Test**: Index a fixture repo, call `get_call_graph` for a known function,
verify callers and callees match expected values at depth 1 and depth 2.

**Acceptance Scenarios**:

1. **Given** an indexed repository, **When** `get_call_graph` is called with
   `symbol_name: "validate_token"`, `direction: "both"`, `depth: 1`, **Then** the
   response includes direct callers and direct callees of `validate_token`.
2. **Given** `depth: 2`, **When** `get_call_graph` is called, **Then** the response
   includes transitive callers/callees up to 2 levels deep.
3. **Given** `direction: "callers"`, **When** `get_call_graph` is called, **Then**
   only callers are returned (no callees).
4. **Given** a symbol that has no call edges, **When** `get_call_graph` is called,
   **Then** the response returns empty `callers` and `callees` arrays with
   `total_edges: 0`.
5. **Given** `limit: 5` with a symbol that has 20 callers, **When** `get_call_graph`
   is called, **Then** only the top 5 callers are returned, with `truncated: true`
   in metadata.

---

### User Story 3 - Compare a Symbol Between Commits (Priority: P2)

A developer or AI agent uses the `compare_symbol_between_commits` MCP tool to see
how a specific symbol changed between two refs (commits, branches, tags). The tool
returns the diff of the symbol's signature, body, and line range, enabling agents to
understand the evolution of a function across branches or over time.

**Why this priority**: Comparing symbol states across refs is essential for code
review assistance and understanding what changed in a PR.

**Independent Test**: Index a fixture repo at two different commits where a known
function changed, call `compare_symbol_between_commits`, verify the diff summary
is accurate.

**Acceptance Scenarios**:

1. **Given** a function `process_request` that exists in both `main` and `feat/auth`,
   **When** `compare_symbol_between_commits` is called with `base_ref: "main"`,
   `head_ref: "feat/auth"`, **Then** the response shows the signature diff, body diff
   summary, and line range changes.
2. **Given** a symbol that was added in `head_ref` but does not exist in `base_ref`,
   **When** `compare_symbol_between_commits` is called, **Then** the response shows
   `base_version: null` and `head_version` with full symbol details.
3. **Given** a symbol that was deleted in `head_ref`, **When**
   `compare_symbol_between_commits` is called, **Then** the response shows
   `head_version: null` and `base_version` with full symbol details.
4. **Given** a symbol that is identical in both refs, **When**
   `compare_symbol_between_commits` is called, **Then** the response shows
   `diff_summary: "unchanged"`.

---

### User Story 4 - Get Suggested Follow-up Queries (Priority: P3)

An AI agent receives search results with low confidence and uses the
`suggest_followup_queries` MCP tool to determine what to try next. The tool analyzes
the previous query and results, then suggests specific tool calls that might yield
better results. This enables agents to self-correct their search strategy.

**Why this priority**: Improves agent effectiveness by providing actionable guidance
when initial search attempts are not productive.

**Independent Test**: Call `suggest_followup_queries` with a low-confidence
`search_code` result, verify the suggestions include concrete tool calls with
parameters.

**Acceptance Scenarios**:

1. **Given** a previous `search_code` query with `natural_language` intent and top
   score < 0.3, **When** `suggest_followup_queries` is called, **Then** suggestions
   include a `locate_symbol` call with extracted identifiers from the query.
2. **Given** a previous `locate_symbol` query that returned 0 results, **When**
   `suggest_followup_queries` is called, **Then** suggestions include a `search_code`
   query with the symbol name and possibly `get_call_graph` if the symbol might be
   a callee.
3. **Given** previous results with confidence above the threshold, **When**
   `suggest_followup_queries` is called, **Then** the response includes an empty
   suggestions array with `reason: "results are above confidence threshold"`.

### Edge Cases

- What happens when a recursive function calls itself?
  A self-referencing edge is created with `from_symbol_id = to_symbol_id`.
- What happens when a call site uses a function pointer or closure?
  The call is recorded with `confidence = 'heuristic'` and `to_name` set to the
  variable name. Resolution to the actual target is not attempted.
- What happens when `get_call_graph` is called with `depth > 5`?
  Depth is capped at 5 to prevent runaway graph traversal. A warning is included
  in the metadata.
- What happens when `compare_symbol_between_commits` references a ref that is not
  indexed?
  An error is returned: `ref_not_indexed` with the unindexed ref name.
- What happens when call edge extraction encounters a syntax error in the source file?
  The file is skipped for call edge extraction (but still indexed for symbols),
  and a warning is logged.

## Requirements

### Functional Requirements

- **FR-601**: System MUST extract call edges from source files using tree-sitter during
  indexing, matching function/method call sites for Rust, TypeScript, Python, and Go.
- **FR-602**: System MUST store call edges in `symbol_edges` table with `edge_type = 'calls'`,
  `from_symbol_id`, `to_symbol_id`, `confidence` (`static` or `heuristic`), and
  `source_location` (file path + line number of the call site).
- **FR-603**: System MUST resolve cross-file call targets best-effort by matching the
  callee name against the qualified names in the symbol index for the same ref.
- **FR-604**: System MUST record unresolvable call targets with `to_symbol_id = NULL` and
  `to_name` set to the best available name from the call site AST node.
- **FR-605**: System MUST provide a `get_call_graph` MCP tool that returns callers and/or
  callees for a given symbol, scoped by ref, with configurable depth (1-5) and limit.
- **FR-606**: System MUST provide a `compare_symbol_between_commits` MCP tool that shows
  the diff of a symbol's signature, body, and line range between two refs.
- **FR-607**: System MUST provide a `suggest_followup_queries` MCP tool that analyzes
  previous query results and suggests next tool calls when confidence is low.
- **FR-608**: System MUST cap call graph traversal depth at 5 and include a warning in
  metadata when the requested depth exceeds this limit.
- **FR-609**: System MUST include Protocol v1 metadata in all new tool responses.
- **FR-610**: System MUST handle recursive calls (self-edges) correctly in graph traversal
  without infinite loops.

### Key Entities

- **CallEdge**: A directed relationship from a caller symbol to a callee symbol, with
  confidence level, source location (call site file + line), and edge type.
- **CallGraph**: A directed graph of symbols connected by call edges, with traversal
  bounded by depth and result count limits.
- **SymbolComparison**: A diff between two versions of the same symbol across different
  refs, including signature, body, and line range changes.
- **FollowupSuggestion**: A recommended tool call with parameters and rationale, generated
  from analysis of previous query results.

## Success Criteria

### Measurable Outcomes

- **SC-601**: Call edge extraction achieves >= 80% precision for direct (static) calls
  on fixture repositories with known call graphs.
- **SC-602**: `get_call_graph` returns results within 500ms p95 for depth <= 2 on a
  repository with 10,000 symbols.
- **SC-603**: `compare_symbol_between_commits` returns accurate diff summaries for
  at least 90% of symbol changes in a fixture PR.
- **SC-604**: `suggest_followup_queries` provides actionable suggestions that, when
  followed, improve result quality in >= 70% of low-confidence scenarios.
