# CodeCompass -- Project Instructions

## Overview

Rust workspace with 6 crates. Single binary that indexes code repositories and serves symbol location / code search results to AI coding agents via MCP protocol.

Repository: https://github.com/signalridge/codecompass

## Build Commands

```bash
cargo build                              # Debug build
cargo build --release                    # Release build
cargo test --workspace                   # Run all tests
cargo clippy --workspace --all-targets   # Lint
cargo fmt --check --all                  # Format check
```

## Architecture

| Crate | Purpose |
|-------|---------|
| `codecompass-core` | Types, constants, config, error types. All other crates depend on this. |
| `codecompass-state` | SQLite (rusqlite 0.32, WAL mode) + Tantivy 0.22 storage layer. |
| `codecompass-indexer` | tree-sitter 0.24 parsing, per-language symbol/snippet extractors (Rust, TS, Python, Go). |
| `codecompass-query` | Search, locate, intent classification (symbol/path/error/NL), rule-based ranking. |
| `codecompass-mcp` | MCP JSON-RPC server over stdio. Tools: `search_code`, `locate_symbol`, `index_repo`, `sync_repo`, `index_status`. |
| `codecompass-cli` | clap-based CLI entry point. Commands: `init`, `index`, `search`, `doctor`, `serve-mcp`. |

## Key Conventions

### SQL: quote reserved keywords

SQLite reserved keywords `ref` and `commit` MUST be double-quoted in ALL SQL statements:

```sql
-- Correct
SELECT "ref", "commit" FROM file_manifest WHERE "ref" = ?1;

-- Wrong (will fail or produce unexpected results)
SELECT ref, commit FROM file_manifest WHERE ref = ?1;
```

### SymbolKind parsing

Use `SymbolKind::parse_kind()`, not `from_str()`.

### Error types

Each crate has its own error variant in `codecompass_core::error`. Use the crate-level error type, not raw `anyhow` in library code.

### tree-sitter 0.24

Use index-based child iteration (`node.child(i)`, `node.child_count()`) rather than cursor-based traversal where practical.

### Tantivy 0.22

For extracting string values from `OwnedValue`, you must import the `Value` trait:

```rust
use tantivy::schema::Value;
let s = owned_value.as_str();
```

### Rust edition

Edition 2024. This is set in `[workspace.package]` in the root `Cargo.toml`.

## Test Fixtures

Test fixture repositories are located at:

```
testdata/fixtures/rust-sample/
testdata/fixtures/ts-sample/
testdata/fixtures/python-sample/
testdata/fixtures/go-sample/
```

## Project Layout

```
Cargo.toml                    # Workspace root
crates/
  codecompass-cli/            # Binary crate
  codecompass-core/           # Shared types
  codecompass-state/          # Storage (SQLite + Tantivy)
  codecompass-indexer/        # tree-sitter parsing
  codecompass-query/          # Search and ranking
  codecompass-mcp/            # MCP server
configs/
  default.toml                # Default configuration
testdata/
  fixtures/                   # Language-specific test repos
  golden/                     # Expected output snapshots
specs/
  001-core-mvp/               # Feature specification and plan
```
