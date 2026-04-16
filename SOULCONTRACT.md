# Soulcontract

The soulcontract defines the immutable governance invariants of the Resilient Token Protocol.
Amendments require a human signature and a 24-hour monitoring window before taking effect.

> **Execution venue:** The Trading Wing executes validated yield strategies as perpetuals trades on **Hyperliquid**, signed via **Phantom Connect** (agentic wallet). Yield (USDC) flows back to the Solana treasury PDA. This venue is a swarm-level decision, not a constitutional invariant — it can be changed by the Evolve Wing with Audit Wing approval. The invariants below govern *how* execution happens, not *where*.

---

## Constitutional Invariants

1. **PDA owns treasury** — no private key risk. The treasury is controlled exclusively by the program-derived address.
2. **TransferFeeConfig immutable** — fee configuration cannot be revoked after mint. Token adopters are protected.
3. **CPI-only transfers** — all on-chain token movements are atomic and verifiable.
4. **Agent proposes, human approves** — irreversible actions require explicit human sign-off.
5. **No SOL liquidation** — SOL reserves are never sold on the open market. The Phantom bridge converts SOL↔USDC trustlessly; the treasury never sells SOL to fund operations. Hyperliquid positions are USDC-margined. SOL on the treasury PDA is never at risk of liquidation.
6. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity. No downgrade path.
7. **Soulcontract amendments require human signature + 24h monitoring** — no autonomous self-modification of governance.
8. **Auto-rollback on degradation** — if performance drops > 5% post-amendment, rollback is automatic.
9. **Self-hydration gated on runway** — ops funding only if sustenance bucket covers > 90-day runway.
10. **Strategies remain black-boxed** — the yield brain is a competitive moat; strategy configs and research internals are not exposed on-chain or in public interfaces.

---

## Treasury Capital Model — Unified SOL Flow

The protocol operates on a single transparent cycle:

```
SOL in → USDC (Phantom bridge) → trade on Hyperliquid → USDC yield → SOL (Phantom bridge) → treasury PDA
```

### Core Invariant

> **Each adopting token's transfer fees flow to its per-mint treasury vault PDA.** The swarm trades yield strategies on Hyperliquid (USDC-margined). Yield returns to the treasury for redistribution.

### Capital Flow

| Step | Asset | Location | Mechanism |
|------|-------|----------|-----------|
| 1. Fees arrive | Token | Treasury vault PDA (Solana) | TransferFeeConfig — per-mint vault receives withheld fees |
| 2. Fund trading | SOL → USDC | Phantom bridge (mainnet) | Trustless swap at oracle price, 0.3% fee |
| 3. Execute strategies | USDC | HL clearinghouse | USDC-margined perps, EIP-712 signed |
| 4. Yield returns | USDC → SOL | Phantom bridge (mainnet) | Trustless swap at oracle price |
| 5. Redistribute | Token | Treasury vault PDA | 70% holders / 20% dev / 10% ecosystem (on-chain) |

### Why This Model

- **Per-mint isolation**: each adopting token has its own treasury PDA and vault. Judges can verify any token's treasury balance on Solana Explorer.
- **USDC only in-flight**: Hyperliquid positions are USDC-margined. SOL is never at risk of liquidation on HL.
- **Trustless conversion**: the Phantom bridge handles SOL↔USDC without custodial risk. The swarm never holds USDC off-chain.
- **Auditable**: every step produces an on-chain signature or API receipt. The full cycle is visible in demo output.

### Devnet Note

The Phantom SOL↔USDC bridge is **mainnet-only**. On devnet, the HL clearinghouse is funded directly via faucet, and the `devnet_fund_stub()` simulates the bridge conversion for demo narrative purposes. The treasury PDA holds SOL on devnet as it would in production. See `CLAUDE.md → Devnet Limitations` for details.

### Self-Sustaining Threshold

When USDC yield exceeds ops cost by 10× sustained over 90 days, external seed capital is no longer required and can be returned or recycled to the ecosystem fund.

---

## Execution Constraints (Enforced by Soulguard)

These apply specifically to the Hyperliquid perps execution path:

- **Max position size**: 20% of treasury reserves per trade (enforced in Trading Wing)
- **USDC-margined only**: no cross-margin, no SOL-margined positions
- **Soulguard-gated**: every ExecutePermit payload is validated by soulguard.rs before the Hyperliquid API call is made
- **Audit Wing approval required** for new strategy configs before first live execution
- **Phantom signing only**: no raw private key usage; all order signing via Phantom Connect agentic wallet flow

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
| **Execution** | Trading Wing enforces position limits before every Hyperliquid API call |
| **Lifecycle** | `promotion_criteria.py` gates strategy promotion and retirement; `DecayMonitor` tracks live performance against hard/soft thresholds |
| **This document** | Constitutional reference for all LLM sessions, code reviews, and agent prompts |

> Any code that could violate these invariants — even in edge cases — is a CRITICAL finding.
> See `docs/CODEREVIEW.md` for review protocol.

---

## Key Links

| Resource | URL |
|----------|-----|
| Hyperliquid API | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Rust SDK | https://github.com/hyperliquid-dex/hyperliquid-rust-sdk |
| Phantom Connect | https://docs.phantom.com/phantom-connect |
| MoonPay Agents | https://www.moonpay.com/developers/agents |
| Treasury program | `rtp/programs/rtp-treasury/` |
| Soulguard enforcement | `rtp/swarm/src/coordinator/soulguard.rs` |
