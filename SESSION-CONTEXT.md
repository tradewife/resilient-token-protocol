# RTP Session Context

> **How to use this file:** Paste the relevant sections at the top of every fresh agent session. Do not paste the full papers or full repo. This file is the compressed institutional memory of the project. Update it after each significant session.

---

## 1. Canonical Project Definition

**Project:** Resilient Token Protocol (RTP)
**Hackathon:** SWARMs / Canteen — April 6 – May 11, 2026
**Stack:** Solana (Anchor), Rust swarm runtime, Python research agents

**Core thesis:**
Transform "don't rug" from a social promise into a cryptographically enforced, autonomously operated on-chain system.

**Evolved definition (current):**
RTP is a memory-persistent, self-coordinating, self-improving agent system whose actions are bounded by a Solana program so that token longevity is enforced by code, not trust.

**Functional description:**
- A token allocates a portion of fees/emissions into an on-chain treasury (Solana Anchor program).
- The Anchor program enforces hard constraints: price floor, treasury limits, permitted actions, distribution rules.
- An off-chain Rust swarm observes protocol state and executes treasury operations only inside those constraints.
- The Python research layer (Night Shift) runs 30K configs/night, 9-fold WFA, Darwinian evolution — validated strategies are handed to the Rust Trading Wing via bridge.rs.
- The Trading Wing executes validated strategies as **perpetuals trades on Hyperliquid**, signed and submitted via **Phantom wallet integration**.
- Yield (USDC) flows back to the Solana treasury PDA. The redistribution split (70/20/10) is enforced on-chain.
- The swarm accumulates memory, distills strategy knowledge, and improves over repeated market cycles.
- Core claim: agent operations are bounded by on-chain invariants, fully auditable, and designed for token survival over time.

**Product story (never change this regardless of architecture depth):**
> A token launches with RTP. Part of its economics flow into a program-enforced treasury. An autonomous agent swarm manages that treasury forever under hard on-chain constraints — executing perps strategies on Hyperliquid via Phantom, returning yield to holders. The agents remember prior cycles, improve strategy over time, and cannot rug because the program forbids it.

---

## 2. Execution Venue — The Hyperliquid + Phantom Path

This is the **critical trajectory** for the demo and for judging. All build work converges here.

### Why Hyperliquid
- Highest-liquidity perps DEX with a documented REST + WebSocket API
- No KYC for programmatic access; supports USDC-margined perpetuals
- API: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api
- Python SDK: https://github.com/hyperliquid-dex/hyperliquid-python-sdk
- Rust SDK (community): https://github.com/hyperliquid-dex/hyperliquid-rust-sdk

### Why Phantom
- Sponsored hackathon resource: https://docs.phantom.app/phantom-connect/introduction
- Phantom Connect enables agentic wallet flows — the swarm signs Hyperliquid orders via Phantom
- CASH stablecoin (sponsored) is the settlement currency for treasury yield flows
- The demo shows: Phantom wallet connected → Trading Wing submits order → Hyperliquid fills → USDC yield → treasury PDA

### Execution Flow (target state for demo)
```
Night Shift (Python)
  └── validated strategy config (SOL/USDT Survivor 2.69)
        │
        ▼ bridge.rs (JSON)
Trading Wing (Rust)
  └── ExecutePermit payload
        │
        ▼ Hyperliquid REST API
           POST /exchange  (place_order)
           signed via Phantom Connect (agentic wallet)
        │
        ▼ fill confirmed
           USDC yield → Treasury PDA (Solana)
        │
        ▼ check_redistribute (on-chain)
           70% holders / 20% project dev / 10% ecosystem
```

### Current State of Execution Path
| Step | Status | Gap |
|------|--------|-----|
| Strategy validated (SOL/USDT Survivor 2.69) | ✅ DONE | — |
| bridge.rs wires Python → Rust | ✅ DONE | — |
| Trading Wing handles ExecutePermit | ✅ DONE | In-memory mock only |
| Hyperliquid API call in Trading Wing | ❌ MISSING | Need `reqwest` + HL order struct |
| Phantom signing of HL order | ❌ MISSING | Phantom Connect agentic flow |
| USDC yield → treasury PDA | ❌ MISSING | CPI transfer after fill confirmed |
| devnet end-to-end | ❌ MISSING | Entire HL→PDA path untested |

**This is the single critical path. Everything else is scaffolding.**

---

## 3. Architecture — Accepted Decisions

These are not proposals. They are decisions made. Do not relitigate them unless a concrete technical blocker requires it.

### Layer 1 — Anchor Program (Constitution)
- Enforces hard constraints: price floor, max withdrawal, permitted instruction set
- This is Ring 1 (immutable, human-defined). Agents cannot override it.
- Demo must show at least one constraint being enforced visibly.

### Layer 2 — Orchestration Daemon
- Long-running Rust process that polls on-chain events and dispatches tasks
- Manages task lifecycle: retry, stall detection, reconciliation
- Heartbeat triggers: reflection (per-iteration), consolidation (periodic), redirection (stagnation)

### Layer 3 — Swarm Coordination
- Shared persistent memory hub
- Asynchronous multi-wing execution via Coordinator message bus
- Sequential protocol: wings hand off completed outputs, not intentions
- All cross-wing communication typed and signed via soulguard

### Layer 4 — Memory Layer
- Durable memory across cycles: working → project → overview → core compression ladder
- memory_promotion.rs: 23 tests, built — not yet wired into demo binary

### Execution Venue (decided)
- **Perps:** Hyperliquid (REST API, USDC-margined)
- **Signing:** Phantom Connect (agentic wallet, sponsored)
- **Settlement:** CASH stablecoin (sponsored) for treasury yield flows
- **On-chain:** Solana devnet treasury PDA receives yield via CPI transfer

---

## 4. Research Takeaways (Compressed)

Do not re-read the papers. Use only these extracted design consequences.

### From CORAL (arxiv 2604.01658) — https://arxiv.org/pdf/2604.01658
- Use shared persistent memory, not stateless agents — knowledge reuse is the primary driver of improvement
- Use heartbeat triggers for reflection (per-iteration), consolidation (periodic), redirection (stagnation)
- Multi-agent co-evolution outperforms running multiple independent agents with same compute
- 4 agents achieved 20% better score than best-known single-agent result on kernel engineering task
- Extract minimum viable mechanism — do not implement full evolutionary search for hackathon

### From Self-Organization Paper (arxiv 2603.28990)
- Hybrid Sequential protocol (fixed order + self-selected roles) outperformed centralized by +14%, fully autonomous by +44%
- Agents receiving completed outputs of predecessors outperform agents receiving intentions, history, or a coordinator's plan
- Do not pre-assign rigid roles — roles are emergent computational functions, not org chart positions
- Scaling agents beyond what's needed yields no quality gain at high cost

### From karpathy/autoresearch — https://github.com/karpathy/autoresearch
- The Modify/Verify/Keep loop is the core primitive: generate candidate → verify against objective → keep if better
- RTP's Night Shift implements this loop over strategy configs (30K candidates → WFA → Darwinian)
- Apply same loop to the Hyperliquid execution layer: propose order → simulate → submit if passes soulguard

### Night Shift Research Output (live — Apr 9 run)
- SOL/USDT candidate #1: Survivor score 2.69 (+2.46 over baseline)
- OOS Sharpe +3.96, 100% consistency (9/9 folds profitable), fragility 0.29, 47 trades/fold
- Config: signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop_atr=0.5
- Status: STRONG RECOMMEND — this is the strategy the Trading Wing executes on Hyperliquid

---

## 5. MVP Boundary

The MVP **is**:
- One constrained Anchor treasury program (done)
- One autonomous orchestration loop (done)
- One bounded swarm coordination mechanism (done)
- One persistent memory layer (built, needs demo wiring)
- **One live Hyperliquid perps trade signed via Phantom** (critical gap)
- Observable treasury state on devnet explorer or dashboard

Anything beyond this is stretch. Label stretch goals explicitly.

---

## 6. Demo Requirements

A judge must be able to verify these five things in under 3 minutes:

1. On-chain enforced treasury constraint (show a violation being rejected)
2. Autonomous agent operation without human approval per step
3. Persistent memory across cycles (agent references prior session knowledge)
4. Visible strategy adaptation or learning (heartbeat redirect or skill promotion)
5. Observable treasury state on a dashboard or explorer

### Current Coverage (as of Apr 11 audit)

| Point | Status | Gap |
|---|---|---|
| 1. On-chain constraint rejected | PARTIAL | demo.sh shows Rust-side soulguard rejection. On-chain BelowThreshold rejection exists in devnet-demo.ts but requires live validator. |
| 2. Autonomous operation | COVERED | rtp-demo binary runs full 8-step pipeline without human approval. |
| 3. Persistent memory across cycles | MISSING | memory_promotion.rs exists with 23 tests but demo binary does not invoke it. No cross-cycle persistence shown. |
| 4. Visible adaptation/learning | MISSING | heartbeat.rs has redirect triggers (26 tests) but demo binary doesn't exercise them. No redirect visible in output. |
| 5. Observable treasury state | MISSING | No dashboard. demo.sh prints ASCII. No explorer link in output. |

**Fix path for Points 3 & 4 (pure Rust, ~2h):** Extend demo.rs to run two orchestrator cycles with memory_promotion persistence between them. Trigger a heartbeat redirect in cycle 2 that references cycle 1 yield data.

**Fix path for Point 5 (~4-6h):** Single-page HTML dashboard reading a static JSON file dumped by demo binary + one Solana RPC call for treasury balance. Devnet explorer link for the treasury PDA printed by demo binary satisfies this at minimum.

**Fix path for Hyperliquid execution (~1-2 days):** Wire `reqwest` in Trading Wing → POST to Hyperliquid testnet → sign via Phantom Connect agentic flow → receive fill → CPI transfer to treasury PDA.

---

## 7. Open Decisions

| Decision | Status | Notes |
|---|---|---|
| Trust model for agent execution | OPEN | Multisig? Optimistic challenge? ZK? Not required for MVP demo. |
| Demo UX | **DECISION REQUIRED** | Browser dashboard (~6h) vs recorded video (~2h). Dashboard covers more judge points. Video is faster. |
| Invariant 7 (soulguard reload sig) | CLOSED (documented) | Production TODO: ed25519 on reload(). Comment in soulguard.rs. Demo path unaffected. |
| Hyperliquid testnet vs mainnet for demo | **DECISION REQUIRED** | Testnet is safer, mainnet is more impressive. Testnet recommended for hackathon. |

---

## 8. Session Status

**Session 2026-04-11 — repo cleanup + scheduled prompt suite created**

State as of Apr 11:
- 205 tests (29 evaluator, 26 heartbeat, 23 memory_promotion, 14 orchestrator, 12 audit, 12 bridge, 10 config, 10 rollback, 9 security, 9 proposer, 9 assessor, 9 soulcontract_spec, 8 knowledge, 8 lifecycle, 7 trading, 5 futureproof, 3 evolve/mod, 2 router)
- 0 failures, 0 warnings
- Invariant enforcement: 9/10 (Invariant 7 documented stub)
- Repo cleaned: stale root docs deleted, docs/ reorganised, RESOURCES.md created
- **Hyperliquid perps via Phantom**: execution path defined (Section 2), implementation NOT started
- Judge points 3/4/5: still missing demo coverage

**Priority order for next session:**
1. Wire Hyperliquid REST call in Trading Wing (`reqwest` + order struct + HL testnet)
2. Phantom Connect agentic signing of HL order
3. Extend demo.rs for two-cycle run (closes judge points 3 + 4)
4. HTML dashboard (closes judge point 5)
5. Register individually on Colosseum before May 4

---

## 9. Key Links (always include these — LLMs go stale without them)

| Resource | URL |
|----------|-----|
| This repo | https://github.com/tradewife/resilient-token-protocol |
| Hyperliquid API docs | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Python SDK | https://github.com/hyperliquid-dex/hyperliquid-python-sdk |
| Hyperliquid Rust SDK | https://github.com/hyperliquid-dex/hyperliquid-rust-sdk |
| Phantom Connect docs | https://docs.phantom.app/phantom-connect/introduction |
| CASH stablecoin | https://docs.phantom.app/phantom-connect/cash |
| Squads Multisig | https://docs.squads.so |
| Swig smart wallets | https://docs.swig.fi |
| MoonPay Agents | https://www.moonpay.com/developers/agents |
| Solana MCP | https://github.com/solana-developers/solana-mcp |
| Anchor docs | https://www.anchor-lang.com/docs |
| Solana devnet RPC | https://api.devnet.solana.com |
| Colosseum hackathon | https://arena.colosseum.org |
| CORAL paper | https://arxiv.org/pdf/2604.01658 |
| karpathy/autoresearch | https://github.com/karpathy/autoresearch |
| Arcium (stretch) | https://docs.arcium.com |

---

## 10. Response Style

For any significant proposal, return:
- **(a) What's strong**
- **(b) What's weak**
- **(c) Next concrete action**

For any architecture decision, evaluate against:
- Hackathon feasibility
- Demoability on judging day
- Novelty
- Trust model clarity

For any new subsystem, state:
- MVP or stretch
- What assumption it relies on
- How to test that assumption fast

---

## 11. Mental Model

```
Anchor program     = constitution        (immutable, Ring 1)
Orchestrator       = executive scheduler (dispatches, watches, wires the loop)
Agent swarm        = bounded civil service (executes within law)
Memory layer       = institutional memory (learns across cycles)
Evaluator          = survival objective   (defines success)
Heartbeat          = rhythm & triggers    (CORAL-style coordination)
Hyperliquid        = execution venue      (where yield is generated)
Phantom            = signing layer        (agentic wallet, sponsored)
Demo               = proof the institution persists without founder trust
```

---

*Last updated: 2026-04-11 — repo cleaned, Hyperliquid+Phantom trajectory formalised in Section 2. 205 tests, 0 failures. Judge points 3/4/5 uncovered. Hyperliquid execution path not yet implemented.*
*Update this file after each session that changes canonical decisions or resolves open decisions.*
