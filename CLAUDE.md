# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**RTP (Resilient Token Protocol)** — a Solana-native, self-funding treasury governed by a modular Rust swarm. Any token project adopts RTP — their trading fees route to the swarm, which autonomously researches, validates, and executes yield strategies — returning yield back to the project and its holders. The swarm executes validated strategies as **perpetuals trades on Hyperliquid**, signed via **Phantom Connect** (agentic wallet). Yield (USDC) flows back to the Solana treasury PDA on devnet.

**Hackathon**: SWARMs / Canteen × Colosseum, deadline May 11, 2026.
**License**: MIT

---

## Execution Venue — Critical Path

The **#1 unimplemented gap** is the Hyperliquid perps execution path. Everything else is built. This is what ships next.

```
Night Shift (Python, DONE)
  └── validated strategy: SOL/USDT Survivor 2.69, signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5
        │
        ▼ bridge.rs (DONE)
Trading Wing (Rust, PARTIAL — in-memory mock only)
  └── ExecutePermit payload → needs reqwest + Hyperliquid order struct
        │
        ▼ Hyperliquid REST API (NOT IMPLEMENTED)
           POST https://api.hyperliquid.xyz/exchange
           Signed via Phantom Connect agentic wallet
        │
        ▼ fill confirmed → USDC yield → Treasury PDA (NOT IMPLEMENTED)
        │
        ▼ check_redistribute on-chain (DONE on devnet)
```

### Hyperliquid Integration Resources
| Resource | URL |
|----------|-----|
| Hyperliquid API docs | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Python SDK | https://github.com/hyperliquid-dex/hyperliquid-python-sdk |
| Hyperliquid Rust SDK | https://github.com/hyperliquid-dex/hyperliquid-rust-sdk |
| Testnet endpoint | https://api.hyperliquid-testnet.xyz/exchange |
| Phantom Connect docs | https://docs.phantom.app/phantom-connect/introduction |
| CASH stablecoin docs | https://docs.phantom.app/phantom-connect/cash |

### What to build in Trading Wing (`rtp/swarm/src/wings/trading/mod.rs`)
1. Add `reqwest` + `serde_json` to `rtp/swarm/Cargo.toml`
2. Define `HyperliquidOrder` struct (asset, isBuy, limitPx, sz, orderType)
3. In `handle_execute_permit()`: construct order from `TradingConfig` payload, POST to HL testnet
4. Parse fill response → emit `YieldReport` with realized PnL
5. CPI transfer: yield USDC → treasury PDA via `transfer_checked`
6. Phantom signing: use Phantom Connect agentic wallet API for order signing (see docs above)

---

## Repo Layout

This repo has three layers:
1. **Proven Python fractal-swarm** (shipping) — backtesting, optimization, paper trading
2. **Rust swarm + Solana treasury** (built, 205 tests) — 6-wing architecture, Coordinator, soulcontract
3. **Hyperliquid execution** (critical gap) — Trading Wing → HL testnet → yield → treasury PDA

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
cd rtp/swarm && cargo test --lib trading::tests
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
| `rtp/swarm/src/wings/trading/mod.rs` | **Trading Wing — ExecutePermit handler. Needs Hyperliquid REST wiring.** |
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
| `BUILD_PLAN_v3.md` | Post-audit schedule (active) |
| `docs/RESOURCES.md` | All hackathon links, SDK links, sponsor links |
| `docs/SECURITY_AUDIT_2026-04-07.md` | Full security audit — 18 findings |
| `docs/CODEREVIEW.md` | Code review protocol |
| `docs/demo-flow.md` | 3-minute hackathon demo script |

---

## Key Invariants (enforced on-chain)

1. **PDA owns treasury** — no private key risk
2. **SPL TransferFeeConfig immutable from mint** — fees cannot be revoked
3. **CPI-only transfers** — atomic, verifiable
4. **Agent proposes, human approves irreversible actions**
5. **No SOL liquidation** — USDC-only yield flows; Hyperliquid positions are USDC-margined
6. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity
7. **Soulcontract amendments require human signature + 24h monitoring**
8. **Auto-rollback if performance degrades > 5% post-amendment**
9. **Self-hydration only if sustenance bucket > 90-day runway**
10. **Research code remains reviewable while collaboration is active**

---

## Sponsored Hackathon Resources

| Sponsor | Use in RTP | Link |
|---------|-----------|------|
| Phantom Connect | **Agentic wallet signing for Hyperliquid orders** | https://docs.phantom.app/phantom-connect/introduction |
| CASH stablecoin | **Treasury yield settlement currency** | https://docs.phantom.app/phantom-connect/cash |
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
| SOL/USDT | +36.9% | **+118.3%** | 78% | 429 |
| BNB/USDT | +49.6% | — | 67% | 178 |
| ETH/USDT | +48.1% | — | 78% | 155 |
| BTC/USDT | +17.5% | — | 67% | 153 |

Active symbols: BTC/USDT, ETH/USDT, SOL/USDT, BNB/USDT. XRP dropped (net negative).

**Top live candidate (Apr 9 Night Shift):**
SOL/USDT Survivor 2.69 — signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop_atr=0.5
This is the config the Trading Wing targets on Hyperliquid.

---

## CI/CD

- **Night shift**: GitHub Actions cron at 14:00 UTC (`night_shift.yml`, 300 min timeout)
- **Swarm CI**: `swarm-ci.yml` — cargo build + test + clippy + fmt + anchor build
- **Python tests**: `python-tests.yml` — module imports + CLI help + bridge-mode schema
- **Binance geo-blocked on GitHub runners** — OHLCV data in `data/ohlcv/`, fetch defaults to `false`

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
