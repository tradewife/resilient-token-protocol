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

---

## 6. Current Blocker

**None.** The evaluator / objective function was the critical blocker and is now
resolved. See `EVALUATOR.md` for the full specification.

**Next priority:** Implement `evaluator.rs` in the swarm runtime (see
EVALUATOR.md implementation checklist).

---

## 7. Open Decisions (Do Not Resolve Speculatively)

| Decision | Status | Notes |
|---|---|---|
| Trust model for agent execution | OPEN | Multisig? Optimistic challenge? ZK? |
| Evaluator / objective function | **RESOLVED** | `EVALUATOR.md` — Treasury Survival Index (TSI) |
| Memory backend for hackathon | **RESOLVED** | `memory_promotion.rs` — file-based JSON, Prologue structure |
| Demo UX | OPEN | Browser dashboard vs. recorded walkthrough |

---

## 8. Session Status

**Session 2026-04-09 deliverables: COMPLETE**

1. `EVALUATOR.md` — spec drafted and accepted (§6 blocker resolved)
2. `evaluator.rs` — TSI scoring, stagnation/terminal detection, degraded mode (29 tests)
3. `heartbeat.rs` — CORAL-style triggers, priority chain, safety short-circuit (26 tests)
4. `memory_promotion.rs` — four-tier compression ladder, redirect events, core (human-only) (23 tests)
5. `orchestrator.rs` — daemon loop wiring evaluator + heartbeat + memory (14 tests)

**Total: 238 tests passing, 0 failures, 0 warnings.**

**The autonomous loop is now complete.** The orchestrator wires evaluator → heartbeat → memory promotion into a single dispatch loop with hooks, fetcher traits, graceful shutdown, and structured logging. `demo.sh` runs clean end-to-end across all three layers.

**Next session:** Focus on demo visibility — dashboard, observable adaptation moment, or recorded walkthrough. The runtime is feature-complete for MVP.

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

*Last updated: 2026-04-09 — orchestrator + autonomous loop complete. 238 tests, 0 failures, 0 warnings.*
*Update this file after each session that changes canonical decisions or resolves open decisions.*
