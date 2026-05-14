# AgentHub Local

A local macOS desktop application for managing Claude Code MCP servers, skills, sub-agents and related developer resources. Built on **Tauri 2 + React + TypeScript + Vite + Rust + SQLite**.

> Status: Phase 0 — project skeleton only. No business features are implemented yet.

## Requirements

- macOS 13 or later
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/) 10+
- [Rust](https://www.rust-lang.org/tools/install) 1.77+ (stable toolchain)
- Xcode Command Line Tools (`xcode-select --install`)

## Getting started

```bash
pnpm install
pnpm tauri:dev
```

The first `tauri:dev` run will compile the Rust side and may take several minutes. Subsequent runs are incremental.

## Available scripts

| Command            | Description                                  |
| ------------------ | -------------------------------------------- |
| `pnpm dev`         | Run the Vite dev server only (web).          |
| `pnpm build`       | Type-check and build the frontend bundle.    |
| `pnpm typecheck`   | TypeScript type-check (no emit).             |
| `pnpm lint`        | Run ESLint over the frontend.                |
| `pnpm format`      | Format files with Prettier.                  |
| `pnpm test`        | Run unit tests with Vitest.                  |
| `pnpm tauri:dev`   | Launch the Tauri desktop app in dev mode.    |
| `pnpm tauri:build` | Produce a release bundle of the desktop app. |

For Rust-side checks:

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features
```

## Project structure

```
.
├── index.html                  # Vite entry HTML
├── src/                        # React + TypeScript frontend
│   ├── main.tsx                # React bootstrap
│   ├── App.tsx                 # Routes
│   ├── layout/                 # Shell layout (sidebar, top bar)
│   ├── pages/                  # Route pages (currently placeholders)
│   └── styles/                 # Global CSS variables
├── src-tauri/                  # Rust / Tauri 2 backend
│   ├── src/lib.rs              # Tauri builder + commands
│   ├── src/main.rs             # Binary entry
│   ├── tauri.conf.json         # Tauri app configuration
│   └── capabilities/           # Permission capabilities
├── docs/qa/
│   └── development-checklist.md # Pre-commit and per-task checklist
└── package.json
```

## Conventions

- 2-space indentation in TypeScript/CSS, 4 spaces in Rust.
- `camelCase` for variables, `UPPER_SNAKE_CASE` for constants.
- All code comments in English.
- The app never spawns MCP servers, Pi extensions, packages, or arbitrary user-project code.
- All configuration writes go through `ChangeService` (introduced in a later phase). Adapters never write files directly.

## License

Private / internal.
