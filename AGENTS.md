# AGENTS.md

Operational guide for autonomous agents working in this repository.
`CLAUDE.md` is the deep operational manual (Railway services, trader
config invariants, fee model, on-chain invariants) — read it before any
non-trivial change. This file holds the quick-reference essentials.

## What this repo is

RTP (Resilient Token Protocol) — a high-value bespoke treasury service.
Pipeline: client mandate → research wing manufactures strategy → fixed
gate suite qualifies it → deploys on self-custodied, on-chain-verifiable
rails. License: BSL 1.1. `SOULCONTRACT.md` is constitutional governance
and overrides everything.

## Repository layout

| Path | Purpose |
|---|---|
| `dashboard/` | Next.js 16 product site + intake API (Railway, always-on) |
| `rtp/swarm/` | Rust trading swarm: `rtp-trader` (live GMTrade SOL perps), `rtp-daemon` |
| `rtp/programs/rtp-treasury/` | Anchor treasury program (reference) |
| `research/` | Python Night Shift factory: WFA, full-sim validation |
| `scripts/` | Operator helpers + on-chain crons (fee crank, promote-strategy) |
| `sdk/` | `@resilient-protocol/sdk` — shared TS SDK (library, imported by dashboard) |
| `cli/` | Flash-era legacy (frozen — no Dependabot updates). Only live piece: `flash-sdk-wrapper.mjs` baked into `Dockerfile.trader` as a fallback path |
| `data/ohlcv/`, `data/night_results/` | Night Shift input/output data |
| `archive/` | Historical eras — read-only context, never build against it |

## Setup (fresh clone → running)

Node toolchain: **Node 22 LTS + npm 10.x** (matches `Dockerfile.dashboard`).
The repo carries a `.nvmrc` (`22`); with nvm installed, `cd` into the repo
auto-switches. Lockfiles must only be touched with this toolchain — an
npm 9 lockfile drops the `../sdk` workspace entry and breaks Railway builds
(see CLAUDE.md → "Intentional dependency pins").

```bash
# Dashboard (http://127.0.0.1:3000)
cd dashboard && npm ci --legacy-peer-deps && npm run dev

# Rust swarm tests
cd rtp/swarm && cargo test --lib

# Night Shift (dry run; --skip-fetch uses data/ohlcv)
python -m research.orchestration.night_shift --skip-fetch

# Anchor program (requires anchor CLI)
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet
```

## Build & test commands by app

| App | Build | Test | Lint/format |
|---|---|---|---|
| dashboard | `cd dashboard && npm run build` | — | `cd dashboard && npm run lint` |
| rtp/swarm | `cd rtp/swarm && cargo build --release` | `cargo test --lib` | `cargo fmt --check && cargo clippy -- -D warnings` |
| rtp/programs | `cd rtp/programs/rtp-treasury && anchor build` | `anchor test` (devnet) | — |
| research | — | `pytest research/` | — |
| scripts | `cd scripts && npx tsx <file>` | — | — |

CI (`.github/workflows/swarm-ci.yml`) runs the swarm build/test/clippy/fmt
plus coverage (`cargo llvm-cov`, fail-under threshold) on every PR and
push to `main` touching `rtp/**` or `sdk/**`.

## Environment variables

Required env vars are documented per service in `railway.toml` comments
and `CLAUDE.md` (Railway section). Key ones:
`RTP_OPERATOR_API_SECRET` (dashboard + trader, must match),
`RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE` / `RTP_TRADER_MIN_ALIGNMENT_OVERRIDE`
(loosening-only), `RTP_TRADER_POSITION_FRACTION`,
`LLM_API_BASE_URL` / `LLM_API_KEY` / `LLM_MODEL` (devnet-loop),
`RTP_INTAKE_DB_PATH` (dashboard). Local template: `dashboard/env.example`.
Secrets live in Railway env vars or `.secrets/` (gitignored) — never
commit them. Operational (not secret):
`RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS` (fee-sane entry floor,
default 500M = 0.5 SOL — the venue's $1 minimum does not cover fixed
per-order costs), `RTP_GM_EXECUTION_FEE_LAMPORTS` (keeper fee, default
500k).

## Naming conventions

- **Rust** (`rtp/swarm`, `rtp/programs`): `snake_case` functions/vars,
  `CamelCase` types, `SCREAMING_SNAKE_CASE` consts — enforced by
  `cargo clippy -- -D warnings` in CI.
- **TypeScript/React** (`dashboard`, `sdk`, `scripts`): `camelCase`
  functions/vars, `PascalCase` components/types, enforced by ESLint.
- **Python** (`research`): PEP 8 — `snake_case` functions/vars,
  `CamelCase` classes, `UPPER_CASE` constants.
- Log line tags are capitalized bracket prefixes: `[POLL]`, `[ENTRY]`,
  `[GM-OPEN]` — match the existing style when adding log lines.

## Hard rules (from CLAUDE.md — do not violate)

- Never judge a strategy on the v1-era fee model; use `net_pnl_v2`.
- `min_alignment=2` is WFA-validated — never revert to 3 without re-running WFA.
- Validated trader config (trail=1.0, tp=6.0, sl=2.5, hold=96h,
  decay=48h, flip_delay=2h) must not be silently replaced with defaults.
- No `unwrap()` in production Rust paths.
- Keep security headers global and CORS allow-listed (see `SECURITY.md`).
- Never `railway up` — it wipes custom domain registrations.
- No PII in log lines.
- Respect the intentional dependency pins: `solana-sdk 2.1` (gmsol-sdk
  unification), `bincode 1`, CI toolchain `1.94.0`, frozen `/cli` —
  see CLAUDE.md → "Intentional dependency pins". Dashboard installs use
  `--legacy-peer-deps` (known anchor/sdk peer conflict).

## Verification commands (run in CI)

The commands below are executed verbatim by the `agents-md-validation`
job in `.github/workflows/swarm-ci.yml`. Keep them runnable.

```bash
cd rtp/swarm && cargo test --lib
```

## Deployment

Railway auto-deploys every push to `main` (see CLAUDE.md → "Railway
Project" for the service table). Operator helpers:
`node scripts/railway-logs.mjs`, `scripts/railway-trader-override.mjs`,
`scripts/railway-redeploy-trader.mjs`, `scripts/check-railway-services.ts`.

Dependabot runs weekly (policy + ignore rules in
`.github/dependabot.yml`). It auto-closes a PR once main already carries
the same version — after applying bumps manually, `@dependabot rebase`
the matching PRs instead of merging them.
