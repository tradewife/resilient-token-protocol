# CODEREVIEW.md

This file gives an AI agent all the context it needs to perform a code review on this repository. Read this entire file before starting. Do not make changes to the codebase unless explicitly asked.

---

## What This Project Is

**RTP (Resilient Token Protocol)** — a Solana-native self-funding treasury governed by a modular Rust swarm. Trading fees from any adopting token project route to a Treasury PDA, which is autonomously managed by a 6-wing Rust swarm. The swarm researches, validates, and executes yield strategies, returning yield to project holders.

**Hackathon deadline**: Solana Frontier (Colosseum × Canteen) — **May 11, 2026**. This is a shipping-first, deadline-critical context. Flag blockers loudly. Don't bikeshed.

### Three-Layer Architecture

```
ON-CHAIN (Solana / Anchor)
  └── Treasury PDA: fees → yield → redistribute → self-hydrate

SWARM RUNTIME (Rust)
  └── Coordinator → message bus → 6 wings
      (trading, security, evolve, knowledge, audit, futureproof)

RESEARCH LAYER (Python — gitignored, ships as binary)
  └── Night Shift: 30K configs → WFA → Darwinian → full-sim validate
```

### Key Governance Invariants (must never be broken)
1. PDA owns treasury — no private key risk
2. SPL TransferFeeConfig immutable from mint — fees cannot be revoked
3. CPI-only transfers — atomic, verifiable
4. Agent proposes, human approves irreversible actions
5. No SOL liquidation — USDC-only yield flows
6. Phase transitions irreversible: Sustenance → Ecosystem → Humanity
7. soulcontract amendments require human signature + 24h monitoring
8. Auto-rollback if performance degrades > 5% post-amendment
9. Self-hydration only if sustenance bucket > 90-day runway
10. Yield brain strategies remain black-boxed (competitive moat)

These invariants are enforced on-chain and in `SOULCONTRACT.md`. Any code that could violate them — even in edge cases — is a **CRITICAL** finding.

---

## Repo Map (What Is Tracked vs Gitignored)

**Tracked (open-source, what you can see):**
- `rtp/swarm/` — Rust swarm runtime
- `rtp/programs/rtp-treasury/` — Anchor treasury program
- `SOULCONTRACT.md` — constitutional governance layer
- `CLAUDE.md` — full project context and architecture
- `ONBOARDING.md` — current build state + next-session guide
- `BUILD_PLAN_v3.md` — active milestone tracker
- `SESSION-CONTEXT.md` — canonical current state, open decisions, blockers
- `docs/SECURITY_AUDIT_2026-04-07.md` — last security audit (18 findings, most fixed)
- `docs/RESOURCES.md` — all resource links for development and hackathon
- `.github/workflows/` — CI/CD config

**Gitignored (local dev, ships as binary):**
- `scripts/` — Python night shift, paper trader, calibration
- `backtesting/` — FutureBlindSimulator
- `agents/` — data collectors
- `data/` — OHLCV parquets, night results (exception: `data/ohlcv/` and `data/night_results/` are now tracked)

---

## Current Build State (as of 2026-04-11)

### Completed
- ✅ All CRITICAL findings fixed (C-1, C-2, C-3)
- ✅ All HIGH findings fixed (H-1 through H-5)
- ✅ Most MEDIUM findings fixed (M-1, M-3, M-4, M-5)
- ✅ Invariant 7 closed — ed25519 sig check documented as production TODO in soulguard.rs
- ✅ Rust swarm: 238/238 tests passing
- ✅ Treasury: compiles clean, BPF `.so` produced
- ✅ Anchor integration tests passing
- ✅ Night shift CI pipeline: running nightly (cron 14:00 UTC / midnight AEST)
- ✅ YieldReport reframed — source: wfa_backtest, projected not realized
- ✅ demo.sh narrative loop closed — bridge assessment → PROJECTED_YIELD → check_redistribute on devnet
- ✅ Paper trader state printed in Layer 1
- ⚠️ anchor deploy pending — devnet faucet rate-limited, needs SOL funding

### In Progress
- 🔧 Devnet deploy — program built, awaiting SOL airdrop
- 🔧 Demo video recording

---

## How to Run a Code Review

### What to Review

Perform a **code review** (not a full audit — a targeted review of code quality, correctness, and risk) covering:

1. **Rust swarm** (`rtp/swarm/src/`) — focus on:
   - Coordinator logic (`coordinator/mod.rs`, `router.rs`, `soulguard.rs`, `lifecycle.rs`)
   - Evolve wing (`wings/evolve/`) — this is the most complete wing
   - Audit wing (`wings/audit/mod.rs`) — Byzantine consensus, 3-agent tribunal
   - Types (`types.rs`) — correctness of message/payload definitions

2. **Anchor treasury program** (`rtp/programs/rtp-treasury/`) — focus on:
   - All previously patched invariants — verify patches are solid
   - Any remaining attack surface (reentrancy, authority checks, arithmetic overflow)
   - CPI safety

3. **CI/CD** (`.github/workflows/night_shift.yml`) — focus on:
   - Secret handling, correctness of the rebase-before-push logic
   - Any failure modes that would silently pass

### What NOT to Review
- Python scripts (gitignored, black-boxed — not in scope)
- `data/` directory
- Documentation files (`.md`) — unless you spot a spec contradiction with the code

### Review Depth
This is an **incremental review**, not a full audit. The last full security audit is at `docs/SECURITY_AUDIT_2026-04-07.md`. Focus on:
- Changes since that audit (commits after `de35f261`)
- Code quality, logic bugs, and edge cases in stubbed vs live paths
- Anything that could break the 10 invariants listed above

---

## Output Format

Structure your review output as follows:

```
## Summary
One paragraph: overall quality signal, top risks, confidence level.

## Findings

### CRITICAL — [title]
File: path/to/file.rs:line
Description: What is wrong and why it matters.
Recommendation: Specific fix.

### HIGH — [title]
...

### MEDIUM — [title]
...

### LOW / NITPICK — [title]
...

## Invariant Check
For each of the 10 invariants: ✅ Holds / ⚠️ Uncertain / ❌ Violated — one line each.

## Demo Readiness
Is the codebase in a state for demo recording? What must be resolved first?
```

Severity definitions:
- **CRITICAL** — could lose funds, break on-chain invariants, or block hackathon submission
- **HIGH** — logic bug, security risk, or test coverage gap that will cause problems in production
- **MEDIUM** — correctness concern, design smell, or missing guard that should be fixed before demo
- **LOW** — style, naming, doc, or minor optimisation — fix if time allows

---

## Key Files to Read First

Before reviewing any code, read these files in order for full context:

1. `CLAUDE.md` — full architecture, invariants, data flow, design decisions
2. `SOULCONTRACT.md` — constitutional rules the swarm must enforce
3. `docs/SECURITY_AUDIT_2026-04-07.md` — previous findings and fix status
4. `ONBOARDING.md` — current build state
5. `BUILD_PLAN_v3.md` — active milestone tracker
6. `SESSION-CONTEXT.md` — current decisions and blockers

---

## Fast Sim Calibration — Critical Invariants (Python Layer)

Even though the Python scripts are gitignored, if you are reviewing any interface between the Python research layer and the Rust swarm, be aware of these hard-won invariants that must be preserved across any bridge:

1. **ATR formula**: `std(returns, 20h) × price` — NOT True Range
2. **MR entry condition**: `rsi < 35 and daily_trend == bullish` — NOT `bull_count >= min_alignment`
3. **Sharpe annualization**: `sqrt(n_trades / total_hours × 8760)` — NOT `sqrt(24 × 365)`

Any Rust code that reimplements or calls these calculations must match exactly.

---

## Contact / Repo

- **Repo**: https://github.com/tradewife/resilient-token-protocol
- **Source (Python)**: `git@github.com:tradewife/fractal-swarm.git`
- **Research**: `git@github.com:tradewife/rtp-skills-research.git`
