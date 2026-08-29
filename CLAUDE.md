# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**RTP (Resilient Token Protocol)** is a high-value bespoke treasury service.
Each client gets one manufactured strategy — engineered to their mandate
(capital, drawdown, horizon, accumulation target), validated against a fixed
ten-gate suite at measured on-chain costs, and deployed on self-custodied,
on-chain-verifiable rails.

**Specimen (proof asset)**: SOL/USDT Survivor 2.69 — runs live on GMTrade
as the blueprint that proves the factory works. Not a product; mass-deploying
it would crowd the trade.

**Pipeline is the product**: client mandate → research wing manufactures
strategy → fixed gate suite qualifies it → it deploys on self-custodied
rails with on-chain enforcement.

**Strategic direction (canonical)**: `docs/STRATEGIC-DIRECTION.md`. Bespoke
first, scale later. Client #1 is a close friend; their mandate is the S15
lineage (`research/missions/s15_final_verdict.md`). Consult that doc
before any product / positioning / copy work.

**License**: BSL 1.1 (converts to Apache 2.0 on 2030-05-11)

---

## What lives in this repo

| Path | Status | Purpose |
|---|---|---|
| `dashboard/` | **Live** | The product — bespoke landing, Compatibility Check, docs |
| `rtp/swarm/` | **Live** | Live trader (`rtp-trader`) + daemon + wings |
| `research/` | **Live** | Night Shift factory, WFA, full-sim validation |
| `scripts/railway-*.mjs` | **Live** | Operator helpers (logs, overrides, redeploy) |
| `scripts/redistribute.ts` `promote-strategy.ts` | **Live** | Required by Railway Dockerfiles |
| `sdk/` | **Live** | Solana program SDK (fetchTreasuryState) |
| `cli/flash-sdk-wrapper.mjs` + `cli/package.json` | **Live** | Required by `rtp/swarm/Dockerfile.trader` |
| `data/ohlcv/` | **Live** | Night Shift input data |
| `data/night_results/` (latest 2 days) | **Live** | Baked into promote-strategy image |
| `rtp/programs/` | **Reference** | Anchor program source (proves CPI architecture) |
| `archive/` | **Archived** | Token-onboarding era, Flash-era, hackathon-era. Preserved for portfolio context — `archive/README.md` lists what's there and why |

`SOULCONTRACT.md` is constitutional governance; it overrides everything.

---

## Execution Venue — GMTrade (Solana)

Trader target venue is **GMTrade** (replacing Flash Trade which wound
down Aug 2026 — see `archive/flash-trade/` for the historical venue docs).

```
Night Shift (Python, LIVE)
  └── validated strategy: SOL/USDT Survivor 2.69
        │
        ▼ bridge.rs (DONE)
Trading Wing / rtp-trader (Rust, LIVE)
  └── StrategyParams → entry/exit math → position sizing
        │
        ▼ GMTrade REST API (transaction-builder, live on mainnet)
        │
        ▼ open_position / close_position → SOL perp position
        │
        ▼ SOL returned to trader wallet (HDQ79…) on close
```

**Signing**: There is no human key for treasury/trading. The trading
wallet (`HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5`, keypair at
`~/.config/solana/rtp-trader.json`) is funded and pays gas. The Anchor
program (`rtp-treasury`) uses Treasury PDA `invoke_signed` for on-chain
program interactions.

---

## Trader Operational Notes (lessons learned)

- **Real multi-TF only** — `compute_signal` must receive **independent**
  1h / 4h / 1d close series from Binance (`interval=1h|4h|1d`). Never slice
  a single buffer at multiple lookbacks; the TFs lock together and block
  opposite-side entries. Warmup uses `tokio::join!`; poll line shows
  `1h=N 4h=N 1d=N`.
- **All TF buffers refresh from Binance; in-progress candles are dropped** —
  1h every 1h, 4h every 2h, 1d every 6h (`last_1h_refresh` /
  `last_4h_refresh` / `last_1d_refresh` in `run_cycle`, logs
  `[REFRESH]`). Without the slow-TF refresh, `tf_4h.trend` /
  `tf_1d.trend` compare stale close vs stale SMA and bullish/bearish
  counts never flip. The 1h refresh matters too: tick-built candles
  carry tick-count "volumes" (vol_confirm score term dies) and drift
  from true hourly closes with uptime. `drop_in_progress_candle` strips
  Binance's still-forming last candle on every warmup/refresh — loading
  it as final duplicates the hour when `append_tick` rolls.
- **Reconcile uses venue entry time; phantom clears estimate PnL** —
  `[RECONCILE]` restores orphaned positions with the venue-reported
  `increased_at` (GMTrade) so MaxHold measures the true hold, not a
  synthetic "1h ago". PhantomClear rows book an estimated PnL against
  the current price instead of 0.0 (never counted in `total_pnl_sol`).
- **Stacking defense (Aug 26-27 incident — do not weaken)** — a blind
  duplicate trader process stacked 5-6 entry orders every 5 min while the
  logged instance was flat (up to 3.7× intended size; unmanaged until a
  redeploy reconciled it). Three layers now protect against this:
  (1) **per-poll reconcile** — `reconcile_from_venue()` runs every cycle
  while flat (not just startup), so out-of-process positions become
  visible and managed within one poll;
  (2) **venue stacking guard** — `gmtrade::open_position` refuses with
  `GM_POSITION_ALREADY_OPEN:` when the owner already holds a SOL position
  (or the book cannot be verified — fail closed). Soft-skip, no cooldown;
  (3) the collateral floor still caps runaway stacking at 0.5 SOL orders.
  Do NOT remove the per-poll venue check "for performance" — it is the
  fix for unmanaged $6.8k positions. If you see
  `[ENTRY] Venue position already open` in logs, suspect a duplicate
  process holding the trader keypair — investigate before the next entry.
- **Venue-side protective stops (GMTrade — Aug 28 give-back fix)** — the
  process-side exit checks only see confirmed hourly closes every 5 min,
  so an intrabar crash blows through the trail floor before the next poll
  (Aug 28: +$96 peak gave back to +$1.5). Stops now also live ON THE
  VENUE and execute via keepers when the oracle touches the trigger:
  (1) SL order (`StopLossDecrease`) at entry ∓ sl_atr×ATR, ratcheted to
  the trail floor (peak ∓ trail×ATR, live ATR — exactly mirroring
  `check_exit`) while in profit via owner-signed `update_order` (no
  keeper fee; 0.1%-of-price step filters noise);
  (2) TP order (`LimitDecrease`) at entry ± tp_atr×ATR harvests big wins
  even while the process is down;
  (3) lifecycle — placed immediately after entry + retried per-poll
  (`maintain_venue_stops`), adopted for reconciled orphans, cancelled on
  our closes, flat-swept every poll while flat;
  (4) **per-poll venue existence check while OPEN** (Aug 29 gap) — a
  venue stop firing leaves the local trail/TP conditions unfired for
  hours (closes can stay above the trail floor), so run_cycle verifies
  the tracked position still exists on the venue EVERY poll; when gone,
  `book_vanished_position` books the outcome within one cycle;
  (5) when a venue stop fired, `venue_stop_fill_report` (order-PDA-scoped,
  attribution-checked — same rule as `wait_for_fill`) recovers the actual
  fill price + fees and books a real `StopLoss(Venue)` /
  `TakeProfit(Venue)` row counted in `total_pnl_sol`; no attributable
  fill → `PhantomClear(VenueMissing)` audit row (not counted). Venue
  stops change WHERE execution happens, never the validated levels —
  same ATR multiples as the validated config.
  Kill switch: `RTP_TRADER_VENUE_STOPS=0`. Log tag `[GM-STOP]`.
- **S16: the live multi-TF model has NO validation artifact** (Aug 23) —
  every prior validation (Calmar 44.89, OOS Sharpe 3.96, sensitivity
  CSV, night-shift candidates) used fake multi-TF (lookback 20/80/200
  on one 1h series). The trader runs independent 1h/4h/1d feeds.
  `research/missions/s16_real_tf_revalidation.py` re-validated the REAL
  model over 1y: raw edge ≈ +0.035%/trade @ threshold 0.24, ~0 above;
  no candidate cleared promotion gates at 9× + measured GMTrade fees.
  Do NOT tighten `trailing_stop_atr` based on the old sensitivity CSV —
  it hurts on the real model (0.5 ATR: −453% vs 1.0 baseline). See
  `research/dead_ends.md` (S16 entry).
- **Entry gates on score only, NOT alignment count** — Long = `score >
  threshold`, Short = `score < -threshold`. The alignment count is already
  baked into the score (0.4 × bull/3); an extra `bull_count >=
  min_alignment` gate double-counts and caps the score at 0.267 in
  sideways markets.
- **`min_alignment=2`** — matches the Python reference
  (`research/simulation/run_backtest_r2.py`). The old `min_alignment=3`
  is a stale inheritance from the fake-multi-TF era, **never WFA-validated**,
  absent from `data/sensitivity_sol_survivor_2_69_lev3.csv`. Do NOT
  revert to 3 without re-verifying against the WFA sweep.
- **Trader config loads from validated file** — `rtp/swarm/Dockerfile.trader`
  copies `data/trader-strategy-config.json` into the container and sets
  `RTP_STRATEGY_CONFIG`. If missing/invalid, falls back to hardcoded
  defaults with a warning log. **Validated config: trail=1.0, tp=6.0,
  sl=2.5, hold=96h, decay=48h, flip_delay=2h** — must NOT be silently
  replaced with defaults; check Railway logs for the startup param line.
- **Loosening-only env overrides** —
  `RTP_TRADER_MIN_ALIGNMENT_OVERRIDE` and
  `RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE` relax strict-WFA confluence
  params on Railway without rebuilding. Override values >= configured are
  silently ignored (one-way loosening); missing env = validated config.
  Application logs `[OVERRIDE]` WARN on every applied override. Use
  `node scripts/railway-trader-override.mjs set …` then
  `node scripts/railway-redeploy-trader.mjs` to apply. Tighter values =
  silent no-op (designed to prevent accidental riskier-than-baseline).
- **Score flip delay** — `score_flip_delay_hrs` (default 0.0, set to 2.0
  in validated config) gives a grace period before ScoreFlip exit. Timer
  starts from `first_negative_score_time`, resets when score goes positive.
- **Both sides supported** — Long when score > threshold AND bullish_count
  >= min_alignment; Short when score < -threshold AND bearish_count >=
  min_alignment. Exit math (PnL, trailing, SL/TP) is inverted for Short.
- **`/health` returns 503 when unhealthy** — `consecutive_errors >= 5` OR
  `last_healthy > 30 min`. Not a static "ok".
- **`/state` returns `active_config`** — TraderState includes loaded
  StrategyParams so config drift is visible from the dashboard.
- **Watchdog** — `tokio::time::timeout(120s)` per cycle; exponential
  backoff on repeated failures; 5-min sleep after 10 consecutive.
  All HTTP clients have 30s timeouts.
- **GMTrade position sizing** — keep `RTP_TRADER_POSITION_FRACTION=0.20`
  and a collateral floor (`RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS`,
  default 500M = 0.5 SOL) that keeps the FIXED per-order costs
  (execution fee + wrap ≈ 0.0012 SOL/RT) a small fraction of the
  position. The venue's own $1 minimum only clears keeper validation —
  a drained wallet churning ~$1-collateral positions loses ~4%/leg to
  fixed fees (Aug 2026). Sub-floor sizing soft-skips with entry cooldown.
- **aes-gcm is on 0.11 (aead 0.6)** — `config.rs` migrated 2026-08-17:
  nonce generation is `Nonce::<<Aes256Gcm as AeadCore>::NonceSize>::generate()`
  (no `OsRng`), keys/nonces build via `TryFrom` from byte slices
  (`Array::from_slice` is deprecated), `Aes256Gcm::new(&key)` /
  `.decrypt(&nonce, …)` take references. AES-256-GCM is a standardized
  algorithm so configs encrypted under 0.10 still decrypt. Do NOT
  reintroduce the old `generate_nonce(&mut OsRng)` / `from_slice` calls.

### Intentional dependency pins (do not "fix" without a reason)

- `solana-sdk = "2.1"` in `rtp/swarm` — unifies with gmsol-sdk's internal
  range (`>=2.1,<2.2`). Dependabot major updates ignored.
- `bincode = "1"` — bincode 2/3 is a full API rewrite of
  `serialize`/`deserialize` used by the live trader executor. Majors ignored.
- `dtolnay/rust-toolchain@1.94.0` in CI — matches dev toolchain + Docker
  builders; newer stable fired lints that never appear locally.
- `/cli` is **frozen Flash-era legacy** (Flash Trade wound down Aug 2026;
  GMTrade is the live venue). No Dependabot updates; the only live piece is
  `flash-sdk-wrapper.mjs` in `Dockerfile.trader`'s wrapper stage (fallback
  path). Don't bump its deps.
- Dashboard peer conflict is known and tolerated: `@coral-xyz/anchor ^0.32`
  vs sdk peer `^0.31` — install/build with `npm ci --legacy-peer-deps`
  (same as `Dockerfile.dashboard`).
- **Dashboard lockfile MUST be regenerated with npm 10 (node:22)** — the
  Dockerfile.dashboard toolchain. npm 9 writes `file:../sdk` deps as an
  inline entry and drops the `../sdk` source-package entry, so the build
  fails at `npm ci` with "Missing: @resilient-protocol/sdk@0.1.0 from
  lock file" (Aug 16–23: every dashboard deploy failed from this after a
  batch bump ran under npm 9). Regenerate with
  `docker run --rm -v "$PWD":/repo -w /repo/dashboard node:22 npm install
  --package-lock-only --legacy-peer-deps` — never under a local npm 9.
  Always verify `packages["../sdk"]` exists after regenerating.
- `next` / `react` / `react-dom` / `eslint-config-next` use **exact pins**
  (no `^`) in `dashboard/package.json` — keep that convention.
- Dependabot ignores (`.github/dependabot.yml`): typescript majors (TS7
  native-compiler rewrite), @types/node majors (track Node runtime),
  eslint majors (10 crashes eslint-plugin-react — revisit when
  eslint-config-next supports it).

### Git/Dependabot workflow

- `main` has branch protection (1 approving review) **with admin bypass** —
  direct pushes work for the repo owner/operator.
- Dependabot **auto-closes** a PR once main already contains its exact
  version (diff becomes empty after rebase). When applying a batch of
  bumps manually, comment `@dependabot rebase` on the matching PRs instead
  of merging them — cheaper than reviewing 20 identical diffs.
- **GMTrade fill attribution MUST be verified** — gmsol-sdk's
  `complete_order()` watches STORE-WIDE CPI events and returns the last
  `TradeEvent` before any `OrderRemoved`, WITHOUT checking it belongs to
  our order PDA. Keepers batch many traders' fills in one tx, so a foreign
  fill can be handed back (Aug 9: a close logged `FILLED @ $3.12 pnl
  $391.12` on a $320 SOL long — another trader's event). `wait_for_fill`
  re-checks `TradeEvent.order == our order PDA` on every path and falls
  back to an order-PDA-scoped historical scan. Do NOT trust an SDK
  TradeEvent's price/pnl without this attribution check.

---

## Research fee model + fold artifact (critical)

1. **Never judge a strategy on the v1-era fee model** (open 0.06% + close
   0.06% + borrow 0.0042%/hr shorts-only ≈ 0.32%/trip). Measured Flash
   v2 costs were ≈ **0.06%/trip — ~5× cheaper**. The v6 S15
   "falsification" was entirely this artifact. Import `net_pnl_v2` from
   `research/missions/s15_v7_v2fee_recheck.py`.
2. **`create_folds()` absorbs all leftover bars into the LAST fold** when
   data is longer than `num_folds × test_window` — on multi-year data
   one mega-fold dominates the headline stats. Use explicit equal
   anchored windows (`equal_folds()` in `s15_v7e_corrected_folds.py`)
   for multi-year WFA.
3. **Latency absorber finding**: confirmation entries (close_reassert)
   pay detection latency at the fill (47% retention under +1-bar stress).
   Blind touch / limit-at-zone (`confirm_mode=none`) absorbs it via the
   order book (105% retention). `confirm_bars=2` is NOT an absorber
   (−88%).
4. **Momentum off-by-one — FIXED (Aug 7, `b178da7`)**: `timeframe_signal()`
   computed returns over the lookback-close slice (lookback−1 returns),
   so momentum/volatility were permanently 0.0 in production. Fixed to
   match the Python reference (`returns.rolling(lookback).mean()` over
   the full series). Do NOT reintroduce a window-slice returns
   computation here.

### Fast sim ↔ full sim calibration

The fast simulator (`per_symbol_optimizer`) MUST match the full
simulator exactly:

1. **ATR formula**: `std(returns, 20h) × price` — NOT True Range
2. **MR entry**: `rsi < 35 and daily_trend == bullish` — NOT
   `bull_count >= min_alignment`
3. **Sharpe annualization**: `sqrt(n_trades / total_hours × 8760)` —
   NOT `sqrt(24 × 365)`

If you change anything in `_compute_score()` or `simulate_trades()`, run
`evaluator_calibration.py` to verify directional agreement.

---

## Key on-chain invariants

The Anchor program (`rtp/programs/rtp-treasury/`) enforces these
on-chain. Full history is in `archive/COLOSSEUM-AUDIT.md` (v1.1 + v1.2
remediation). The current invariants:

1. **PDA owns treasury** — no private key risk
2. **Per-token isolation** — each mint gets its own Treasury PDA + vault
3. **CPI-only transfers** — atomic, verifiable
4. **No cross-chain** — execution stays on Solana
5. **Emergency freeze** — authority-gated halt; all 15 state-mutating
   instructions check the frozen flag
6. **Zero-address rejection** — `Pubkey::default()` rejected on all
   critical fields
7. **PDA seed validation** — cross-treasury corruption rejected
8. **Fee attribution authority-gated** — only `treasury.authority` can
   write strategy metrics
9. **Soft decay recovery** — strikes reset only after 3 consecutive
   positive updates (single lucky trade cannot clear strikes)

Trust model: **permissionless inbound** (anyone can pull fees into the
PDA, anyone can trigger the deterministic redistribution check), but
**authority-gated outbound + metric writes**. The PDA owns everything;
no private key exists for treasury funds.

---

## Railway Project: `resilient-token-protocol`

**Account**: katejcooper.atelier@gmail.com  
**Project**: https://railway.com/project/11004852-2ba7-46d9-aeb5-ab9558e965a0  
**Environment**: production (`986bee12-1028-4016-aa42-ba0a174233b4`)

| Service | Type | Dockerfile | Schedule | URL |
|---|---|---|---|---|
| **rtp-dashboard** | Always-on SSR | `Dockerfile.dashboard` (repo root) | — | https://rtp-dashboard-production.up.railway.app |
| **rtp-trader** | Always-on Rust | `rtp/swarm/Dockerfile.trader` | — | HTTP status server on port 8080 (Railway private networking) |
| **rtp-devnet-loop** | Cron | `rtp/swarm/Dockerfile.daemon` | `0 */6 * * *` | (dev only) |
| **rtp-night-shift** | Cron | `research/Dockerfile` | `0 14 * * *` UTC | (writes results back to repo) |
| **rtp-fee-crank** | Cron | `scripts/Dockerfile.crank` | `0 * * * *` | (dev only) |
| **rtp-promote-strategy** | Cron | `scripts/Dockerfile.promote` | `30 14 * * *` UTC | (uses last 2 days of night_results) |
| **rtp-swarm-ci** | Manual | `rtp/Dockerfile.ci` | manual | (cargo/anchor build + test) |

### Critical Railway gotchas

- **Root Directory must be `/`** for every service — all Dockerfiles use
  repo-root-relative paths. Setting `/dashboard` breaks the build
  (`sdk/` is outside).
- **`RAILPACK_DOCKERFILE_PATH=Dockerfile.dashboard`** on rtp-dashboard
  so Railway uses our Dockerfile, not Nixpacks.
- **Never `railway up`** — it wipes custom domain registrations. Use
  Railway dashboard redeploy or `railway redeploy --yes`. If domains are
  lost, re-add via GraphQL `customDomainCreate` + `customDomainUpdate`.
- **`rtp-night-shift` may lose GitHub repo connection** — reconnect via
  Settings → Connect Repo → `tradewife/resilient-token-protocol` if it
  stops auto-deploying.
- **Workspace API token** is in `.secrets/railway-workspace-token`
  (gitignored). Use `RAILWAY_TOKEN=$(cat .secrets/railway-workspace-token)`
  for GraphQL mutations. Regenerate at `railway.com/account/tokens` if
  missing.
- **Droid-Shield blocks AI-agent pushes** (false positives on Solana
  pubkeys). Manual push required after large commits.

### Operator helpers

```bash
node scripts/railway-logs.mjs --service rtp-trader --last 200
node scripts/railway-trader-override.mjs show
node scripts/railway-trader-override.mjs set --min-alignment 2 --signal-threshold 0.2
node scripts/railway-redeploy-trader.mjs
node scripts/check-railway-services.ts
```

These read `RAILWAY_TOKEN` from env or `.secrets/railway-workspace-token`.
They do NOT trigger deploys by themselves — run `redeploy-trader.mjs`
after override changes.

### Security

- **`RTP_OPERATOR_API_SECRET` gates position-clear** — must be set to the
  SAME value on `rtp-dashboard` AND `rtp-trader`. If unset, `/api/clear-position`
  fails closed with 503 (safer than public access). Rotate with
  `node scripts/railway-operator-secret.mjs --rotate`, then redeploy both.
- **No PII in `[DIAGNOSTIC-INTAKE]` logs** — log lines carry id/kind/
  timestamp/notified only. SQLite + Resend are the durable paths. Do not
  reintroduce name/email/payload into log lines.
- **Intake is rate-limited** — 5 POSTs / IP / 10 min, 32KB body cap, strict
  email validation, honeypot (constants in
  `dashboard/src/app/api/diagnostic-intake/route.ts`). Don't raise casually.
- **Security headers are global** (HSTS, `X-Frame-Options: DENY`, nosniff,
  Referrer-Policy, Permissions-Policy, CSP) via `headers()` in
  `dashboard/next.config.ts`. Keep `frame-ancestors 'none'` / `object-src 'none'`.
- **CORS is allow-listed** on public API routes — add new origins to
  `ALLOWED_ORIGINS` in `dashboard/src/lib/cors.ts`; never set
  `Access-Control-Allow-Origin: *`.
- Full posture + reporting: `SECURITY.md` (repo root).

---

## Quick setup

(Agent quick-reference: see `AGENTS.md` — layout, build/test commands,
naming conventions, and the CI-verified command list.)

```bash
# Dashboard
cd dashboard && npm ci && npm run dev      # http://127.0.0.1:3000

# Night Shift (dry run; --skip-fetch uses data/ohlcv)
python -m research.orchestration.night_shift --skip-fetch

# Rust swarm (full test suite)
cd rtp/swarm && cargo test --lib

# Anchor program
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet
```

---

## GitHub

- **This repo**: `git@github.com:tradewife/resilient-token-protocol.git`
- Research archives: `git@github.com:tradewife/fractal-swarm.git` (Python origin), `git@github.com:tradewife/rtp-skills-research.git`

---

## Design decisions (current)

- **Self-custody + kill switch** — every Bespoke Strategy Build deploys
  via scoped permission the client revokes. Capital never touches an
  RTP-controlled wallet.
- **Measured on-chain fees, not assumed** — venue costs are measured via
  the venue's `/preview/*` endpoints + live accrual. When a venue moves,
  we re-measure and re-validate. Stale numbers never touch client capital.
- **Bespoke anti-dilutive by construction** — each client gets a distinct
  strategy, no shared edges, capacity per client preserved.
- **Standardize process, customize product** — validation gates, drawdown
  limits, auto-suspension logic identical for every engagement. Only the
  strategy varies.
- **Venue is per-client, not platform-wide** — GMTrade for client #1
  specifically; future clients select their own or we select based on
  their mandate.
- **No `unwrap()` in production paths** — all external input flows use
  proper error handling (`map_err`, `unwrap_or_else`, `ok_or`).
- **Median OOS Sharpe** (not mean) — prevents single-fold outliers
  dominating.
- **Per-fold Sharpe winsorized at ±100** — prevents tiny-sample extremes.
- **Fragility is a penalty, not rejection** — `survivor *= 1/(1+fragility)`.
