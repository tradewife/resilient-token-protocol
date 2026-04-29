
# RTP Pipeline Integrity Audit

```
You are conducting a PIPELINE INTEGRITY AUDIT for the Resilient Token Protocol (RTP).

This is NOT a code quality or security audit. This audit answers one question:

  "For every programmatic action this system is DESIGNED to perform,
   does a real, end-to-end execution path exist — or is there a gap,
   stub, mock, or broken handoff?"

Your output is a structured gap registry. For every finding, classify:
  - BROKEN: path exists in code but is demonstrably non-functional
  - STUBBED: path is mocked/simulated but not real execution
  - UNLINKED: component A produces output but component B never reads it
  - ASSUMED: action is documented as happening but no code triggers it
  - PARTIAL: path works in some environments but not the target environment

---

## PIPELINE STAGES TO AUDIT

Audit each of the following stages IN ORDER. For each stage, answer:
(a) What is this stage SUPPOSED to do?
(b) Does code exist to do it?
(c) Does it actually get called/triggered in the live execution path?
(d) What are the inputs and where do they come from?
(e) What are the outputs and where do they go?
(f) What environment assumptions does it rely on (env vars, network, oracle)?

---

### STAGE 1 — Fee Ingestion
Intended: Token project fees route into the Treasury PDA vault via `withdraw_fees`.
- Is `withdraw_fees` wired to actual SPL TransferFeeConfig CPI?
- Is there a caller that invokes this on a schedule, or does it rely on a permissionless actor?
- Who is the permissionless actor in practice? Is there a cron/bot/script that calls it?
- If no one calls `withdraw_fees`, does vault balance ever actually increase?

### STAGE 2 — Night Shift (Research Pipeline)
Intended: Daily 14:00 UTC — Python runs grid search → WFA → Darwinian → full-sim validation → produces a validated strategy config.
- Does `rtp-night-shift` Railway service actually run and complete successfully?
- Does it write output to a persistent location accessible by the Rust swarm?
- What is the output format and where is it written? (file path, Redis key, DB?)
- Does `bridge.rs` read from this location? Under what conditions?
- If Night Shift fails silently, what is the fallback strategy?

### STAGE 3 — Strategy Promotion (Python → On-Chain)
Intended: A validated strategy config from Night Shift becomes a Live strategy on-chain via `register_strategy`.
- Who calls `register_strategy`? (authority-gated — so who holds the authority key in production?)
- Is there any automation that reads Night Shift output and calls `register_strategy`, or is this a manual step?
- If manual, document it explicitly as a human dependency.
- What is the data path: Night Shift output → bridge.rs → ExecutePermit → `register_strategy` call?

### STAGE 4 — Devnet Loop Daemon
Intended: Every 6h, `rtp-daemon` wakes, runs one cycle: reads on-chain state, builds ExecutePermit, calls `open_flash_position` CPI, waits, calls `close_flash_position`.
- Does `rtp-daemon` actually connect to a live RPC endpoint? What endpoint? Is it set via env var?
- Does it read a real strategy from on-chain state or use a hardcoded config?
- Does it call Flash Trade REST API (`flashapi.trade`) to validate market state before sending CPI?
- Does it actually send a Solana transaction, or does it simulate/log only?
- What happens if Flash Trade returns a stale oracle (error 6007 on devnet)?
- Is the 6h cron schedule confirmed active on Railway, or is it only configured?

### STAGE 5 — Flash Trade CPI Execution
Intended: Treasury PDA calls Flash Trade Perpetuals via `invoke_signed` to open/close positions.
- On mainnet: confirm the two known TXs (`2bLg1Fu...`, `dFqkoP2...`) still represent the current code path.
- On devnet: confirm the known oracle failure mode is handled gracefully (not a panic/crash).
- Is position size correctly computed as ≤ 20% of vault balance at time of execution?
- Is committed_sol_lamports updated atomically with position open?
- What calls `close_flash_position`? Is there a timeout-based auto-close, or only manual/daemon-triggered?
- If the daemon crashes mid-cycle (after open, before close), what resolves the dangling position?

### STAGE 6 — Yield Return & Redistribution
Intended: After `close_flash_position`, SOL returns to vault, then `check_redistribute` splits 70/20/10.
- Who calls `check_redistribute`? Is it called automatically after close, or is it permissionless and unscheduled?
- Does close_flash_position atomically trigger redistribution, or are they separate txns?
- Are the three destination wallets (holders/project/ecosystem) actually configured at treasury init?
- Is 70% to holders implemented as an on-chain airdrop, a claimable balance, or a counter update?

### STAGE 7 — Strategy Evolution (LLM Loop)
Intended: Devnet loop daemon uses LLM API to evolve strategy config each cycle.
- What LLM endpoint is called? Is it `LLM_API_BASE_URL` + `LLM_API_KEY` + `LLM_MODEL`?
- Are these env vars confirmed set in the Railway `rtp-devnet-loop` service?
- What prompt is sent? What is the expected response format?
- Is the LLM response validated before use? What happens on malformed output?
- Does the evolved config get written back to on-chain state, or only used in-memory for the next cycle?
- Is there a drift-detection / soulguard check on evolved configs before they become Live?

### STAGE 8 — Swarm Coordinator & Wings
Intended: Coordinator orchestrates 6 wings via message bus; wings communicate only through Coordinator.
- Is the Coordinator and message bus running as a persistent process, or only instantiated per daemon cycle?
- Are all 6 wings (trading, security, evolve, knowledge, audit, futureproof) actually spawned in the daemon loop?
- Is the Audit wing's 3-agent tribunal (Skeptic/UserProxy/Optimizer) invoked on every strategy proposal?
- Is Byzantine consensus enforced, or does a single agent decision pass?
- Does the Security wing's rate-limiting and suspicious-proposal detection actually gate message bus throughput?

### STAGE 9 — Emergency Controls
Intended: `freeze_treasury` halts all trading; `emergency_close_all_positions` unwinds exposure.
- Who can call `freeze_treasury` in production? Is the authority key accessible to a human within minutes?
- Is there a monitoring alert that triggers when the daemon fails or treasury balance drops below runway floor?
- `emergency_close_all_positions` zeroes counters but does NOT close Flash Trade positions — is there a documented runbook for the follow-up manual close or liquidation path?
- Is there a tested script/CLI command to call `freeze_treasury` from a cold start?

### STAGE 10 — Adopter Onboarding
Intended: Any token project calls `register_adopter`, routes fees, and starts receiving yield.
- Is `register_adopter` functional on mainnet today?
- Is there a documented integration path (SDK call, script, or UI flow) for a new adopter?
- Does the dashboard `/launch` page actually submit `register_adopter` on-chain, or does it simulate?
- After registration, does fee routing happen automatically or does the adopter configure it manually?

---

## OUTPUT FORMAT

For each stage, produce:

```


### STAGE N — [Name]

STATUS: [COMPLETE / PARTIAL / BROKEN / STUBBED / ASSUMED / UNLINKED]
WHAT WORKS: [1–3 sentences]
GAP: [Specific missing link, e.g. "No caller for register_strategy — manual step assumed but undocumented"]
RISK: [HIGH / MEDIUM / LOW] — impact on demo-day or protocol correctness
NEXT ACTION: [Single concrete fix, e.g. "Add post-night-shift script that calls register_strategy via anchor CLI"]

```

After all stages, produce:

### CRITICAL PATH GAPS
List only HIGH-risk gaps ordered by: (1) blocks demo-day, (2) breaks protocol correctness guarantees.

### HUMAN DEPENDENCIES
List every action that requires a human to take, that the protocol CLAIMS is autonomous.
```


***

## How to Use This

1. **Run it as a single Claude Code session** with full repo access — it can walk the files itself
2. The output gap registry becomes your **next sprint's task list**
3. Pay special attention to **Stage 3** (strategy promotion) and **Stage 6** (redistribution) — based on the architecture these are the most likely to be "assumed" rather than wired
4. **Stage 9** (emergency controls) is your demo-day liability — a judge asking "but what if it goes wrong?" needs a live answer

The most dangerous finding class is **ASSUMED**: things documented as autonomous in the README/CLAUDE.md that are actually manual steps in disguise. That's what normal code audits miss entirely.

