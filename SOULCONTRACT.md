# Soulcontract

The soulcontract defines the immutable governance invariants of the Resilient Token Protocol.
Amendments require a human signature and a 24-hour monitoring window before taking effect.

> **Execution venue:** The Trading Wing executes validated yield strategies as on-chain perpetuals via **Flash Trade CPI** (Solana), signed by the **Treasury PDA via `invoke_signed`** (no human keypair). This venue is a swarm-level decision, not a constitutional invariant — it can be changed by the Evolve Wing with Audit Wing approval. The invariants below govern *how* execution happens, not *where*.

---

## Constitutional Invariants

1. **PDA owns treasury** — no private key risk. The treasury is controlled exclusively by the program-derived address.
2. **Per-token isolation** — each adopting mint gets its own Treasury PDA and vault (`seeds: ["treasury", mint]`). No shared pool exists. One token's exploit cannot affect another's reserves.
3. **TransferFeeConfig immutable** — fee percentage and withdraw authority cannot be revoked after mint. Platform-level fee routing varies: Pump.fun allows one-time redirect, Bags.fm supports anytime updates, Raydium requires manual forwarding.
4. **CPI-only transfers** — all on-chain token movements are atomic and verifiable.
5. **No SOL liquidation** — SOL reserves are committed to Flash Trade positions via on-chain CPI (Composability swap-and-open). The treasury never sells SOL on the open market. Positions are on Solana, fully auditable. SOL on the treasury PDA is never at risk of liquidation on an external chain.
6. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity. No downgrade path.
7. **Auto-rollback on degradation** — if performance drops > 5% post-amendment, rollback is automatic.
8. **Self-hydration gated on runway** — ops funding only if sustenance bucket covers > 90-day runway.
9. **Strategies remain black-boxed** — the yield brain is a competitive moat; strategy configs and research internals are not exposed on-chain or in public interfaces.
10. **Emergency freeze** — authority can halt all treasury operations instantly via `freeze_treasury`. All 14 state-mutating instructions check the frozen flag (12 original + 2 Flash Trade: `open_flash_position`, `close_flash_position`). `emergency_close_all_positions` is intentionally exempt so the freeze-then-unwind flow works. Freeze/unfreeze events emitted on-chain for audit.
11. **Zero-address rejection** — `Pubkey::default()` is rejected on all critical fields (authority, mint, wallet addresses). No misconfiguration attacks.

---

## Treasury Capital Model — On-Chain SOL Cycle

The protocol operates on a single transparent cycle:

```
SOL in → Treasury PDA invoke_signed → Flash Trade CPI (on-chain) → SOL returned → Treasury PDA
```

### Core Invariant

> **Each adopting token's transfer fees flow to its per-mint treasury vault PDA.** The swarm trades yield strategies via Flash Trade CPI (on-chain Solana). Yield returns as SOL to the treasury for redistribution.

### Capital Flow

| Step | Asset | Location | Mechanism |
|------|-------|----------|-----------|
| 1. Fees arrive | SOL | Treasury PDA (Solana) | Platform creator fees (Pump.fun, Bags.fm, Raydium) → treasury PDA |
| 2. Execute strategies | SOL | Flash Trade (on-chain CPI) | Treasury PDA invoke_signed → Composability swap-and-open → perps position on Solana |
| 3. Yield returns | SOL | Treasury PDA (Solana) | Close position via CPI → SOL returned to treasury vault |
| 4. Redistribute | SOL | Treasury PDA | 70% holders / 20% dev / 10% ecosystem (on-chain) |

### Why This Model

- **Per-mint isolation**: each adopting token has its own treasury PDA and vault. Judges can verify any token's treasury balance on Solana Explorer.
- **Single chain**: Flash Trade positions are on Solana. No cross-chain bridge. No USDC in-flight.
- **PDA-signed execution**: the Treasury PDA signs via `invoke_signed`. No human keypair exists for trading.
- **Auditable**: every position open/close is an on-chain transaction visible on Solana Explorer.

### Devnet Note

Flash Trade uses **Pyth Network** oracles which are **mainnet only**. Devnet returns stale/zero prices, causing `StaleOraclePrice` on all position operations. CPI testing happens on mainnet with micro positions (~$11-12 USDC minimum) or on local validator for constraint logic only. See `FLASHTRADE-PDA-UPGRADE-SPEC.md` for M1 mainnet proofs.

### Self-Sustaining Threshold

When SOL yield exceeds ops cost by 10× sustained over 90 days, external seed capital is no longer required and can be returned or recycled to the ecosystem fund.

---

## Execution Constraints (Enforced by Soulguard)

These apply specifically to the Flash Trade CPI execution path:

- **Max position size**: 20% of treasury reserves per trade (enforced in both Trading Wing and on-chain `open_flash_position`)
- **Max concurrent positions**: 3 per strategy (enforced on-chain via `StrategyRecord.open_position_count`)
- **SOL input via Composability**: swap-and-open is atomic in a single Solana transaction
- **Soulguard-gated**: every ExecutePermit payload is validated by soulguard.rs before the CPI instruction is built
- **Audit Wing approval required** for new strategy configs before first live execution
- **Treasury PDA signing**: all execution via `invoke_signed` with Treasury PDA seeds — no human keypair involved. The fee-payer wallet pays gas only (< 0.001 SOL/tx) and has zero authority over treasury funds.

### Strategy Lifecycle Governance (Automated)

No strategy goes live or stays live without clearing codified gates. All thresholds are defined in `research/promotion_criteria.py` — no magic numbers elsewhere.

**Promotion** (RESEARCH → PAPER_TRADING → LIVE):
- OOS Sharpe ≥ 1.5 across ≥ 3 profitable folds
- ≤ 40% IS/OOS degradation, ≥ 45% win rate, PF ≥ 1.3, DD ≤ 20%
- Profitable in ≥ 2 of 3 regimes (trending/ranging/high-vol)
- ≥ 72h paper trading confirmation
- Rolling correlation < 0.4 with existing live strategies
- ≥ 2 of 3 validator agents approve

**Retirement** (LIVE → SUSPENDED or RETIRED):
- **Hard stops** (immediate suspension): 10% 24h drawdown, 5 consecutive losses, rolling Sharpe < 0.5
- **Soft decay** (3 strikes = retire): Sharpe drops below 50% of promotion Sharpe, win rate < 38%, regime mismatch > 5 days, funding rate below floor, correlation creep > 0.6

**Decay monitoring** uses risk-adjusted rolling windows: LOW (45 days), MEDIUM (30 days), HIGH (14 days) — matching the strategy's `decay_risk` classification in `strategy_library.md`.

---

## Amendment Process

1. Evolve Wing submits a proposal with full SPARC specification
2. Audit Wing runs 3-agent red-team tribunal (Skeptic / UserProxy / Optimizer)
3. Human signature required to advance
4. 24-hour monitoring window before activation
5. Auto-rollback armed: any degradation > 5% triggers immediate revert

---

## Enforcement Layers

| Layer | Mechanism |
|-------|-----------|
| **On-chain** | Anchor program constraints, PDA authority checks, CPI guards |
| **On-chain lifecycle** | `StrategyRecord` PDA — hydrate_swarm requires Live status; hard stops auto-suspend; 3 soft strikes auto-retire |
| **Swarm runtime** | `coordinator/soulguard.rs` validates every message against invariants |
| **Execution** | Trading Wing enforces position limits before every Flash Trade CPI submission; on-chain `open_flash_position` re-checks the 20% cap, runway floor, and max-3 concurrent positions |
| **Lifecycle** | `promotion_criteria.py` gates strategy promotion and retirement; `DecayMonitor` tracks live performance against hard/soft thresholds |
| **This document** | Constitutional reference for all LLM sessions, code reviews, and agent prompts |

> Any code that could violate these invariants — even in edge cases — is a CRITICAL finding.
> See `docs/CODEREVIEW.md` for review protocol.

---

## Key Links

| Resource | URL |
|----------|-----|
| Flash Trade REST API | https://flashapi.trade |
| Flash Trade SKILL.md | `flash-trade/SKILL.md` (in repo) |
| Flash Trade Program (mainnet) | `FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn` |
| Phantom Connect | https://docs.phantom.com/phantom-connect |
| MoonPay Agents | https://www.moonpay.com/developers/agents |
| Treasury program | `rtp/programs/rtp-treasury/` |
| Soulguard enforcement | `rtp/swarm/src/coordinator/soulguard.rs` |
| Flash Trade CPI spec | `FLASHTRADE-PDA-UPGRADE-SPEC.md` |
