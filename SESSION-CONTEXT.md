# RTP Session Context

> **How to use this file:** Paste the relevant sections at the top of every fresh agent session. Do not paste the full papers or full repo. This file is the compressed institutional memory of the project. Update it after each significant session.

---

## 1. Canonical Project Definition

**Project:** Resilient Token Protocol (RTP)
**Hackathon:** SWARMs / Canteen — April 6 – May 11, 2026
**Stack:** Solana (Anchor), Python agents, Arweave/Filecoin persistence

**Core thesis:**
Transform "don't rug" from a social promise into a cryptographically enforced, autonomously operated on-chain system.

**Evolved definition (current):**
RTP is a memory-persistent, self-coordinating, self-improving agent system whose actions are bounded by a Solana program so that token longevity is enforced by code, not trust.

**Functional description:**
- A token allocates a portion of fees/emissions into an on-chain treasury.
- A Solana Anchor program enforces hard constraints: price floor, treasury limits, permitted actions, distribution rules.
- An off-chain agent swarm observes protocol state and executes treasury operations only inside those constraints.
- The swarm is not stateless — it accumulates memory, distills strategy knowledge, and improves over repeated market cycles.
- Core claim: agent operations are bounded by on-chain invariants, fully auditable, and designed for token survival over time.

**Product story (never change this regardless of architecture depth):**
> A token launches with RTP. Part of its economics flow into a program-enforced treasury. An autonomous agent swarm manages that treasury forever under hard on-chain constraints. The agents remember prior cycles, improve strategy over time, and cannot rug because the program forbids it.

---

## 2. Architecture — Accepted Decisions

These are not proposals. They are decisions made. Do not relitigate them unless a concrete technical blocker requires it.

### Layer 1 — Anchor Program (Constitution)
- Enforces hard constraints: price floor, max withdrawal, permitted instruction set
- This is Ring 1 (immutable, human-defined). Agents cannot override it.
- Demo must show at least one constraint being enforced visibly.

### Layer 2 — Orchestration Daemon (Symphony-style)
- Long-running process that polls on-chain events and dispatches tasks
- Manages task lifecycle: retry, stall detection, reconciliation
- Replaces Symphony's Linear integration with a Solana event source
- Think: scheduler + watchdog, not the agent itself

### Layer 3 — Swarm Coordination (CORAL-style)
- Shared persistent memory hub (attempts, notes, skills folders)
- Asynchronous multi-agent execution in isolated contexts
- Heartbeat triggers: reflection (per-iteration), consolidation (periodic), redirection (stagnation)
- Sequential protocol: agents hand off completed outputs, not intentions
- No pre-assigned rigid roles — agents self-select contribution based on context

### Layer 4 — Memory Layer (Prologue-style)
- Durable memory across cycles using compression ladder: working → project → overview → core
- Visibility tiers: private → inspectable → shared → canonical
- First-principles execution discipline (FPEF): no hope-based language, no solution before analysis
- Post-session insight extraction: git-diff or tx-log → insight → memory store

---

## 3. Research Takeaways (Compressed)

Do not re-read the papers. Use only these extracted design consequences.

### From CORAL (arxiv 2604.01658)
- Use shared persistent memory, not stateless agents — knowledge reuse is the primary driver of improvement
- Use heartbeat triggers for reflection (per-iteration), consolidation (periodic), redirection (stagnation)
- Multi-agent co-evolution outperforms running multiple independent agents with same compute
- 4 agents achieved 20% better score than best-known single-agent result on kernel engineering task
- Extract minimum viable mechanism — do not implement full evolutionary search for hackathon

### From Self-Organization Paper (arxiv 2603.28990)
- Hybrid Sequential protocol (fixed order + self-selected roles) outperformed centralized by +14%, fully autonomous by +44%
- Agents receiving completed outputs of predecessors outperform agents receiving intentions, history, or a coordinator's plan
- Do not pre-assign rigid roles — roles are emergent computational functions, not org chart positions
- Scaling agents beyond what's needed yields no quality gain (p=0.61 at 64→256 agents) at high cost
- Model capability matters more than agent count — invest in model quality, not quantity
- Self-organization requires a capability threshold: weak models need more structure, not less

### From Prologue (github.com/aegntic/prologue)
- MemoryMatrix gives durable file-based persistence with atomic writes — zero-dependency (only Zod)
- Compression ladder: working (scratchpad) → project (task context) → overview (cross-cycle) → core (durable truths)
- FPEF 4-phase enforcement prevents agents jumping to solutions: Find → Prove → Evidence → Fix
- Orchestrator runs post-session pipeline: git diff → insight extraction → automatic memory storage
- MCP server available — Claude Code compatible via stdio transport
- Python bridge available for optional Graphiti knowledge graph + embedding support

### Night Shift Research Output (live finding — Apr 9 run)
- SOL/USDT candidate #1: Survivor score 2.69 (+2.46 over baseline)
- OOS Sharpe +3.96, 100% consistency (9/9 folds profitable), fragility 0.29, 47 trades/fold
- Config: signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop_atr=0.5
- Status: STRONG RECOMMEND — candidate for live execution in Trading Wing
- Apr 10 run: completed on CI (3h01m), results in CI artifact only — not yet committed to repo

---

## 4. MVP Boundary

The MVP is not "fully autonomous open-ended financial AGI."

The MVP **is**:
- One constrained Anchor treasury program
- One autonomous orchestration loop
- One bounded swarm coordination mechanism
- One persistent memory layer (working→project compression minimum)
- One or two treasury actions (e.g. buyback, LP defense)
- One observable adaptation moment (agent referencing prior cycle memory)

Anything beyond this is stretch. Label stretch goals explicitly.

---

## 5. Demo Requirements

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

**Fix path for Points 3 & 4 (pure Rust, ~2h):** Extend demo.rs to run two orchestrator cycles with memory_promotion persistence between them. Trigger a heartbeat redirect in cycle 2 that references cycle 1 yield data. No frontend needed.

**Fix path for Point 5 (~4-6h):** Single-page HTML dashboard reading a static JSON file dumped by the Rust demo binary + one Solana RPC call for treasury balance. See Task 5 spec in audit report.

---

## 6. Current Blocker

**Judge verification points 3, 4, and 5 have zero demo coverage.**

Priority order to resolve:
1. **Tonight (~2h):** Extend demo.rs for two-cycle memory_promotion + heartbeat redirect — closes points 3 and 4
2. **This weekend (~4-6h):** Build static HTML dashboard — closes point 5
3. **Before May 4 (15min):** Register individually on Colosseum — hard deadline, blocks submission

---

## 7. Open Decisions (Do Not Resolve Speculatively)

| Decision | Status | Notes |
|---|---|---|
| Trust model for agent execution | OPEN | Multisig? Optimistic challenge? ZK? Not required for MVP demo. |
| Demo UX | **DECISION REQUIRED** | Browser dashboard (~6h) vs recorded video (~2h). Dashboard covers more judge points. Video is faster. |
| Invariant 7 (soulguard reload sig) | **CLOSED (documented)** | Production TODO: ed25519 on reload(). Comment added to soulguard.rs. Demo path unaffected. |

---

## 8. Session Status

**Session 2026-04-11 deliverables: AUDIT COMPLETE**

Audit findings (Apr 11 deep audit):
1. Test count corrected: 238 → **205** (stale count from Apr 9 commit 57e6f7e; cargo fmt + clippy refactored config.rs and other modules)
   - Per-file: evaluator(29), heartbeat(26), memory_promotion(23), orchestrator(14), audit(12), bridge(12), config(10), rollback(10), security(9), proposer(9), assessor(9), soulcontract_spec(9), knowledge(8), lifecycle(8), trading(7), futureproof(5), evolve/mod(3), router(2)
   - 0 #[ignore] markers, no external test files outside workspace
2. Invariant 7 STUB confirmed: soulguard.rs:107-115 reload() has no signature verification
3. H-5 fix CONFIRMED: exceeds_rollback_threshold() correctly reads from spec via RwLock ✅
4. CI status: swarm-ci.yml ✅, night_shift.yml ✅, node-build.yml ✅ (no-op — no frontend dir)
5. Night shift pipeline: OPERATIONAL — real output in research/ subdirs, Apr 9 top candidate is SOL/USDT Survivor 2.69
6. Dashboard: NOTHING EXISTS — zero frontend files in repo
7. Demo coverage: 2/5 judge points covered (points 3, 4, 5 missing)

**Invariant enforcement: 9/10. Invariant 7 is documented stub. All others enforced.**

**Next session:** Extend demo.rs for two-cycle run (points 3 + 4), then build HTML dashboard (point 5).

---

## 9. Response Style

For any significant proposal, return:
- **(a) What's strong**
- **(b) What's weak**
- **(c) Next concrete action**

For any architecture decision, evaluate against:
- Hackathon feasibility
- Demoability
- Novelty
- Trust model clarity

For any new subsystem, state:
- MVP or stretch
- What assumption it relies on
- How to test that assumption fast

---

## 10. Mental Model

```
Anchor program     = constitution        (immutable, Ring 1)
Orchestrator       = executive scheduler (dispatches, watches, wires the loop)
Agent swarm        = bounded civil service (executes within law)
Memory layer       = institutional memory (learns across cycles)
Evaluator          = survival objective   (defines success)
Heartbeat          = rhythm & triggers    (CORAL-style coordination)
Demo               = proof the institution persists without founder trust
```

---

*Last updated: 2026-04-11 — deep audit complete. 205 tests (corrected from 238), 0 failures, 0 warnings. Judge points 3/4/5 uncovered — demo extension required.*
*Update this file after each session that changes canonical decisions or resolves open decisions.*
