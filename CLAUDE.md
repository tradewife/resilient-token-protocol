# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**RTP (Resilient Token Protocol)** — a Solana-native, self-funding treasury governed by a modular Rust swarm. Any token project adopts RTP — their trading fees route to the swarm, which autonomously researches, validates, and executes yield strategies — returning yield back to the project and its holders. The swarm executes validated strategies as **perpetuals trades on Hyperliquid**, signed via **Phantom Connect** (agentic wallet). Yield (USDC) flows back to the Solana treasury PDA on devnet.

**Hackathon**: SWARMs / Canteen × Colosseum, deadline May 11, 2026.
**License**: MIT

---

## Execution Venue — Complete

The Hyperliquid perps execution path is fully implemented. BUY→fill→SELL→fill→PnL round-trip verified from Rust. Yield deposits to treasury PDA confirmed on devnet.

```
Night Shift (Python, DONE)
  └── validated strategy: SOL/USDT Survivor 2.69, signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5
        │
        ▼ bridge.rs (DONE)
Trading Wing (Rust, DONE)
  └── ExecutePermit payload → EIP-712 signed HL order
        │
        ▼ Hyperliquid REST API (DONE)
           POST https://api.hyperliquid-testnet.xyz/exchange
           Signed via ETH keypair (EIP-712)
        │
        ▼ fill confirmed → YieldReport with PnL (DONE)
        │
        ▼ Treasury CPI transfer (DONE)
           USDC → Treasury PDA via transfer_checked on devnet
        │
        ▼ check_redistribute on-chain (DONE on devnet)
        │
        ▼ Devnet loop daemon (DONE)
           6h cron, LLM-driven strategy evolution, auditable trail
```

### Integration Resources
| Resource | URL |
|----------|-----|
| Hyperliquid API docs | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Python SDK | https://github.com/hyperliquid-dex/hyperliquid-python-sdk |
| Hyperliquid Rust SDK | https://github.com/hyperliquid-dex/hyperliquid-rust-sdk |
| Testnet endpoint | https://api.hyperliquid-testnet.xyz/exchange |
| Phantom Connect docs | https://docs.phantom.com/phantom-connect |
| CASH stablecoin docs | https://docs.phantom.com/phantom-connect |

### Signing Architecture
- **HL order signing**: ETH keypair directly (`configs/hl_testnet_key.json`), EIP-712
- **MCP bridge signing**: Phantom MCP server subprocess (`@phantom/mcp-server`) — fee-free swaps, Relay cross-chain bridge to HL
- **Solana CPI signing**: Phantom KMS (production) → local devnet keypair (demo)
- **Signing cascade**: Phantom MCP → Phantom KMS → `~/.config/solana/id.json` → manual fallback

---

## Repo Layout

This repo has three layers:
1. **Proven Python fractal-swarm** (shipping) — backtesting, optimization, paper trading
2. **Rust swarm + Solana treasury** (built, 307 tests) — 6-wing architecture, Coordinator, soulcontract
3. **Hyperliquid execution** (done — devnet verified) — Trading Wing → HL testnet → yield → treasury PDA

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
cd rtp/swarm && cargo run --bin rtp-demo      # full 8-step demo + MCP bridge
cd rtp/swarm && cargo test --lib trading::tests
cd rtp/swarm && cargo test --lib trading::phantom_mcp::tests  # MCP integration
cd rtp/swarm && cargo test --lib audit::tests
cd rtp/swarm && cargo test --test coordinator_integration
```

### Solana (Treasury Program)

```bash
cd rtp/programs/rtp-treasury && anchor build
cd rtp/programs/rtp-treasury && anchor test --provider.cluster devnet
cd rtp/programs/rtp-treasury && anchor deploy --provider.cluster devnet
```

---

## Architecture

### Three-Layer Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                    ON-CHAIN (Solana / Anchor)                    │
│  Treasury PDA: fees → yield → redistribute → self-hydrate       │
│  Phase evolution: Sustenance → Ecosystem → Humanity Fund        │
├─────────────────────────────────────────────────────────────────┤
│                    SWARM RUNTIME (Rust)                          │
│  Coordinator → message bus → 6 wings (trading, security,        │
│  evolve, knowledge, audit, futureproof)                          │
│  Trading Wing → Hyperliquid perps → USDC yield → treasury PDA   │
│  Signed via Phantom Connect agentic wallet                       │
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
| `rtp/swarm/src/wings/trading/mod.rs` | **Trading Wing — HL execution, PnL tracking, apply_mutations, MCP bridge** |
| `rtp/swarm/src/wings/trading/types.rs` | Trading types — HlSignature, StrategyConfig, PositionState, TradingWing |
| `rtp/swarm/src/wings/trading/phantom_mcp.rs` | **Phantom MCP client — subprocess MCP server, fee-free swaps, HL bridge, perps reads** |
| `rtp/swarm/src/bin/rtp-daemon.rs` | **Devnet loop daemon — single-cycle, 6h cron, LLM evolution** |
| `rtp/swarm/src/wings/security/mod.rs` | Threat detection, rate-limiting, suspicious-proposal detection |
| `rtp/swarm/src/wings/evolve/` | Assessor, proposer, rollback (complete, tested) |
| `rtp/swarm/src/wings/knowledge/mod.rs` | In-memory knowledge graph, cross-wing queries |
| `rtp/swarm/src/wings/audit/mod.rs` | 3-agent tribunal (Skeptic/UserProxy/Optimizer), Byzantine consensus |
| `rtp/swarm/src/wings/futureproof/mod.rs` | Deprecation monitoring, heartbeat |

#### Solana (Treasury Program)

| File | Purpose |
|------|---------|
| `rtp/programs/rtp-treasury/` | Anchor: withdraw_fees, check_redistribute, hydrate_swarm, evolve_phase |

#### Governance

| File | Purpose |
|------|---------|
| `SOULCONTRACT.md` | Constitutional governance — invariants, execution constraints, key links |
| `SESSION-CONTEXT.md` | Compressed project memory — paste into every fresh session |
| `docs/RESOURCES.md` | All hackathon links, SDK links, sponsor links |
| `docs/SECURITY_AUDIT_2026-04-07.md` | Full security audit — 18 findings |
| `docs/CODEREVIEW.md` | Code review protocol |
| `docs/demo-flow.md` | 3-minute hackathon demo script |

---

## Devnet Limitations

### Phantom Wallet Perps Bridge (SOL → HL USDC) — Mainnet-Only

The Phantom wallet native perps bridge that converts SOL to USDC for Hyperliquid
perpetual trading operates **on mainnet only**. Phantom's Testnet Mode supports
basic SOL transactions and dApp connections, but the specific integration for
bridging to and trading on Hyperliquid relies on mainnet liquidity pools.

**Impact on RTP:** The Trading Wing cannot route SOL from the treasury through
Phantom's bridge to fund the HL perps account on devnet.

**Devnet workaround:** `devnet_fund_stub()` in `trading/mod.rs` simulates the
SOL→USDC conversion at the current oracle price (fetched from HL testnet).
It applies a 0.3% bridge fee (realistic swap cost) and returns the simulated
USDC amount. This function is gated behind `#[cfg(feature = "devnet")]` and
is never compiled for the mainnet binary.

```bash
# Run tests with devnet stub (310 tests):
cd rtp/swarm && cargo test --lib --features devnet

# Run without devnet stub (307 tests, production config):
cd rtp/swarm && cargo test --lib
```

**Production path (mainnet):** The Phantom perps bridge will work natively.
`devnet_fund_stub()` is excluded from compilation when the `devnet` feature
is not set, so the mainnet binary remains clean.

---

## Key Invariants (enforced on-chain)

1. **PDA owns treasury** — no private key risk
2. **Per-token isolation** — each mint gets its own Treasury PDA + vault, no shared pool, no honeypot
3. **SPL TransferFeeConfig immutable from mint** — fee percentage and withdraw authority cannot be revoked. Platform-level fee routing varies (Pump.fun: one-time, Bags.fm: anytime, Raydium: manual).
4. **CPI-only transfers** — atomic, verifiable
5. **No SOL liquidation** — USDC-only yield flows; Hyperliquid positions are USDC-margined
6. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity
7. **Auto-rollback if performance degrades > 5% post-amendment**
8. **Self-hydration only if sustenance bucket > 90-day runway**
9. **Research code remains reviewable while collaboration is active**

## Trust Model — Permissionless Recording, Authority-Gated Actions

The on-chain program separates instructions into two categories:

**Authority-gated (treasury.authority required):**
- `initialize` — creates treasury, sets authority/wallets/runway
- `evolve_phase` — irreversible phase transitions, authority checked via Anchor constraint
- `register_strategy` — promotes strategy to Live status
- `force_retire_strategy` — emergency strategy retirement
- `end_beta` — manual beta adopter sunset
- `create_swarm_vault` — creates hydration vault (anyone can pay, but treasury PDA is authority)

**Permissionless (any signer can call):**
- `withdraw_fees` — anyone can pull TransferFeeConfig fees INTO the PDA vault (not out)
- `check_redistribute` — anyone can trigger 70/20/10 split (deterministic, no discretion)
- `hydrate_swarm` — anyone can propose hydration (gated by strategy Live status + beta check + runway invariant)
- `register_adopter` / `register_adopter_beta` — anyone can create an adopter record (caller pays rent)
- `record_fee_deposit` — anyone can record fee accounting (no fund movement, just counters)
- `update_strategy_performance` — anyone can write strategy metrics (enforcement is on-chain via hydrate_swarm gate)
- `verify_adoption` — read-only verification

**Why this is safe:** Permissionless instructions either move funds INTO the PDA (never out), record accounting state (no fund movement), or write metrics where the real enforcement happens via authority-gated on-chain checks. The PDA owns all treasury assets — no private key can sign them away. Cumulative counters use `saturating_add` (never panics, never overflows to wrong values).

**Known mainnet considerations (accepted for launch, post-launch improvements):**
- `evolve_phase` thresholds checked against raw vault balance, not oracle-denominated USD. Authority manually verifies reserves before calling. Post-launch: integrate Pyth/Switchboard oracle.
- `check_redistribute` emits a `Redistribution` event for auditability (added Apr 2026).

---

## Sponsored Hackathon Resources

| Sponsor | Use in RTP | Link |
|---------|-----------|------|
| Phantom Connect | **Phantom Portal app registered**. `@solana/wallet-adapter-react` wired to dashboard (/, /launch, /docs). Wallet connect + live token launch flow operational on devnet. MCP server for AI agent wallet ops. | https://docs.phantom.com/introduction |
| CASH stablecoin | **Treasury yield settlement currency** | https://docs.phantom.com/phantom-connect |
| Squads Multisig | Treasury PDA security (production path) | https://docs.squads.so |
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
This is the config the Trading Wing targets on Hyperliquid.

---

## CI/CD

- **Night shift**: GitHub Actions cron at 14:00 UTC (`night_shift.yml`, 300 min timeout) — also runs Python module import + CLI tests
- **Swarm CI**: `swarm-ci.yml` — cargo build + test + clippy + fmt + anchor build
- **Devnet loop**: `devnet-loop.yml` — cron every 6h + manual dispatch, runs rtp-daemon, commits cycle output
- **Binance geo-blocked on GitHub runners** — OHLCV data in `data/ohlcv/`, fetch defaults to `false`
- **All workflow push/PR triggers currently paused** (workflow_dispatch only) to conserve Actions minutes. Re-enable `swarm-ci.yml` push trigger before May 11 submission for one final green CI run.
- **Dashboard deployment is manual** — `deploy-dashboard.yml` push triggers are commented out. After merging dashboard changes, run `gh workflow run deploy-dashboard.yml --ref main` to deploy to GitHub Pages (resilientprotocol.xyz). The site does NOT auto-deploy on push.

---

## GitHub

- **This repo**: `git@github.com:tradewife/resilient-token-protocol.git`
- **Source repo**: `git@github.com:tradewife/fractal-swarm.git` (Python fractal-swarm origin)
- **Research repo**: `git@github.com:tradewife/rtp-skills-research.git`

---

## Design Decisions

- **Hyperliquid for execution**: highest-liquidity perps DEX, REST API, USDC-margined, no KYC
- **Phantom for signing**: sponsored, agentic wallet flow, CASH stablecoin settlement
- **Median OOS Sharpe** (not mean) — prevents single-fold outliers dominating
- **Per-fold Sharpe winsorized at ±100** — prevents tiny-sample extremes
- **Fragility is a penalty, not rejection** — `survivor *= 1/(1+fragility)`
- **Wings never modify each other directly** — all cross-wing communication via Coordinator
- **Python ↔ Rust interface is typed JSON** — any wing can propose, any wing can act
