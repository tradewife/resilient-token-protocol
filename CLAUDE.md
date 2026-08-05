# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**RTP (Resilient Token Protocol)** — a Solana-native, self-funding treasury governed by a modular Rust swarm. Any token project adopts RTP — their trading fees route to the swarm, which autonomously researches, validates, and executes yield strategies — returning yield back to the project and its holders. The swarm executes validated strategies as **on-chain perpetuals via Flash Trade CPI**, signed by the **Treasury PDA via `invoke_signed`** (no human keypair). All execution stays on Solana — no cross-chain bridge, no off-chain signing.

**Hackathon**: SWARMs / Canteen × Colosseum, deadline May 11, 2026.
**License**: BSL 1.1 (converts to Apache 2.0 on 2030-05-11)

---

## Execution Venue — Complete (Flash Trade CPI)

The Flash Trade on-chain CPI execution path is fully implemented (M0–M5). PDA-signed position open/close confirmed on mainnet. The Hyperliquid/Phantom MCP path is archived behind `#[cfg(feature = "hyperliquid")]`.

```
Night Shift (Python, DONE)
  └── validated strategy: SOL/USDT Survivor 2.69, signal_threshold=0.3, tp_atr=6.0, sl_atr=2.5, trailing_stop_atr=1.0, max_hold_hours=96, score_flip_delay_hrs=2, min_alignment=2
        │
        ▼ bridge.rs (DONE)
Trading Wing (Rust, DONE)
  └── ExecutePermit payload → reads on-chain state, builds Anchor instruction
        │
        ▼ RTP Treasury Program (on-chain, DONE)
           open_flash_position: validates frozen, strategy Live, runway floor
           invoke_signed with Treasury PDA seeds → Flash Trade Perpetuals CPI
        │
        ▼ Flash Trade Perpetuals Program (on-chain CPI)
           FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn (mainnet)
           Position opened/closed on Solana, fully auditable on Explorer
        │
        ▼ close_flash_position (invoke_signed) → SOL returned to treasury vault
        │
        ▼ check_redistribute on-chain (DONE)
           70% holders / 20% project dev / 10% ecosystem
        │
        ▼ Devnet loop daemon (DONE)
           6h cron, LLM-driven strategy evolution, auditable trail

Fee-Payer Wallet (gas only, DONE)
  └── Funded keypair pays Solana transaction gas (< 0.001 SOL/tx)
        │
        ▼ No authority over treasury funds — cannot sign for Treasury PDA
        ▼ Losing this key means losing gas money, not treasury funds
```

### Integration Resources
| Resource | URL |
|----------|-----|
| Flash Trade REST API | https://flashapi.trade |
| Flash Trade SKILL.md | `flash-trade/SKILL.md` (in repo) |
| Flash Trade TransactionFlow | `flash-trade/TransactionFlow.md` (in repo) |
| Flash Trade ProtocolConcepts | `flash-trade/ProtocolConcepts.md` (in repo) |
| Flash Trade SDK (TypeScript) | `flash-sdk` (NPM) |
| Flash Trade Program (mainnet) | `FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn` |
| Flash Trade Program (devnet) | `FTPP4jEWW1n8s2FEccwVfS9KCPjpndaswg7Nkkuz4ER4` |
| Composability Program (mainnet) | `FSWAPViR8ny5K96hezav8jynVubP2dJ2L7SbKzds2hwm` |
| Solana Wallet Adapter | Browser wallet for dashboard (`@solana/wallet-adapter-react`). Supports Phantom, Solflare, Backpack, and any Solana wallet. |

**Legacy (archived behind `#[cfg(feature = "hyperliquid")]`):**
| Resource | URL |
|----------|-----|
| Hyperliquid API docs | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Python SDK | https://github.com/hyperliquid-dex/hyperliquid-python-sdk |
| Testnet endpoint | https://api.hyperliquid-testnet.xyz/exchange |

### Signing Architecture
- **Flash Trade CPI signing**: Treasury PDA `invoke_signed` — no private key exists. The program IS the only authority.
- **Fee-payer wallet**: Funded keypair for Solana gas fees only — has zero authority over treasury funds
- **Dashboard signing**: `@solana/wallet-adapter-react` for browser wallet ops (freeze/unfreeze, multisig status). Supports Phantom, Solflare, Backpack, and any Solana wallet.
- **Phantom MCP** (archived): `phantom_mcp.rs` gated behind `#[cfg(feature = "hyperliquid")]`. Not compiled in default build. Available for legacy reference.

### Security Hardening (v1.1 → v1.2)

**v1.1 (Apr 26):**
- **Zero-address guard**: `Pubkey::default()` rejected on all critical fields in `initialize`.
- **Emergency freeze/unfreeze**: authority-gated. 15 state-mutating instructions check frozen flag. Events emitted for audit.
- **Emergency reset of position counters**: `emergency_close_all_positions` — zeroes `open_position_count` and `committed_sol_lamports`, emits `EmergencyPositionsReset`.
- **PDA seed validation on all treasury accounts**: `RecordFeeDeposit.treasury` and `RegisterAdopter.treasury` have `seeds` constraints.
- **FlashSide::None rejected for open/close**: positions must have a direction.
- **size_amount overflow guard**: `open_flash_position` checks `size_amount <= u64::MAX as u128` before truncation.
- **Soft decay strike recovery**: strikes reset after 3 consecutive positive updates (`recovery_counter >= MIN_RECOVERY_TRADES`).

**v1.2 — Colosseum Audit Remediation (Apr 29):**
- **`update_strategy_performance` authority check** (CRITICAL): now requires `treasury.authority`. Previously any signer could write arbitrary metrics and keep bad strategies Live.
- **`AdopterRecord.treasury` back-reference**: links adopter records to their treasury for cross-validation.
- **Anchor `constraint` on all authority-gated instructions**: `end_beta`, `register_strategy`, `force_retire_strategy`, `update_strategy_performance` use account-level constraints, not manual handler `require!`.
- **Open handler remaining accounts validation**: `open_flash_position` validates Flash Trade program ID at `remaining[15]` and treasury PDA at `remaining[0]`.
- **Recovery counter on `StrategyRecord`**: `recovery_counter: u8` requires 3 consecutive positive updates before strike reset. Single lucky trade cannot clear strikes.
- **`StrategyPerformanceUpdated` event** now includes `recovery_counter` field for audit.

### Operational Notes (Lessons Learned)
- **Never use `railway up` for redeployment** — it wipes custom domain registrations. Use Railway dashboard redeploy instead. If domains are lost, re-add via GraphQL `customDomainCreate` + `customDomainUpdate` to trigger verification.
- **rtp-night-shift may lose GitHub repo connection** — if the service stops auto-deploying on push, reconnect the repo in Railway dashboard (Settings → Connect Repo → select `tradewife/resilient-token-protocol`). Root directory must be `/` (repo root), Dockerfile path stays `research/Dockerfile`.
- **Dashboard RPC must match treasury network**: WalletProvider uses devnet RPC. The `/launch` page creates a separate mainnet connection for platform launches. Never mix networks — treasury balance reads from wrong RPC return 0.
- **Frozen state must use SDK decoder**: Never read on-chain frozen field via hardcoded byte offsets — the Anchor account layout can change. Use `fetchTreasuryState()` from the SDK which uses Borsh decoding.
- **FlashTradeClient is async**: The REST client uses async `reqwest` with retry (3 attempts, exponential backoff). Synchronous callers use `_blocking()` wrappers that create a tokio current-thread runtime.
- **No `.unwrap()` in production paths**: All msgpack encoding, ATA derivation, daemon serialization, and evolve proposals use proper error handling (`map_err`, `unwrap_or_else`, `ok_or`). Never re-introduce `.unwrap()` on code that handles external input.
- **Trader config loads from validated file** — `Dockerfile.trader` copies `data/trader-strategy-config.json` into container and sets `RTP_STRATEGY_CONFIG`. If config file is missing or invalid, falls back to hardcoded defaults with a warning log. The validated config (trail=1.0, tp=6.0, sl=2.5, hold=96h, decay=48h, flip_delay=2h) must NOT be silently replaced with defaults — check Railway logs for the startup param line.
- **Trader has loosening-only env overrides** — `RTP_TRADER_MIN_ALIGNMENT_OVERRIDE` and `RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE` let the operator relax strict-WFA confluence params on Railway without rebuilding the binary. Override values >= configured are silently ignored (one-way loosening); missing env = validated config. Application logs a `[OVERRIDE]` WARN line on every applied override. Use `node scripts/railway-trader-override.mjs set --min-alignment 2 --signal-threshold 0.2` then `node scripts/railway-redeploy-trader.mjs` to apply. Set values tighter than config = silent no-op (designed to prevent accidental overrides that would re-introduce riskier-than-baseline behavior).
- **Real multi-TF only (Jul 28, 2026)** — `compute_signal` must receive **independent** 1h / 4h / 1d close series from Binance (`interval=1h|4h|1d`). Never slice a single 1h buffer at 20/80/200 lookbacks; that made all three TFs lock together and blocked opposite-side entries for days. Warmup uses `tokio::join!` of three fetches; poll line shows `1h=N 4h=N 1d=N`.
- **Slow-TF buffers MUST be refreshed (Aug 5, 2026)** — after Binance warmup, `run_cycle` only calls `buffer_1h.append_tick()`. The 4h and 1d `CandleBuffer`s stay frozen at the warmup snapshot unless periodically refetched, so `tf_4h.trend`/`tf_1d.trend` compare a stale close vs a stale SMA and bullish/bearish counts can never flip with the market. Now fixed: 4h refreshes every 2h, 1d every 6h from Binance (`last_4h_refresh`/`last_1d_refresh` in `run_cycle`, logs `[REFRESH]`). This was bug A/B of the "trader blocked" root cause (`311457f`).
- **Entry gates on score only, NOT alignment count (Aug 5, 2026)** — the Rust entry adds an extra `bullish_count >= min_alignment` AND-gate that the Python Survivor 2.69 reference (`run_backtest_r2.py` line ~257) does NOT have. The alignment count is already baked into the score as the trend weight (0.4 × bull/3), so the extra gate double-counts it and caps the score at 0.267 in sideways markets (momentum/MR/BB don't fire). This was bug C of the "trader blocked" root cause (`311457f`). Long = `score > threshold`, Short = `score < -threshold`.
- **min_alignment=2 (Aug 4, 2026)** — `data/trader-strategy-config.json` uses `min_alignment: 2`, matching the Python reference (`research/simulation/run_backtest_r2.py`). The old `min_alignment=3` was a stale inheritance from the fake-multi-TF era (when bull/bear always showed 3/3 together) and was **never WFA-validated** — it is absent from `data/sensitivity_sol_survivor_2_69_lev3.csv`. Do NOT "fix" this back to 3; re-verify against the WFA sweep before touching it. This was a necessary but insufficient fix (see the two Aug 5 notes above for the rest of the story).
- **Flash MinCollateral / 0x1792** — on-chain `custom program error: 0x1792` is **6034 MinCollateral**, not basket.delegate. Collateral must clear Flash's ~$11–12 floor. Production sizing: `RTP_TRADER_POSITION_FRACTION=0.20` and `RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS=150000000` on ~0.9 SOL wallet. 1% fraction produced ~$0.66 notionals that always failed open.
- **Trader supports both LONG and SHORT positions** — entry conditions: LONG when score > threshold AND bullish_count >= min_alignment; SHORT when score < -threshold AND bearish_count >= min_alignment. Exit math (PnL, trailing, SL/TP) is inverted for SHORT positions. OpenPosition has a `side` field ("Long"/"Short").
- **Score flip delay** — `score_flip_delay_hrs` (default 0.0, set to 2.0 in validated config) provides a grace period before ScoreFlip exit. Timer starts from `first_negative_score_time` (tracked in OpenPosition), resets when score goes positive.
- **/health endpoint returns 503 when unhealthy** — consecutive_errors >= 5 OR last_healthy > 30 minutes ago. Not a static "ok" anymore.
- **/state returns active_config** — TraderState now includes the loaded StrategyParams so config drift is visible from the dashboard.

### Flash SDK v2 Migration (Jul 7, 2026) + Deposit Fix (Jul 23, 2026)

- **Migration scope**: `rtp/swarm/src/trader/executor.rs::open_position` / `close_position` now attempt the Flash SDK v2 wrapper first; on `node unavailable` they fall back to the legacy `/transaction-builder/*` REST path. Strategy rules (TP/SL/trail/time-decay/score-flip-delay/MR-target), risk management (priority ordering, side-correct PnL math), and position sizing (computed in `trader/mod.rs`, forwarded as `amount_sol`) are **unchanged**. All 87 trader tests still pass.
- **Wrapper**: `cli/flash-sdk-wrapper.mjs` is a thin stdio JSON-RPC bridge loaded by `rtp/swarm/src/trader/executor.rs::FlashSdkClient`. It uses `@flash_trade/flash-sdk-v2@1.0.46` (pinned in `cli/package.json`), `Side`/`isVariant`/`PoolConfig.fromIdsByName("Crypto.1","mainnet-beta")` per the SDK's trader-interactions guide. Collateral resolved from the market PDA — never hardcoded — to avoid `Custom 2006` (ConstraintSeeds). `sizeAmount` derived from `getOpenPositionQuoteEr` to avoid `Custom 6021/6023`. The wrapper loads the keypair exclusively from `RTP_TRADER_KEYPAIR_JSON` (env), never argv.
- **Deposit fix (Jul 23, 2026)**: The Flash program's Squads upgrade at slot 434407053 (~2026-07-22T00:33Z) removed the bare `deposit_direct` on-chain instruction. The SDK's `c.depositDirect()` calls that instruction directly and gets `InstructionFallbackNotFound (101)`. Without a funded deposit ledger, every `openPositionEr` fails with `CustodyAmountLimit (6024)`. **Fix**: `doSetup()` in `cli/flash-sdk-wrapper.mjs` now calls the Flash REST API's `POST /transaction-builder/deposit` endpoint instead, which builds a composite 4-instruction tx (system + token + flash + token) that bundles all missing setup (basket, deposit ledger, delegation, trade vault) and works with the deployed binary. Verified on mainnet: deposit 0.5 SOL confirmed, open + close cycle verified (open sig `2gkCXg...`, close sig `2kwwEh...`). The recovery script `scripts/flash-fund-and-open-sol.mjs` was also updated to use this API path.
- **Idempotent setup**: The wrapper's `setup` skips the deposit step when `fetchUserDepositLedger` reports an already-funded SOL balance (>= 0.05 SOL). Previously called `depositDirect`; now calls the API `/deposit` one-shot.
- **Production trader wallet**: `HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5` (keypair at `~/.config/solana/rtp-trader.json`). Do NOT use the local default `id.json` (`Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R`) — that is a different wallet. The MCP `sign_and_send` tool can silently load `id.json` if `KEYPAIR_PATH` is not injected; prefer local Node signing with `rtp-trader.json` until MCP env injection is fixed.
- **Respawn lifecycle**: `FlashSdkState` tracks spawn attempts in a rolling 60s window. After 3 spawn failures, the cell returns `node unavailable (respawn budget exceeded)` so callers fall back to REST. Child process deaths detected by `is_sdk_dead_error` patterns on the response error strings.
- **Docker**: `rtp/swarm/Dockerfile.trader` now has a `node:20-bookworm-slim` intermediate stage that runs `npm ci` against `cli/package.json` and bakes the wrapper + node_modules into `/app/wrapper/`. Runtime stage installs `nodejs` + sets `RTP_TRADER_WRAPPER_PATH=/app/wrapper/flash-sdk-wrapper.mjs`, `RTP_TRADER_ER_RPC=https://flash.magicblock.xyz`, `RTP_SOLANA_RPC_URL=https://api.mainnet-beta.solana.com` (the first three have defaults baked in but remain env-overridable for dev). Node is required: removing it from the image forces the trader to rely on REST.
- **Env**: `RTP_TRADER_KEYPAIR_JSON` is read directly from the wrapper (env-only, never CLI args). It's already set on Railway's `rtp-trader` service. No change needed to existing Railway env vars.
- **Flash V2 API field reference**: Open uses `inputTokenSymbol`, `outputTokenSymbol`, `inputAmountUi`, `leverage` (numeric), `tradeType` (LONG/SHORT), `owner`, `orderType` (MARKET/LIMIT), `slippagePercentage`. Close uses `marketSymbol`, `side` (LONG/SHORT), `inputUsdUi`, `closeAll` (boolean), `withdrawTokenSymbol`, `owner`, `slippagePercentage`. Deposit uses `owner`, `tokenSymbol` (SOL/USDC), `amount` (UI units). All trading txs submit to ER (`https://flash.magicblock.xyz`); funds/setup txs submit to Solana mainnet RPC. API returns partially signed txs — wallet adds owner signature only, never mutates blockhash.
- **Memory**: This migration lives next to the existing "Operational Notes (Lessons Learned)" — keep the wrapper path in sync. Spec at `/home/kt/.factory/specs/2026-07-07-spec-corrected-replace-rest-api-calls-with-flash-sdk-v2-via-node-js-child-proces.md`.

---

## Repo Layout

This repo has three layers:
1. **Proven Python fractal-swarm** (shipping) — backtesting, optimization, paper trading
2. **Rust swarm + Solana treasury** (built, 362 unit + 5 integration tests) — 6-wing architecture, Coordinator, soulcontract, Flash Trade CPI execution, emergency freeze, zero-address guard
3. **Flash Trade CPI execution** (done — mainnet verified) — Trading Wing → Treasury PDA invoke_signed → Flash Trade Perpetuals CPI → on-chain positions → SOL yield → treasury PDA

---

## Quick Setup

```bash
# Python environment (fractal-swarm)
python -m venv .venv && source .venv/bin/activate
pip install pandas numpy ccxt pyarrow redis

# Night shift (30K configs/night, 9-fold WFA)
python -m research.orchestration.night_shift --skip-fetch

# Paper trading (live Binance)
PYTHONUNBUFFERED=1 python -m research.live.paper_trader

# Full-sim validation
python -m research.validation.validate_night_shift --production

# Self-correction
python -m research.optimization.evaluator_calibration --samples 20
python -m research.validation.discrepancy_detector

# Rust (swarm runtime)
cd rtp/swarm && cargo build
cd rtp/programs/rtp-treasury && anchor build
```

---

## Commands

### Python (Yield Brain)

```bash
python -m research.orchestration.night_shift
python -m research.orchestration.night_shift --skip-fetch
python -m research.orchestration.night_shift --symbols SOL/USDT
python -m research.live.paper_trader
python -m research.validation.validate_night_shift --production
python -m research.validation.validate_night_shift --symbol SOL/USDT --top 3
python -m research.optimization.evaluator_calibration --samples 20
python -m research.validation.discrepancy_detector
python -m research.data.download_ohlcv
```

### Rust (Swarm Runtime)

```bash
cd rtp/swarm && cargo build --release
cd rtp/swarm && cargo test
cd rtp/swarm && cargo run --bin rtp-daemon    # single devnet cycle
cd rtp/swarm && cargo run --bin rtp-demo      # full 8-step demo + Flash Trade CPI
cd rtp/swarm && cargo run --bin rtp-trader   # live autonomous trader (REST API + HTTP status server)
cd rtp/swarm && cargo test --lib trading::tests
cd rtp/swarm && cargo test --lib trading::flash_trade_client::tests  # Flash Trade REST client
cd rtp/swarm && cargo test --lib audit::tests
cd rtp/swarm && cargo test --test coordinator_integration
```

### Solana (Treasury Program)

```bash
cd rtp/programs/rtp-treasury && anchor build
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet
cd rtp/programs/rtp-treasury && anchor deploy --provider.cluster devnet
```

### Flash Trade (Demo + Account Derivation)

```bash
# Derive all Flash Trade PDAs offline
npx tsx cli/bin/rtp.ts accounts derive --mint <MINT_PUBKEY>

# Flash Trade CPI demo (mainnet simulation)
# NOTE: scripts/flash-trade-demo.ts archived to scripts/archive/ — use rtp demo instead
npx tsx cli/bin/rtp.ts demo

# Derive PDAs (legacy script — prefer rtp CLI)
npx tsx scripts/derive_flash_accounts.ts
```

### Operator CLI (`cli/`)

The `rtp` CLI consolidates all operational scripts into a single Commander.js tool. It is the ops interface for whoever deploys, monitors, and controls the protocol.

```bash
# Interactive onboarding wizard
npx tsx cli/bin/rtp.ts init

# Deploy treasury PDA for a new token
npx tsx cli/bin/rtp.ts deploy treasury --mint <PUBKEY> --authority <KEYPAIR>

# Sweep fees into treasury vault
npx tsx cli/bin/rtp.ts crank fees --mint <PUBKEY>

# Trigger 70/20/10 redistribution
npx tsx cli/bin/rtp.ts crank redistribute --mint <PUBKEY> --dry-run

# Emergency freeze (authority-gated, --yes required)
npx tsx cli/bin/rtp.ts freeze --mint <PUBKEY> --authority <KEYPAIR> --yes

# Derive all PDAs offline (no RPC needed)
npx tsx cli/bin/rtp.ts accounts derive --mint <PUBKEY>

# Fetch live treasury state
npx tsx cli/bin/rtp.ts accounts show --mint <PUBKEY>

# Protocol health overview
npx tsx cli/bin/rtp.ts status --mint <PUBKEY>

# Railway service status
npx tsx cli/bin/rtp.ts status services

# Full 8-step demo (replaces demo.sh)
npx tsx cli/bin/rtp.ts demo                    # dry-run (default)
npx tsx cli/bin/rtp.ts demo --execute          # actually send transactions

# Promote validated strategy to Live
npx tsx cli/bin/rtp.ts strategy promote --id <STRATEGY_ID> --authority <KEYPAIR>

# Force-retire a strategy (destructive, --yes required)
npx tsx cli/bin/rtp.ts strategy retire --id <STRATEGY_ID> --authority <KEYPAIR> --yes
```

All commands support `--json` (machine-readable), `--quiet` (errors only), `--cluster <devnet|mainnet>`.

**Implementation:** `cli/` with Commander.js, TypeScript, chalk, inquirer, ora, cli-table3. Imports from `sdk/` and refactored exports in `scripts/`. See `cli/README.md` for full command reference.

**Script refactoring:** `fee-crank.ts`, `promote-strategy.ts`, `emergency-freeze.ts`, `derive_flash_accounts.ts` now export async functions (`exportSweepFees`, `exportPromoteStrategy`, `exportFreezeTreasury`/`exportUnfreezeTreasury`, `exportDeriveAccounts`) with guarded `main()` calls (only run when executed directly, not when imported). Railway Dockerfiles call the scripts directly and remain unchanged.

**Archived:** `demo.sh` and `scripts/flash-trade-demo.ts` moved to `scripts/archive/`. Use `rtp demo` instead.

### Railway Operator Helpers (`scripts/`)

Lightweight Node-based GraphQL helpers for the rtp-trader service. Read `RAILWAY_TOKEN` from env or `.secrets/railway-workspace-token`. They do NOT trigger a deploy by themselves.

```bash
# View current rtp-trader env vars (filter for override block)
node scripts/railway-trader-override.mjs show

# Loosen strict-WFA confluence params on the fly (deploy after to apply)
node scripts/railway-trader-override.mjs set --min-alignment 2 --signal-threshold 0.2
node scripts/railway-redeploy-trader.mjs

# Revert to validated config (one-shot — both vars go away)
node scripts/railway-trader-override.mjs unset
node scripts/railway-redeploy-trader.mjs
```

**Override semantics:** see "Trader has loosening-only env overrides" in Operational Notes below. The script silently no-ops on values >= configured (one-way loosening is enforced in Rust, not the helper — the helper just sets whatever the operator passes).

---

## Architecture

### Three-Layer Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│  Treasury PDA: fees → yield → redistribute → self-hydrate       │
│  Flash Trade CPI: invoke_signed → open/close positions          │
│  Phase evolution: Sustenance → Ecosystem → Humanity Fund        │
├─────────────────────────────────────────────────────────────────┤
│                    SWARM RUNTIME (Rust)                          │
│  Coordinator → message bus → 6 wings (trading, security,        │
│  evolve, knowledge, audit, futureproof)                          │
│  Trading Wing → Flash Trade CPI → on-chain perps → SOL yield    │
│  Signed by Treasury PDA via invoke_signed (no human key)         │
├─────────────────────────────────────────────────────────────────┤
│                    RESEARCH LAYER (Python)                       │
│  Night Shift: 30K configs → WFA → Darwinian → full-sim validate │
│  Paper Trader: live Binance → state persistence → degradation   │
└─────────────────────────────────────────────────────────────────┘
```

### Key Files

#### Python (Yield Brain)

| File | Purpose |
|------|---------|
| `research/orchestration/night_shift.py` | Main pipeline: grid search → WFA → Darwinian → report → validation |
| `research/optimization/per_symbol_optimizer.py` | Fast simulator: `compute_indicators()`, `simulate_trades()`, `_compute_score()` |
| `research/live/paper_trader.py` | Live paper trader: polls Binance, ADX filter, per-symbol configs |
| `research/validation/validate_night_shift.py` | Bridges fast sim → full sim for candidate validation |
| `research/simulation/run_backtest_r2.py` | Production `MultiTFStrategy` class + `timeframe_signal()` helper |
| `research/optimization/evaluator_calibration.py` | Compares fast vs full sim on random configs |
| `research/validation/discrepancy_detector.py` | Post-night-shift check, flags fast/full sim divergences |
| `research/simulation/future_blind_simulator.py` | `FutureBlindSimulator`: 0.1% fees, 10bps slippage, max 20% position |

#### Rust (Swarm Runtime)

| File | Purpose |
|------|---------|
| `rtp/swarm/src/types.rs` | Message, Payload, WingId, Priority — all swarm types |
| `rtp/swarm/src/bridge.rs` | Python ↔ Rust typed subprocess interface |
| `rtp/swarm/src/demo.rs` | End-to-end demo loop (8-step pipeline) |
| `rtp/swarm/src/coordinator/mod.rs` | Multi-stage quality gate (soulguard → router → audit) |
| `rtp/swarm/src/coordinator/soulguard.rs` | Enforce soulcontract on every message |
| `rtp/swarm/src/coordinator/soulcontract_spec.rs` | Parse SOULCONTRACT.md → structured constraints + drift detection |
| `rtp/swarm/src/coordinator/lifecycle.rs` | Wing spawn, health-check, retire |
| `rtp/swarm/src/wings/trading/mod.rs` | **Trading Wing — Flash Trade CPI execution, PnL tracking, apply_mutations** |
| `rtp/swarm/src/wings/trading/types.rs` | Trading types — StrategyConfig, PositionState, TradingState |
| `rtp/swarm/src/wings/trading/flash_trade_client.rs` | **Flash Trade REST API client — markets, prices, positions, pool data queries** |
| `rtp/swarm/src/wings/trading/phantom_mcp.rs` | **[ARCHIVED]** Phantom MCP client — gated behind `#[cfg(feature = "hyperliquid")]`, not compiled by default |
| `rtp/swarm/src/bin/rtp-daemon.rs` | **Devnet loop daemon — real chain execution via chain_client, stale position close, single-cycle (Railway cron) or watchdog mode (RTP_WATCHDOG=1)** |
| `rtp/swarm/src/chain_client.rs` | **On-chain client — ChainConfig from env, ExecutionMode simulate/devnet/mainnet, PDA derivation, open/close instruction builders, submit/simulate with retry** |
| `rtp/swarm/src/trader/mod.rs` | **Live autonomous trader — REST API trading via Flash Trade, LONG + SHORT positions, score flip delay, health monitoring (503 on stale/error), Arc<Mutex<TraderState>> with active_config, HTTP status server on configurable port, watchdog (120s cycle timeout, consecutive error tracking, exponential backoff)** |
| `rtp/swarm/src/wings/security/mod.rs` | Threat detection, rate-limiting, suspicious-proposal detection |
| `rtp/swarm/src/wings/evolve/` | Assessor, proposer, rollback (complete, tested) |
| `rtp/swarm/src/wings/knowledge/mod.rs` | Persistent knowledge store (JSON file-backed), cross-wing queries |
| `rtp/swarm/src/wings/audit/mod.rs` | 3-agent tribunal (Skeptic/UserProxy/Optimizer), Byzantine consensus |
| `rtp/swarm/src/wings/futureproof/mod.rs` | Deprecation monitoring, heartbeat |

#### Solana (Treasury Program)

| File | Purpose |
|------|---------|
| `rtp/programs/rtp-treasury/` | Anchor: withdraw_fees, check_redistribute, hydrate_swarm, evolve_phase, **open_flash_position, close_flash_position, emergency_close_all_positions** |

#### Operator CLI

| File | Purpose |
|------|---------|
| `cli/bin/rtp.ts` | Entry point — `npx tsx cli/bin/rtp.ts <command>` |
| `cli/src/index.ts` | Commander program setup, register all commands |
| `cli/src/commands/init.ts` | `rtp init` — interactive onboarding wizard |
| `cli/src/commands/demo.ts` | `rtp demo` — full 8-step demo pipeline (replaces demo.sh) |
| `cli/src/commands/freeze.ts` | `rtp freeze` / `rtp unfreeze` — emergency halt/resume |
| `cli/src/commands/crank.ts` | `rtp crank fees` / `rtp crank redistribute` |
| `cli/src/commands/accounts.ts` | `rtp accounts derive` / `rtp accounts show` |
| `cli/src/commands/status.ts` | `rtp status` / `rtp status services` (Railway) |
| `cli/src/commands/strategy.ts` | `rtp strategy list` / `promote` / `retire` |
| `cli/src/commands/deploy.ts` | `rtp deploy treasury` / `rtp deploy program` |
| `cli/src/config.ts` | Config loading (`~/.rtp/config.json`), resolution order |
| `cli/src/keypair.ts` | Keypair loading, pubkey truncation, SOL formatting |
| `cli/src/format.ts` | Output formatting (human/JSON/quiet) |
| `cli/src/errors.ts` | Error types with actionable hints |
| `cli/src/lib/railway.ts` | Railway GraphQL API client |
| `cli/src/lib/safety.ts` | Confirmation prompts, hot-wallet warnings |

#### Governance

| File | Purpose |
|------|---------|
| `SOULCONTRACT.md` | Constitutional governance — invariants, execution constraints, key links |
| `SESSION-CONTEXT.md` | Compressed project memory — paste into every fresh session |
| `docs/RESOURCES.md` | All hackathon links, SDK links, sponsor links |
| `docs/SECURITY_AUDIT_2026-04-07.md` | Full security audit — 18 findings |
| `docs/CODEREVIEW.md` | Code review protocol |
| `docs/demo-flow.md` | 3-minute hackathon demo script |
| `cli/README.md` | Operator CLI command reference |

---

## Devnet Limitations

### Flash Trade — Pyth Oracle Mainnet-Only

Flash Trade uses **Pyth Network** oracles for pricing. Pyth prices are **mainnet only** — devnet returns stale/zero prices, causing `StaleOraclePrice` (error 6007) on all position operations.

**Impact on RTP:** Flash Trade CPI execution (open/close positions) cannot work on devnet. Constraint logic tests (frozen, strategy gate, position limits) run on local validator without invoking actual Flash Trade CPI.

**Mainnet CPI proofs (M1):**
- Open position: TX `2bLg1Fu...` — 99,214 CU consumed, confirmed on mainnet
- Close position: TX `dFqkoP2...` — confirmed on mainnet

**Testing strategy:**
| Environment | Purpose | Works |
|---|---|---|
| Mainnet | Full CPI with real prices | Yes — micro positions (~$11-12 USDC minimum) |
| Local validator | Constraint logic (frozen, runway, strategy gate) without CPI | Yes — 9/9 tests passing |
| Devnet | Account derivation only (no fills) | Partial — stale oracle prices |

**Flash Trade devnet program:** `FTPP4jEWW1n8s2FEccwVfS9KCPjpndaswg7Nkkuz4ER4` (available but limited by oracle)

```bash
# Run Flash Trade CPI tests (local validator):
cd rtp/programs/rtp-treasury && anchor test

# Run Rust swarm tests (325 tests):
cd rtp/swarm && cargo test --lib
```

---

## Key Invariants (enforced on-chain)

1. **PDA owns treasury** — no private key risk
2. **Per-token isolation** — each mint gets its own Treasury PDA + vault, no shared pool, no honeypot
3. **SPL TransferFeeConfig immutable from mint** — fee percentage and withdraw authority cannot be revoked. Platform-level fee routing varies (Pump.fun: one-time, Bags.fm: anytime, Raydium: manual).
4. **CPI-only transfers** — atomic, verifiable
5. **No SOL liquidation** — SOL committed as Flash Trade input via Composability swap-and-open; positions are on-chain on Solana, no cross-chain risk
6. **Flash Trade CPI-only execution** — Treasury PDA signs via `invoke_signed`, no human keypair involved in trading
7. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity
8. **Auto-rollback if performance degrades > 5% post-amendment**
9. **Self-hydration only if sustenance bucket > 90-day runway**
10. **Research code remains reviewable while collaboration is active**
11. **Emergency freeze** — authority-gated halt, all 15 state-mutating instructions check frozen flag. Unfreeze also authority-gated.
12. **Zero-address rejection** — `Pubkey::default()` rejected on all critical fields
13. **FlashSide::None rejection** — open/close require Long or Short direction, None is rejected with `InvalidFlashSide` error
14. **size_amount bounds check** — `u128` to `u64` truncation guarded by `require!(size_amount <= u64::MAX as u128)`
15. **Soft decay recovery** — strikes reset to 0 after 3 consecutive positive updates (`recovery_counter >= MIN_RECOVERY_TRADES`). Single lucky trade cannot clear strikes.
16. **PDA seed validation** — `RecordFeeDeposit.treasury` and `RegisterAdopter.treasury` have seeds constraints, preventing cross-treasury corruption
17. **AdopterRecord.treasury cross-validation** — `HydrateSwarm`, `RecordFeeDeposit`, and `EndBeta` enforce `adopter_record.treasury == treasury.key()`. Cross-treasury adopter records rejected with `AdopterTreasuryMismatch`.
18. **Fee attribution authority-gated** — `record_fee_deposit` requires `authority.key() == treasury.authority`. Random signers cannot inflate adopter contributions.

## Trust Model — Permissionless Recording, Authority-Gated Actions

The on-chain program separates instructions into two categories:

**Authority-gated (treasury.authority required):**
- `initialize` — creates treasury, sets authority/wallets/runway
- `evolve_phase` — irreversible phase transitions, authority checked via Anchor constraint
- `register_strategy` — promotes strategy to Live status
- `force_retire_strategy` — emergency strategy retirement
- `end_beta` — manual beta adopter sunset
- `freeze_treasury` — emergency halt, sets frozen=true, no time lock (emergency speed)
- `unfreeze_treasury` — resume operations, authority-gated. Post-launch: Squads 2-of-3 + 24h time lock
- `open_flash_position` — open Flash Trade perps position via CPI (invoke_signed)
- `close_flash_position` — close position, SOL returned to treasury vault
- `emergency_close_all_positions` — authority-gated, closes all open positions during freeze
- `update_strategy_performance` — authority-gated (v1.2). Only treasury.authority can write strategy metrics. Prevents arbitrary metric manipulation.

**Permissionless (any signer can call):**
- `withdraw_fees` — anyone can pull TransferFeeConfig fees INTO the PDA vault (not out)
- `check_redistribute` — anyone can trigger 70/20/10 split (deterministic, no discretion)
- `create_swarm_vault` — anyone can pay to create the hydration vault (no authority check)
- `hydrate_swarm` — anyone can propose hydration (gated by strategy Live status + beta check + runway invariant)
- `register_adopter` / `register_adopter_beta` — anyone can create an adopter record (caller pays rent)
- `record_fee_deposit` — authority-gated (v1.3). Only treasury.authority can record fee accounting. Prevents arbitrary metric inflation. AdopterRecord.treasury cross-validated against treasury.
- `verify_adoption` — read-only verification

**Flash Trade CPI instructions (fee-payer submits, Treasury PDA signs via invoke_signed):**
- `open_flash_position` — any fee-payer can submit, PDA signs the CPI. Validates: not frozen, strategy Live, position count < 3, runway floor, position size ≤ 20% vault.
- `close_flash_position` — any fee-payer can submit. Permitted even if strategy Suspended (exiting is always safe).
- `emergency_close_all_positions` — authority-gated. Closes up to 3 positions. Used with freeze_treasury for emergency halts.

**Why this is safe:** Permissionless instructions either move funds INTO the PDA (never out) or record accounting state (no fund movement). All strategy metric writes require treasury.authority. The PDA owns all treasury assets — no private key can sign them away. Cumulative counters use `saturating_add` (never panics, never overflows to wrong values).

**Known mainnet considerations (accepted for launch, post-launch improvements):**
- `evolve_phase` thresholds checked against raw vault balance, not oracle-denominated USD. Authority manually verifies reserves before calling. Post-launch: integrate Pyth/Switchboard oracle.
- `check_redistribute` emits a `Redistribution` event for auditability (added Apr 2026).
- `freeze_treasury` / `unfreeze_treasury` events (`TreasuryFrozen`, `TreasuryUnfrozen`) emitted for audit (added Apr 2026).
- All 15 state-mutating instructions check `treasury.frozen` flag before executing (12 original + 3 Flash Trade CPI, added Apr 2026).
- `reject_zero_address` guard on `initialize` for all critical fields (added Apr 2026).

---

## Hackathon Resources

| Resource | Use in RTP | Link |
|---------|-----------|------|
| Flash Trade Perpetuals | **Execution venue**. On-chain Solana perps DEX. CPI via `invoke_signed` from Treasury PDA. REST API for queries (prices, positions, markets). Pool-to-peer model, up to 100x leverage, Pyth oracle pricing. | `flash-trade/SKILL.md` (in repo), https://flashapi.trade |
| Solana Wallet Adapter | **Browser wallet**. `@solana/wallet-adapter-react` wired to dashboard (/, /launch, /docs). Supports Phantom, Solflare, Backpack, and any Solana wallet. Wallet connect + live token launch flow operational on devnet. | https://github.com/solana-labs/wallet-adapter |
| CASH stablecoin | Third-party resource (not currently used) | https://phantom.app/cash |
| Squads Multisig | Post-launch: `treasury.authority` rotation to Squads PDA for 2-of-3 multisig governance | https://docs.squads.so |
| Swig | Programmable smart wallets for wing message bus | https://docs.swig.fi |
| MoonPay Agents | Agent money movement infrastructure | https://www.moonpay.com/developers/agents |
| Solana MCP | AI dev assistant for Anchor | https://github.com/solana-developers/solana-mcp |
| Arcium | Encrypted computation (stretch) | https://docs.arcium.com |

---

## Critical: Fast Sim Calibration

The fast simulator (`per_symbol_optimizer`) MUST match the full simulator exactly. Three invariants:

1. **ATR formula**: `std(returns, 20h) × price` — NOT True Range
2. **MR entry condition**: `rsi < 35 and daily_trend == bullish` — NOT `bull_count >= min_alignment`
3. **Sharpe annualization**: `sqrt(n_trades / total_hours × 8760)` — NOT `sqrt(24 × 365)`

If you change anything in `_compute_score()` or `simulate_trades()`, run `evaluator_calibration.py` to verify directional agreement.

---

## Yield Brain Results

| Symbol | Production PnL | Optimized PnL | Consistency | Trades |
|--------|---------------|--------------|-------------|--------|
| SOL/USDT | +36.9% | **+118.3%** | 78% → **100%** (optimized) | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

Active symbols: BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT. XRP dropped (net negative).

**Top live candidate (Apr 9 Night Shift):**
SOL/USDT Survivor 2.69 — signal_threshold=0.3, tp_atr=6.0, sl_atr=2.5, max_hold=96h, trailing_stop_atr=1.0, time_decay_hours=48, score_flip_delay_hrs=2, min_alignment=2
This is the config the Trading Wing targets on Flash Trade (via on-chain CPI).

---

## CI/CD

All CI/CD runs on **Railway** (migrated from GitHub Actions to conserve Actions minutes).

### Railway Project: `resilient-token-protocol`

| Service | Type | Dockerfile | Schedule | URL |
|---------|------|-----------|----------|-----|
| **rtp-dashboard** | Always-on SSR | `Dockerfile.dashboard` | — | https://rtp-dashboard-production.up.railway.app |
| **rtp-devnet-loop** | Cron (one-shot) | `rtp/swarm/Dockerfile.daemon` | `0 */6 * * *` (every 6h) | https://rtp-devnet-loop-production.up.railway.app |
| **rtp-night-shift** | Cron (one-shot) | `research/Dockerfile` | `0 14 * * *` (daily 14:00 UTC) | https://rtp-night-shift-production.up.railway.app |
| **rtp-swarm-ci** | Manual trigger | `rtp/Dockerfile.ci` | Manual redeploy only | https://rtp-swarm-ci-production.up.railway.app |
| **rtp-fee-crank** | Cron (one-shot) | `scripts/Dockerfile.crank` | `0 * * * *` (hourly) | — |
| **rtp-promote-strategy** | Cron (one-shot) | `scripts/Dockerfile.promote` | `30 14 * * *` | — |
| **rtp-trader** | Always-on | `rtp/swarm/Dockerfile.trader` | — | HTTP status server on port 8080 (Railway private networking). Config loaded from `data/trader-strategy-config.json` via `RTP_STRATEGY_CONFIG` env var. |

**Railway account:** katejcooper.atelier@gmail.com
**Project dashboard:** https://railway.com/project/11004852-2ba7-46d9-aeb5-ab9558e965a0
**Region:** Southeast Asia (asia-southeast1-eqsg3a)
**Environment:** production (`986bee12-1028-4016-aa42-ba0a174233b4`)

### Service Details

- **rtp-dashboard**: SSR Next.js (`output: "standalone"` in `next.config.ts`). Multi-stage Docker build from `Dockerfile.dashboard` at repo root. Auto-deploys from connected GitHub repo on push to main. **Railway config: Root Directory must be `/` (repo root), NOT `/dashboard`** — the Dockerfile needs access to both `sdk/` and `dashboard/`. Env var `RAILPACK_DOCKERFILE_PATH=Dockerfile.dashboard` tells Railway to use our Dockerfile instead of Nixpacks. The standalone build copies static assets to `.next/static` (not `dashboard/.next/static`) relative to `server.js`.
- **rtp-devnet-loop**: Rust `rtp-daemon` binary. Dockerfile uses `rust:1.88-slim` builder + `debian:bookworm-slim` runner. Connected to GitHub repo (`tradewife/resilient-token-protocol`), auto-deploys on push. Build context is repo root — COPY paths in Dockerfile use `rtp/swarm/` prefix. Needs env vars: `LLM_API_BASE_URL`, `LLM_API_KEY`, `LLM_MODEL`.
- **rtp-night-shift**: Python 3.12, installs from `requirements-ci.txt`, runs `night_shift --skip-fetch`. One-shot: runs to completion and exits. OHLCV data in `data/ohlcv/` included via `.railwayignore` exclusion.
- **rtp-swarm-ci**: Rust builder with Solana CLI + Anchor. Runs `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check`, `anchor build`. One-shot CI validation.
- **rtp-trader**: Always-on Rust binary (`rtp-trader`). Runs Survivor 2.69 strategy autonomously, polls Flash Trade every 5 minutes, executes SOL LONG and SHORT positions when signal conditions met. HTTP status server on port 8080 serves `GET /state` (live TraderState JSON) and `GET /health` (returns 503 when consecutive_errors >= 5 or last_healthy stale > 30min). State shared via `Arc<Mutex<TraderState>>` between trading loop and HTTP handler. Dashboard fetches via Railway private networking (`http://rtp-trader.railway.internal:8080/state`). Dockerfile: `rtp/swarm/Dockerfile.trader`. Env var `RTP_TRADER_HTTP_PORT` (default 8080). **Watchdog:** cycle wrapped in `tokio::time::timeout(120s)` — kills hung cycles (e.g., stalled HTTP). Tracks `consecutive_errors` + `last_healthy` in TraderState. Exponential backoff on repeated failures, 5-min sleep after 10 consecutive. All HTTP clients have 30s timeouts (Flash Trade API, Solana RPC).

### Cron Schedule Configuration

Cron schedules are set via Railway's GraphQL API (`serviceInstanceUpdate` mutation) — not via CLI. Requires a workspace-level API token. The workspace token is stored locally at `.secrets/railway-workspace-token` (gitignored, never committed).

```bash
# Railway workspace token (local only, gitignored)
RAILWAY_TOKEN=$(cat .secrets/railway-workspace-token)

# Example: Set cron schedule
curl -s -X POST https://backboard.railway.com/graphql/v2 \
  -H "Authorization: Bearer $RAILWAY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation($sid:String!,$eid:String!,$input:ServiceInstanceUpdateInput!){serviceInstanceUpdate(serviceId:$sid,environmentId:$eid,input:$input)}","variables":{"sid":"<SERVICE_ID>","eid":"986bee12-1028-4016-aa42-ba0a174233b4","input":{"cronSchedule":"0 */6 * * *"}}}'

# Example: Update service config (root directory, dockerfile, etc.)
curl -s -X POST https://backboard.railway.com/graphql/v2 \
  -H "Authorization: Bearer $RAILWAY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation($sid:String!,$eid:String!,$input:ServiceInstanceUpdateInput!){serviceInstanceUpdate(serviceId:$sid,environmentId:$eid,input:$input)}","variables":{"sid":"<SERVICE_ID>","eid":"986bee12-1028-4016-aa42-ba0a174233b4","input":{"rootDirectory":"/","dockerfilePath":"Dockerfile.dashboard"}}}'

# Example: Trigger deploy
curl -s -X POST https://backboard.railway.com/graphql/v2 \
  -H "Authorization: Bearer $RAILWAY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation($sid:String!,$eid:String!){serviceInstanceDeployV2(serviceId:$sid,environmentId:$eid)}","variables":{"sid":"<SERVICE_ID>","eid":"986bee12-1028-4016-aa42-ba0a174233b4"}}'

# Service IDs:
# rtp-dashboard:        f44e64aa-81d0-429d-b3e5-605d72ef2778
# rtp-devnet-loop:      (check via CLI: railway service list)
# rtp-night-shift:      (check via CLI: railway service list)
# rtp-fee-crank:        (check via CLI: railway service list)
# rtp-promote-strategy: (check via CLI: railway service list)
```

### Legacy GitHub Actions (paused)

All 4 GitHub Actions workflows have push/PR triggers commented out (`workflow_dispatch` only):
- `swarm-ci.yml` — cargo build + test + clippy + fmt + anchor build
- `deploy-dashboard.yml` — was GitHub Pages deploy (now superseded by Railway SSR)
- `night_shift.yml` — cron at 14:00 UTC (now runs on Railway)
- `devnet-loop.yml` — cron every 6h (now runs on Railway)

### Key Notes

- **Binance geo-blocked on GitHub runners** — OHLCV data in `data/ohlcv/`, fetch defaults to `false`. Same constraint applies on Railway (night-shift uses `--skip-fetch`).
- **Droid-Shield** blocks pushes from AI agents (false positives on Solana pubkeys). Manual push required after commits.
- **Railway dashboard root directory must be `/`** — all Dockerfiles reference paths relative to repo root. Setting root to `/dashboard` breaks the build because `sdk/` is outside `dashboard/`.
- **Railway workspace API token** — stored locally at `.secrets/railway-workspace-token` (gitignored). Use `RAILWAY_TOKEN=$(cat .secrets/railway-workspace-token)` for GraphQL mutations. If missing, regenerate at `railway.com/account/tokens`.
- **Never use `railway up` for redeployment** — it wipes custom domain registrations. Use `railway redeploy --yes` or Railway dashboard redeploy instead.

---

## GitHub

- **This repo**: `git@github.com:tradewife/resilient-token-protocol.git`
- **Source repo**: `git@github.com:tradewife/fractal-swarm.git` (Python fractal-swarm origin)
- **Research repo**: `git@github.com:tradewife/rtp-skills-research.git`

---

## Design Decisions

- **Flash Trade for execution**: on-chain Solana perps DEX, CPI via invoke_signed, no cross-chain bridge, no human keypair, fully auditable on Explorer
- **Treasury PDA for signing**: invoke_signed with PDA seeds — the program IS the only authority, no private key exists
- **Hyperliquid (archived)**: legacy execution path gated behind `#[cfg(feature = "hyperliquid")]`, not compiled by default
- **Median OOS Sharpe** (not mean) — prevents single-fold outliers dominating
- **Per-fold Sharpe winsorized at ±100** — prevents tiny-sample extremes
- **Fragility is a penalty, not rejection** — `survivor *= 1/(1+fragility)`
- **Wings never modify each other directly** — all cross-wing communication via Coordinator
- **Python ↔ Rust interface is typed JSON** — any wing can propose, any wing can act
