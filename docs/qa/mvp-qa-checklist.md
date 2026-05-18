# MVP QA Checklist

This checklist is the Phase 10 release gate for a self-usable AgentHub Local MVP.

## Environment

- Date: 2026-05-18
- Platform: macOS local desktop app
- Build target: Tauri 2 release bundle
- Data source rule: filesystem agent configs remain the source of truth; SQLite is an index/history store.

## Core Flow Checklist

| Area | Scenario | Expected result | Status |
| --- | --- | --- | --- |
| First launch | Start the built app bundle | App creates `library/`, `backups/`, `logs/`, `cache/scans/`, and `agenthub.sqlite3` without broad OS prompts | Passed on existing local app-data root |
| Add project | Add an existing local project path | Project appears in Projects and path is registered for guarded writes | Pending manual app launch |
| Scan | Rescan a project | UI shows scan progress, adapter failures are isolated, retry remains available | Pending manual app launch |
| Matrix | Open project detail after scan | MCP, Skills, Sub-agent, and Pi matrices render from indexed resources | Pending manual app launch |
| Doctor | Run Doctor checks | Issues are listed by severity without blocking unrelated checks | Pending manual app launch |
| MCP write | Create, preview, confirm, and apply an MCP change | Diff is shown before write; ChangeService records backup and change status | Pending fixture/manual write test |
| Backup | Open Backups after an applied change | Backup record points to manifest and file snapshot | Pending fixture/manual write test |
| Restore | Restore an applied backup | Original file content is restored and change history records restore state | Pending fixture/manual write test |
| Skill enable | Enable a library skill for a supported agent | Change plan routes through ChangeService and never writes directly from adapters | Pending fixture/manual write test |
| Sub-agent enable | Enable a library sub-agent for Claude/Codex | Change plan routes through ChangeService with diff preview | Pending fixture/manual write test |
| Pi resource path | Update Pi resource path | Diff preview is generated; extension/package execution is never triggered | Pending fixture/manual write test |
| Prompt copy | Render and copy a prompt with variables | Missing variables are shown inline; rendered output copies without file writes | Pending manual app launch |

## Automated Checks

| Check | Command | Status |
| --- | --- | --- |
| Frontend pagination test | `pnpm test src/lib/pagination.test.ts` | Passed |
| Rust logging redaction test | `cargo test logging --lib` from `src-tauri/` | Passed |
| Frontend typecheck | `pnpm typecheck` | Passed |
| Frontend lint | `pnpm lint` | Passed |
| Frontend build | `pnpm build` | Passed |
| Rust tests | `cargo test --lib` from `src-tauri/` | Passed |
| Rust formatting | `cargo fmt --check` from `src-tauri/` | Passed |
| Rust clippy | `cargo clippy --all-targets --all-features` from `src-tauri/` | Passed |
| macOS bundle | `pnpm tauri:build` | Passed for local `.app` bundle |

## Known Phase 10 Risks

- Full write-flow end-to-end QA still requires using local fixture or disposable config paths for MCP, Skill, Sub-agent, Backup, and Restore scenarios.
- DMG generation is intentionally not part of the MVP bundle target; `tauri:build` produces a local `.app` bundle for self-use.
- The MVP uses pagination instead of full virtual scrolling; this satisfies the 1000-resource performance target for table views while keeping implementation scope small.
