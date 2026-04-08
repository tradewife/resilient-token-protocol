# SYMPHONY.md — RTP Orchestrator Implementation Spec

## Project Orientation

This is the **Resilient Token Protocol (RTP)** — an autonomous on-chain Solana system that
transforms "don't rug" from a social promise into a cryptographically enforced, agent-operated
guarantee.

**How it works:**
- Token projects adopt RTP by enabling `TransferFeeConfig` on their SPL Token-2022 mint,
  locking the RTP Treasury PDA as the withdraw authority — permanently and immutably
- Every transfer auto-routes a fee into the Treasury PDA (program-owned, no private key)
- An autonomous agent swarm manages the treasury under hard on-chain constraints
- Agents execute strategies (buybacks, LP seeding, yield harvesting) to grow reserves
- A programmatic price floor is enforced — the treasury cannot be drained

**The soul of this system is `soulcontract.md`** — the constitutional governance layer.
Every agent, at every decision point, consults it. No action that violates the five core
values can be executed. Read it before touching anything.

**Key directories:**
- `rtp/` — Solana program (Anchor), treasury PDA, price floor logic
- `agents/` — strategy agents (buyback, LP, yield, audit wings)
- `backtesting/` — historical simulation harness
- `orchestrator/` — (to be built by this spec) the Symphony daemon in Rust
- `soulcontract.md` — constitutional constraints; immutable core values
- `BUILD_PLAN_v3.md` — current canonical build plan and milestone tracking

**This document defines the task for building `orchestrator/`** — a native Rust implementation
of the [OpenAI Symphony SPEC](https://github.com/openai/symphony/blob/main/SPEC.md), adapted
to use Solana on-chain events as the trigger source instead of Linear issues.

The goal: an eternally running daemon with no human in the loop, where the Anchor program
enforces the safety constraints and Symphony handles the orchestration.

---

## Task: Build the RTP Orchestrator in Rust (Symphony SPEC)

**Read first:**
- The Symphony SPEC: https://github.com/openai/symphony/blob/main/SPEC.md
- The Elixir reference implementation (for behavioural reference only): https://github.com/openai/symphony/tree/main/elixir
- `soulcontract.md` — understand operating constraints before writing any agent-facing code
- `rtp/` — understand the Anchor program structure and treasury account layout
- `agents/` — understand existing agent entry points

**Goal:** Implement the Symphony orchestration daemon in Rust as a new crate at `orchestrator/`.
Do **not** use Linear. Replace the Issue Tracker with a **Solana event source** that polls
on-chain treasury state. Replace the Codex agent executable with RTP's own agent processes.

---

## Crate Structure to Create

```
orchestrator/
├── Cargo.toml
├── WORKFLOW.md          ← RTP strategy contract (see below)
├── src/
│   ├── main.rs
│   ├── config.rs        ← WORKFLOW.md front matter parser (serde + yaml)
│   ├── domain.rs        ← Issue, WorkflowDefinition, RunAttempt, LiveSession, RetryEntry, OrchestratorState
│   ├── tracker/
│   │   ├── mod.rs       ← IssueTrackerClient trait (3 methods from SPEC §11)
│   │   └── solana.rs    ← SolanaTracker impl + mock stub
│   ├── orchestrator.rs  ← poll loop, dispatch, concurrency, retry, reconciliation (SPEC §7–8)
│   ├── workspace.rs     ← per-task workspace dirs + lifecycle hooks (SPEC §9)
│   ├── runner.rs        ← spawns agent subprocess over stdio JSON-RPC (SPEC §10)
│   ├── observability.rs ← structured tracing logs + optional HTTP status endpoint (SPEC §13)
│   └── server.rs        ← axum HTTP server for /status and /health
└── tests/
    └── orchestrator_tests.rs
```

---

## The IssueTrackerClient Trait (implement exactly)

```rust
#[async_trait]
pub trait IssueTrackerClient: Send + Sync {
    async fn fetch_candidate_issues(&self) -> Result<Vec<Issue>>;
    async fn fetch_issues_by_states(&self, states: &[String]) -> Result<Vec<Issue>>;
    async fn fetch_issue_states_by_ids(&self, ids: &[String]) -> Result<HashMap<String, String>>;
}
```

---

## SolanaTracker Implementation

`SolanaTracker` connects to a Solana RPC endpoint and polls a treasury program account.
Map on-chain events to the `Issue` domain model as follows:

| Solana concept | Symphony Issue field |
|---|---|
| Treasury event type (`PriceFloorBreach`, `RebalanceDue`, `BuybackTriggered`, `YieldHarvest`) | `title` |
| Event pubkey / slot hash | `id` and `identifier` |
| Event parameters (serialised) | `description` |
| Priority: `Critical=1, High=2, Normal=3` | `priority` |
| On-chain status field | `state` → maps to `active_states` / `terminal_states` |
| Slot timestamp | `created_at` |

Use the `solana-client` crate with `RpcClient::get_program_accounts()` for polling.
Parse account data with `borsh`.

**For now, implement a mock/stub `SolanaTracker`** that generates synthetic treasury events
on a timer so the full orchestration loop can be tested without a live devnet connection.
Feature-flag the stub: `#[cfg(feature = "mock-tracker")]`.

---

## WORKFLOW.md to create at `orchestrator/WORKFLOW.md`

```yaml
---
tracker:
  kind: solana
  rpc_url: $SOLANA_RPC_URL
  program_id: $RTP_PROGRAM_ID
  active_states:
    - PriceFloorBreach
    - RebalanceDue
    - BuybackTriggered
    - YieldHarvest
  terminal_states:
    - Executed
    - Cancelled
    - Expired
polling:
  interval_ms: 15000
workspace:
  root: /tmp/rtp_workspaces
agent:
  max_concurrent_agents: 5
  max_turns: 10
  max_retry_backoff_ms: 120000
codex:
  command: python agents/strategy_runner.py
  approval_policy: auto
  turn_timeout_ms: 60000
  stall_timeout_ms: 60000
server:
  port: 4000
---

You are an autonomous RTP strategy agent operating under soulcontract.md.

Task: {{ issue.identifier }} — {{ issue.title }}
Treasury event: {{ issue.description }}
Attempt: {{ attempt | default: 1 }}

Before taking any action:
1. Re-read soulcontract.md — all five core values must be satisfied
2. Confirm the action does not violate any "What Cannot Evolve" constraint
3. If the action is irreversible, require human approval before executing

Execute the appropriate strategy, submit the transaction, and report the
tx signature as the final output when done.
```

---

## Orchestrator State Machine (SPEC §7)

Implement these exact states as a Rust enum:

```rust
pub enum OrchestratorIssueState {
    Unclaimed,
    Claimed,
    Running,
    RetryQueued,
    Released,
}
```

The poll tick sequence must follow SPEC §8.1 exactly:
1. Reconcile running issues (stop any whose on-chain state is now terminal)
2. Dispatch preflight validation (WORKFLOW.md readable, tracker configured)
3. Fetch candidates from `SolanaTracker`
4. Sort: `priority` ascending, then `created_at` oldest first, then `identifier` lexicographic
5. Dispatch while global and per-state concurrency slots remain
6. Emit structured tracing events

---

## Runner Protocol (SPEC §10)

The runner spawns an agent subprocess and communicates over **stdio JSON-RPC**.
Implement a minimal version for this PR:
- Spawn the process defined in `codex.command`
- Write the rendered prompt as a JSON message to stdin
- Read line-delimited JSON events from stdout
- Forward `turn_complete`, `error`, and `token_usage` events back to the orchestrator
- Enforce `turn_timeout_ms` and `stall_timeout_ms` using tokio timeouts

---

## Cargo.toml Dependencies

```toml
[dependencies]
tokio            = { version = "1", features = ["full"] }
serde            = { version = "1", features = ["derive"] }
serde_json       = "1"
serde_yaml       = "0.9"
async-trait      = "0.1"
tracing          = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
axum             = "0.7"
solana-client    = "1.18"
solana-sdk       = "1.18"
borsh            = "1"
minijinja        = "1"
anyhow           = "1"
thiserror        = "1"
dashmap          = "6"

[features]
mock-tracker = []
```

---

## Tests to Write

1. `test_mock_tracker_returns_issues` — stub emits synthetic events; verify Issue fields map correctly
2. `test_orchestrator_dispatches_up_to_concurrency_limit` — verify `max_concurrent_agents` is respected
3. `test_retry_backoff_schedules_correctly` — failed run → retry entry with correct `due_at`
4. `test_workflow_md_parses_front_matter` — WORKFLOW.md loads cleanly, `active_states` correct
5. `test_prompt_renders_with_issue_fields` — minijinja renders `{{ issue.title }}` correctly

---

## What NOT to Do

- Do not implement the Linear adapter — it will never be used
- Do not connect to Solana devnet in this PR — `mock-tracker` covers all tests
- Do not implement transaction signing or submission here — the agent subprocess handles that
- Do not modify anything in `rtp/`, `agents/`, or `soulcontract.md` — read them only

---

## Definition of Done

- `cargo build --features mock-tracker` compiles cleanly with no warnings
- `cargo test --features mock-tracker` passes all 5 tests
- `cargo run --features mock-tracker` starts the daemon, emits structured JSON logs,
  and begins polling the mock tracker every 15 seconds
- `GET http://localhost:4000/status` returns JSON with current orchestrator runtime state
- `GET http://localhost:4000/health` returns `{"status":"ok"}`
