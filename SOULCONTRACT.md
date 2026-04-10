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
5. **No SOL liquidation** — yield flows are USDC-only. SOL reserves are never liquidated. Hyperliquid positions are USDC-margined.
6. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity. No downgrade path.
7. **Soulcontract amendments require human signature + 24h monitoring** — no autonomous self-modification of governance.
8. **Auto-rollback on degradation** — if performance drops > 5% post-amendment, rollback is automatic.
9. **Self-hydration gated on runway** — ops funding only if sustenance bucket covers > 90-day runway.
10. **Strategies remain black-boxed** — the yield brain is a competitive moat; strategy configs and research internals are not exposed on-chain or in public interfaces.

---

## Execution Constraints (Enforced by Soulguard)

These apply specifically to the Hyperliquid perps execution path:

- **Max position size**: 20% of treasury reserves per trade (enforced in Trading Wing)
- **USDC-margined only**: no cross-margin, no SOL-margined positions
- **Soulguard-gated**: every ExecutePermit payload is validated by soulguard.rs before the Hyperliquid API call is made
- **Audit Wing approval required** for new strategy configs before first live execution
- **Phantom signing only**: no raw private key usage; all order signing via Phantom Connect agentic wallet flow

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
| **Swarm runtime** | `coordinator/soulguard.rs` validates every message against invariants |
| **Execution** | Trading Wing enforces position limits before every Hyperliquid API call |
| **This document** | Constitutional reference for all LLM sessions, code reviews, and agent prompts |

> Any code that could violate these invariants — even in edge cases — is a CRITICAL finding.
> See `docs/CODEREVIEW.md` for review protocol.

---

## Key Links

| Resource | URL |
|----------|-----|
| Hyperliquid API | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Rust SDK | https://github.com/hyperliquid-dex/hyperliquid-rust-sdk |
| Phantom Connect | https://docs.phantom.app/phantom-connect/introduction |
| Treasury program | `rtp/programs/rtp-treasury/` |
| Soulguard enforcement | `rtp/swarm/src/coordinator/soulguard.rs` |
