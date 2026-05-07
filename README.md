# SubwaiHQ Coding Factory

A local-first control room for running coding agents across your repos. Forked from
[BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban) (rebranded upstream as "GenCap
Control Room", now sunsetting) and rebuilt as an operator's cockpit rather than a hosted
team kanban.

> Status: working personal build. Not production. Not multi-tenant. Not stable.

## What this fork is for

One place to plan, launch, supervise, review, and merge multiple coding agents working
in parallel across local repositories. The unit of work is a **workspace** — one
disposable git worktree pointed at one agent (Claude Code, Codex, Gemini CLI, Amp,
Copilot, Cursor, OpenCode, Droid, Qwen). Every workspace is a real branch, real files,
real dev server.

The wedge is honest: this is the bench you sit at when you're orchestrating five agents
at once and need to see what's stuck, what's done, and what wants your attention.

## What changed vs upstream

The codebase keeps upstream's React + axum + SQLite skeleton, plus:

- **`gencap-mcp` server** — `crates/mcp` exposes 40+ tools over MCP stdio (workspaces,
  sessions, issues, work items, mail, artifacts, friction events). Two modes: `global`
  (full surface) and `orchestrator` (scoped to one workspace). Register it with Claude
  CLI and drive the cockpit from chat.
- **Friction-log discipline** — `services/friction_emitter` writes structured JSONL
  events at 8 instrumented chokepoints (workspace.create, dispatch, supervisor.crash,
  app.boot, …). Used to decide what to fix vs route around. Kill switch:
  `GENCAP_FRICTION_ENABLED=0`.
- **`/friction` dashboard** — `pages/friction/FrictionPage.tsx`. Per-venture chokepoint
  rollup with sparkline trends; reads `/api/friction/snapshot`.
- **`workspace.venture` column** — every workspace is tagged
  `chief-of-staff | carbonv3 | fultech | port-analytics | other` for cross-repo
  slicing. Surfaces in the create form and in friction aggregation.
- **`gencap` CLI** — `scripts/gencap.sh` with `log` / `status` / `extend` / `teardown`
  subcommands plus a day-3 summarizer that auto-writes
  `docs/designs/phase-14-candidates.md` from captured friction events.
- **List ⇄ Board toggle on `/workspaces`** — kanban columns over the existing derived
  status buckets (Needs me / Running / Errored / Recently completed / Idle). Persists
  in localStorage. Not drag-and-drop — workspace status is derived from session +
  process state, not a writable field.

## Architecture

```
crates/
  server/         axum HTTP API + websocket streams
  db/             SQLx models + migrations (SQLite)
  executors/      coding-agent runners (Claude Code, Codex, Gemini, Amp, …)
  services/       container, friction_emitter, mail, file_search
  git/            worktree + branch + remote operations
  mcp/            gencap-mcp binary (rmcp / stdio)
  api-types/      shared TS-rs types (local + remote)
  remote/         optional cloud surface (ElectricSQL, postgres)
  deployment/     trait abstracting local vs remote deployment
  local-deployment/   the binary that ships
packages/
  local-web/      Vite shell for local dev
  remote-web/     Vite shell for hosted deployment
  web-core/       shared React + TS frontend (TanStack Router)
shared/           generated TS types from Rust (do not hand-edit)
scripts/          dev helpers + gencap CLI
docs/             Mintlify docs + design notes
```

Generated types: `pnpm run generate-types` regenerates `shared/types.ts` from
`crates/server/src/bin/generate_types.rs`. For remote/cloud:
`pnpm run remote:generate-types`.

Per-area guides:
- [`crates/remote/AGENTS.md`](crates/remote/AGENTS.md) — remote server, ElectricSQL
- [`docs/AGENTS.md`](docs/AGENTS.md) — Mintlify docs
- [`packages/local-web/AGENTS.md`](packages/local-web/AGENTS.md) — frontend styling

## Quick start

Prereqs: Node 20, pnpm 10, Rust nightly (`rust-toolchain.toml` pins it), `gh` for PRs.

```bash
pnpm i
pnpm run dev          # frontend + backend, ports auto-assigned
```

The dev script picks free ports and writes a port file at
`$TMPDIR/gencap/gencap.port`. Open the printed URL.

App data lives at `~/Library/Application Support/ai.bloop.vibe-kanban/db.v2.sqlite`.

### Wire up the MCP for Claude CLI

```bash
cargo build --release -p mcp --bin gencap-mcp
claude mcp add -s user gencap -- "$PWD/target/release/gencap-mcp" --mode global
```

Restart Claude Code, then `mcp__gencap__list_workspaces` should appear.

## Common commands

| Command | What it does |
|---|---|
| `pnpm run dev` | Frontend + backend, watch mode |
| `pnpm run backend:dev:watch` | Backend only, cargo-watch |
| `pnpm run local-web:dev` | Frontend only |
| `pnpm run check` | Type-check (frontend + all Rust crates) |
| `pnpm run lint` | ESLint + clippy |
| `pnpm run format` | Prettier + cargo fmt across all workspaces |
| `pnpm run generate-types` | Regenerate `shared/types.ts` |
| `pnpm run prepare-db` | Refresh SQLx offline metadata |
| `cargo test --workspace` | Rust tests |
| `scripts/gencap.sh status` | Friction-log experiment state |
| `scripts/gencap.sh log` | Capture a friction event |

Always run `pnpm run format` before opening a PR.

## Coding standards

- **Rust** — `rustfmt`, snake_case modules, PascalCase types, group imports by crate,
  small accept-interface-return-struct functions.
- **TypeScript** — ESLint + Prettier (2 spaces, single quotes, 80 cols), PascalCase
  components, kebab-case filenames where practical.
- **Tests** — unit tests adjacent to code (`#[cfg(test)]`); add tests for edge cases.
  Frontend changes must keep `pnpm run check` and `pnpm run lint` green.

## Populating demo state

After `pnpm run dev` the app is empty. To get the screenshots:

1. Open the UI, accept the disclaimer, set workspace dir + branch prefix.
2. Add repos via Settings → Repos (or `POST /api/repos`).
3. From Claude CLI (with the MCP registered):
   - `mcp__gencap__list_workspaces`
   - `mcp__gencap__start_workspace` with `name`, `executor`, `repositories`.

## Security & config

- `.env` for local overrides; never commit secrets.
- Key envs: `FRONTEND_PORT`, `BACKEND_PORT`, `HOST`, `GENCAP_FRICTION_ENABLED`.
- Dev ports + assets managed by `scripts/setup-dev-environment.js`.

## Heritage

Upstream lineage and full git history are preserved. Original upstream:
[BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban). The hosted vibekanban.com
service is sunsetting; this fork is local-only by default and indifferent to that.
