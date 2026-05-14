# Development Checklist

Run through this list before opening a PR or marking a task complete. Items marked **required** must pass; **recommended** items are situational.

## Frontend (TypeScript / React)

- [ ] **required** `pnpm typecheck` passes.
- [ ] **required** `pnpm lint` passes with no errors.
- [ ] **required** `pnpm build` succeeds.
- [ ] **required** `pnpm test` passes (or N/A if no tests touched).
- [ ] **recommended** `pnpm format` applied to changed files.
- [ ] **recommended** Manually navigated through any affected routes in `pnpm tauri:dev`.

## Backend (Rust / Tauri)

- [ ] **required** `cargo fmt --check` (run from `src-tauri/`).
- [ ] **required** `cargo clippy --all-targets --all-features` produces no errors.
- [ ] **recommended** `cargo test` for any module touched.

## Code conventions

- [ ] 2-space indentation in TS/CSS; 4-space in Rust.
- [ ] `camelCase` variables, `UPPER_SNAKE_CASE` constants.
- [ ] Comments are in English.
- [ ] No new dependencies introduced without justification.
- [ ] No direct file writes from adapters — config mutations go through `ChangeService`.
- [ ] The app does not execute MCP servers, Pi extensions, packages, or user-project code.

## PR hygiene

- [ ] Each task’s scope respected — no premature implementation from later phases.
- [ ] Updated docs (`README.md`, `docs/`) when developer-facing behavior changed.
- [ ] Screenshot or screen recording attached for UI changes.
