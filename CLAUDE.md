# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**RTP (Resilient Token Protocol)** — a Solana-native, self-funding treasury governed by a modular Rust swarm. Any token project adopts RTP — their trading fees route to the swarm, which autonomously researches, validates, and executes yield strategies — returning yield back to the project and its holders. The swarm executes validated strategies as **on-chain perpetuals via Flash Trade CPI**, signed by the **Treasury PDA via `invoke_signed`** (no human keypair). All execution stays on Solana — no cross-chain bridge, no off-chain signing.

**Hackathon**: SWARMs / Canteen × Colosseum, deadline May 11, 2026.
**License**: MIT

---

## Execution Venue — Complete (Flash Trade CPI)

The Flash Trade on-chain CPI execution path is fully implemented (M0–M5). PDA-signed position open/close confirmed on mainnet. The Hyperliquid/Phantom MCP path is archived behind `#[cfg(feature = "hyperliquid")]`.

```
Night Shift (Python, DONE)
  └── validated strategy: SOL/USDT Survivor 2.69, signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5
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
| Phantom Connect docs (browser wallet) | https://docs.phantom.com/phantom-connect |

**Legacy (archived behind `#[cfg(feature = "hyperliquid")]`):**
| Resource | URL |
|----------|-----|
| Hyperliquid API docs | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Python SDK | https://github.com/hyperliquid-dex/hyperliquid-python-sdk |
| Testnet endpoint | https://api.hyperliquid-testnet.xyz/exchange |

### Signing Architecture
- **Flash Trade CPI signing**: Treasury PDA `invoke_signed` — no private key exists. The program IS the only authority.
- **Fee-payer wallet**: Funded keypair for Solana gas fees only — has zero authority over treasury funds
- **Dashboard signing**: `@solana/wallet-adapter-react` for browser wallet ops (freeze/unfreeze, multisig status)
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
- **Dashboard RPC must match treasury network**: WalletProvider uses devnet RPC. The `/launch` page creates a separate mainnet connection for platform launches. Never mix networks — treasury balance reads from wrong RPC return 0.
- **Frozen state must use SDK decoder**: Never read on-chain frozen field via hardcoded byte offsets — the Anchor account layout can change. Use `fetchTreasuryState()` from the SDK which uses Borsh decoding.
- **FlashTradeClient is async**: The REST client uses async `reqwest` with retry (3 attempts, exponential backoff). Synchronous callers use `_blocking()` wrappers that create a tokio current-thread runtime.
- **No `.unwrap()` in production paths**: All msgpack encoding, ATA derivation, daemon serialization, and evolve proposals use proper error handling (`map_err`, `unwrap_or_else`, `ok_or`). Never re-introduce `.unwrap()` on code that handles external input.

---

## Repo Layout

This repo has three layers:
1. **Proven Python fractal-swarm** (shipping) — backtesting, optimization, paper trading
2. **Rust swarm + Solana treasury** (built, 325 unit + 5 integration tests) — 6-wing architecture, Coordinator, soulcontract, Flash Trade CPI execution, emergency freeze, zero-address guard
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
| `rtp/swarm/src/trader/mod.rs` | **Live autonomous trader — REST API trading via Flash Trade, Arc<Mutex<TraderState>> shared state, HTTP status server on configurable port** |
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
| Phantom Connect | **Browser wallet only**. `@solana/wallet-adapter-react` wired to dashboard (/, /launch, /docs). Wallet connect + live token launch flow operational on devnet. | https://docs.phantom.com/introduction |
| CASH stablecoin | Third-party resource (not currently used) | https://docs.phantom.com/phantom-connect |
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
SOL/USDT Survivor 2.69 — signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop_atr=0.5
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
| **rtp-trader** | Always-on | `rtp/swarm/Dockerfile.trader` | — | HTTP status server on port 8080 (Railway private networking) |

**Railway account:** katejcooper.atelier@gmail.com
**Project dashboard:** https://railway.com/project/11004852-2ba7-46d9-aeb5-ab9558e965a0
**Region:** Southeast Asia (asia-southeast1-eqsg3a)
**Environment:** production (`986bee12-1028-4016-aa42-ba0a174233b4`)

### Service Details

- **rtp-dashboard**: SSR Next.js (`output: "standalone"` in `next.config.ts`). Multi-stage Docker build from `Dockerfile.dashboard` at repo root. Auto-deploys from connected GitHub repo on push to main. **Railway config: Root Directory must be `/` (repo root), NOT `/dashboard`** — the Dockerfile needs access to both `sdk/` and `dashboard/`. Env var `RAILPACK_DOCKERFILE_PATH=Dockerfile.dashboard` tells Railway to use our Dockerfile instead of Nixpacks. The standalone build copies static assets to `.next/static` (not `dashboard/.next/static`) relative to `server.js`.
- **rtp-devnet-loop**: Rust `rtp-daemon` binary. Dockerfile uses `rust:1.88-slim` builder + `debian:bookworm-slim` runner. Connected to GitHub repo (`tradewife/resilient-token-protocol`), auto-deploys on push. Build context is repo root — COPY paths in Dockerfile use `rtp/swarm/` prefix. Needs env vars: `LLM_API_BASE_URL`, `LLM_API_KEY`, `LLM_MODEL`.
- **rtp-night-shift**: Python 3.12, installs from `requirements-ci.txt`, runs `night_shift --skip-fetch`. One-shot: runs to completion and exits. OHLCV data in `data/ohlcv/` included via `.railwayignore` exclusion.
- **rtp-swarm-ci**: Rust builder with Solana CLI + Anchor. Runs `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check`, `anchor build`. One-shot CI validation.
- **rtp-trader**: Always-on Rust binary (`rtp-trader`). Runs Survivor 2.69 strategy autonomously, polls Flash Trade every 5 minutes, executes SOL LONG positions when signal conditions met. HTTP status server on port 8080 serves `GET /state` (live TraderState JSON) and `GET /health`. State shared via `Arc<Mutex<TraderState>>` between trading loop and HTTP handler. Dashboard fetches via Railway private networking (`http://rtp-trader.railway.internal:8080/state`). Dockerfile: `rtp/swarm/Dockerfile.trader`. Env var `RTP_TRADER_HTTP_PORT` (default 8080).

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
