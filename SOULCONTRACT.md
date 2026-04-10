# Soulcontract

The soulcontract defines the immutable governance invariants of the Resilient Token Protocol.
Amendments require a human signature and a 24-hour monitoring window before taking effect.

## Constitutional Invariants

1. **PDA owns treasury** — no private key risk. The treasury is controlled exclusively by the program-derived address.
2. **TransferFeeConfig immutable** — fee configuration cannot be revoked after mint. Token adopters are protected.
3. **CPI-only transfers** — all token movements are atomic and verifiable on-chain.
4. **Agent proposes, human approves** — irreversible actions require explicit human sign-off.
5. **No SOL liquidation** — yield flows are USDC-only. SOL reserves are never liquidated.
6. **Phase transitions irreversible** — Sustenance → Ecosystem → Humanity. No downgrade path.
7. **Soulcontract amendments require human signature + 24h monitoring** — no autonomous self-modification of governance.
8. **Auto-rollback on degradation** — if performance drops > 5% post-amendment, rollback is automatic.
9. **Self-hydration gated on runway** — ops funding only if sustenance bucket covers > 90-day runway.
10. **Strategies remain black-boxed** — the yield brain is a competitive moat; source is not exposed on-chain.

## Amendment Process

1. Evolve Wing submits a proposal with full SPARC specification
2. Audit Wing runs 3-agent red-team tribunal
3. Human signature required to advance
4. 24-hour monitoring window before activation
5. Auto-rollback armed: any degradation > 5% triggers immediate revert

## Enforcement

These invariants are enforced at three layers:
- **On-chain**: Anchor program constraints, PDA authority checks, CPI guards
- **Swarm runtime**: `coordinator/soulguard.rs` validates every message against invariants
- **This document**: Constitutional reference for all LLM sessions and code reviews

> Any code that could violate these invariants — even in edge cases — is a CRITICAL finding.
> See `docs/CODEREVIEW.md` for review protocol.
