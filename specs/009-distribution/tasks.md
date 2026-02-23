# Tasks: Distribution & Release

**Input**: Design documents from `/specs/009-distribution/`
**Prerequisites**: plan.md (required), spec.md (required), contracts/mcp-distribution.md (required)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US5)
- Include exact file paths in descriptions

## Phase 1: Build Pipeline (cargo-dist + cross)

**Purpose**: Cross-platform binary builds and release workflow

- [ ] T412 [US1] Initialize cargo-dist configuration: run `cargo dist init` in workspace root, configure target triples (aarch64-apple-darwin, x86_64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, x86_64-pc-windows-msvc), output `dist.toml`
- [ ] T413 [US1] Configure static linking in `dist.toml`: ensure Linux builds use musl for static linking, macOS uses default (dynamic libc is fine), Windows uses MSVC
- [ ] T414 [US1] Set up cross-compilation config if needed: create `Cross.toml` for Linux aarch64 cross-compilation from x86_64 CI runners
- [ ] T415 [US1] Install and configure git-cliff for changelog generation: create `cliff.toml` with conventional commit grouping (feat, fix, refactor, docs, test, chore), PR link templates, version header format
- [ ] T416 [US1] Create GitHub Actions release workflow in `.github/workflows/release.yml`: trigger on tag push (`v*`), build all 5 target binaries via cargo-dist, generate changelog via git-cliff, create GitHub release with binaries + checksums + changelog
- [ ] T417 [P] [US1] Create GitHub Actions CI workflow in `.github/workflows/ci.yml`: trigger on push/PR, run `cargo test --workspace`, `cargo clippy`, `cargo fmt --check`, build on Linux x86_64 + macOS arm64
- [ ] T418 [US1] Test release workflow: create a test tag, verify all 5 binaries are built, checksums are generated, changelog is correct
- [ ] T419 [P] [US1] Verify static linking: download Linux binary on a minimal container (alpine), run `codecompass --version`, verify no missing shared libraries (check with `ldd`)

**Checkpoint**: Tag push produces GitHub release with 5 platform binaries + checksums + changelog

---

## Phase 2: Homebrew Tap

**Purpose**: Homebrew distribution for macOS (and Linux Homebrew) users

- [ ] T420 [US2] Create Homebrew tap repository: `signalridge/homebrew-tap` on GitHub with initial README
- [ ] T421 [US2] Write Homebrew formula in `Formula/codecompass.rb`: platform detection (arm64 vs x86_64 for macOS, x86_64 for Linux), download URLs pointing to GitHub release assets, SHA-256 checksums, `test` block running `codecompass --version`
- [ ] T422 [US2] Create Homebrew auto-update workflow in `.github/workflows/homebrew-update.yml`: trigger on GitHub release published event (via repository_dispatch or workflow_dispatch from release repo), update formula with new version + checksums + URLs
- [ ] T423 [US2] Test Homebrew formula: run `brew install --build-from-source` locally, verify `codecompass --version` and `codecompass doctor` succeed
- [ ] T424 [US2] Run `brew audit --strict Formula/codecompass.rb` and fix any issues

**Checkpoint**: `brew install signalridge/tap/codecompass` works, auto-updates on release

---

## Phase 3: MCP Configuration Templates

**Purpose**: Ready-to-use config templates for AI coding agents

- [ ] T425 [P] [US3] Create Claude Code MCP config template in `configs/mcp/claude-code.json`: `mcp_servers` format with `codecompass serve-mcp` command, workspace argument, environment variables
- [ ] T426 [P] [US3] Create Cursor MCP config template in `configs/mcp/cursor.json`: Cursor's MCP configuration format with tool server entry
- [ ] T427 [P] [US3] Create Codex MCP config template in `configs/mcp/codex.json`: Codex MCP configuration format
- [ ] T428 [P] [US3] Create generic MCP config template in `configs/mcp/generic.json`: universal MCP server configuration with comments explaining each field
- [ ] T429 [US3] Generate JSON schema for all MCP tool definitions in `configs/mcp/tool-schemas.json`: extract from MCP server `tools/list` response, validate against MCP specification
- [ ] T430 [US3] Write human-readable MCP tool reference in `docs/reference/mcp-tools-schema.md`: all tools with input/output schemas, descriptions, examples

**Checkpoint**: Config templates work when pasted into each agent's configuration

---

## Phase 4: Agent Integration Guides

**Purpose**: Step-by-step setup guides for each supported AI coding agent

- [ ] T431 [P] [US4] Write Claude Code integration guide in `docs/guides/claude-code.md`: prerequisites, installation, MCP config, first indexing, example usage, recommended prompt rule (`"use CodeCompass tools before file reads"`), troubleshooting (tools not showing, stale index, permissions)
- [ ] T432 [P] [US4] Write Cursor integration guide in `docs/guides/cursor.md`: prerequisites, installation, MCP config in Cursor settings, example usage, troubleshooting
- [ ] T433 [P] [US4] Write Copilot integration guide in `docs/guides/copilot.md`: status of MCP support, placeholder for when available, alternative usage via CLI
- [ ] T434 [P] [US4] Write Codex integration guide in `docs/guides/codex.md`: prerequisites, installation, MCP config, example usage, recommended workflow
- [ ] T435 [US4] Write auto-indexing setup guide in `docs/guides/auto-indexing.md`: git hook installation, project-type templates, IDE integration suggestions, troubleshooting

**Checkpoint**: Each guide enables a new user to go from zero to working in 10 minutes

---

## Phase 5: Auto-Indexing Templates

**Purpose**: Reference configurations for automatic indexing in common project types

- [ ] T436 [P] [US5] Create Rust auto-indexing template in `configs/templates/rust/`: `.codecompassignore` (ignore `target/`, `*.o`, `*.a`, `*.so`, `*.dylib`, `*.rlib`), `hooks/post-commit` (run `codecompass sync --workspace .`), `hooks/pre-push` (run `codecompass doctor`)
- [ ] T437 [P] [US5] Create TypeScript auto-indexing template in `configs/templates/typescript/`: `.codecompassignore` (ignore `node_modules/`, `dist/`, `build/`, `.next/`, `*.min.js`, `*.min.css`, `coverage/`), `hooks/post-commit`, `hooks/pre-push`
- [ ] T438 [P] [US5] Create Python auto-indexing template in `configs/templates/python/`: `.codecompassignore` (ignore `__pycache__/`, `.venv/`, `venv/`, `.tox/`, `*.pyc`, `*.pyo`, `.eggs/`, `*.egg-info/`, `dist/`, `build/`), `hooks/post-commit`, `hooks/pre-push`
- [ ] T439 [P] [US5] Create Go auto-indexing template in `configs/templates/go/`: `.codecompassignore` (ignore `vendor/` if not vendored, `*.test`, `*.pb.go`, `*_generated.go`), `hooks/post-commit`, `hooks/pre-push`
- [ ] T440 [P] [US5] Create monorepo auto-indexing template in `configs/templates/monorepo/`: `.codecompassignore` (combined patterns for multi-language), `hooks/post-commit`, `hooks/pre-push`
- [ ] T441 [US5] Add error handling to all git hook templates: log failures to `~/.codecompass/logs/hook.log`, exit 0 on failure (hooks should not block git operations)
- [ ] T442 [US5] Write test: install hook in a test repo, make a commit, verify `codecompass sync` was called (mock or check log)

**Checkpoint**: Template git hooks auto-sync the index on commit

---

## Phase 6: Polish & Validation

**Purpose**: End-to-end validation of the full distribution pipeline

- [ ] T443 End-to-end test on macOS arm64: download release binary, init + index + search + serve-mcp, verify all works
- [ ] T444 [P] End-to-end test on Linux x86_64: download release binary in Docker (ubuntu:latest), verify init + index + search
- [ ] T445 [P] End-to-end test on Windows x86_64: download release binary, verify init + index + search (if Windows CI available)
- [ ] T446 Verify Homebrew formula: `brew install signalridge/tap/codecompass && codecompass doctor`
- [ ] T447 [P] Validate all MCP config templates: copy each template into the respective agent's config, verify tools are listed
- [ ] T448 [P] Proofread all integration guides: check for broken links, outdated commands, missing steps
- [ ] T449 Validate `configs/mcp/tool-schemas.json` against MCP specification
- [ ] T450 [P] Verify changelog generation: create 10 conventional commits (mix of feat, fix, docs), tag, verify changelog groups them correctly

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1** (Build Pipeline): No dependencies - can start immediately
- **Phase 2** (Homebrew): Depends on Phase 1 (release binaries must exist)
- **Phase 3** (MCP Config): Independent of Phases 1-2 (templates are static files)
- **Phase 4** (Integration Guides): Independent of Phases 1-2, but benefits from Phase 3 (references config templates)
- **Phase 5** (Auto-Indexing): Independent of all other phases
- **Phase 6** (Validation): Depends on all phases

### Parallel Opportunities

- Phase 1: T417 and T419 can run in parallel with release workflow development
- Phase 3: All config templates (T425-T428) can be created in parallel
- Phase 4: All integration guides (T431-T434) can be written in parallel
- Phase 5: All project-type templates (T436-T440) can be created in parallel
- Phase 6: T444, T445, T447, T448, T450 can run in parallel
- Phases 3, 4, 5 can all be developed in parallel (independent content)

## Implementation Strategy

### Incremental Delivery

1. Phase 1 -> Release pipeline works (binary distribution available)
2. Phase 2 -> Homebrew tap works (easiest install path)
3. Phase 3 -> MCP config templates ready (agent configuration enabled)
4. Phase 4 -> Integration guides ready (user onboarding complete)
5. Phase 5 -> Auto-indexing templates ready (power user workflow)
6. Phase 6 -> End-to-end validation

### Parallel Work Streams

Three independent work streams can proceed simultaneously:
- **Stream A**: Build pipeline + Homebrew (Phases 1-2)
- **Stream B**: MCP templates + Integration guides (Phases 3-4)
- **Stream C**: Auto-indexing templates (Phase 5)

## Notes

- Total: 39 tasks, 6 phases
- No new Rust code in `crates/` -- this is entirely distribution, documentation, and configuration
- The Homebrew tap is a separate repository (`signalridge/homebrew-tap`)
- Phases 3, 4, 5 can be done entirely in parallel with the build pipeline
- Git hook templates must exit 0 on failure to avoid blocking developer workflow
