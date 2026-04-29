# RTP — Colosseum Hackathon Audit Report

**Date:** 2026-04-29 | **Deadline:** 2026-05-11 (12 days) | **Theme:** SWARMs / Canteen

---

## 1. JUDGE SIMULATION (5 Verification Points)

### Point 1: On-chain enforced treasury constraint (violation being rejected)
**PARTIAL**

The Anchor program has real `require!` guards that reject violations: `BelowThreshold`, `TreasuryFrozen`, `StrategyNotLive`, `TooManyOpenPositions`, `PositionSizeExceeded`, `InvalidFlashSide`. These are genuinely enforced on-chain — not decorative.

**However:** The demo simulates the constraint rejection via a Rust function (`simulate_below_threshold_withdrawal()`) that returns a hardcoded error string. A judge watching the demo will see terminal output, not an actual Solana Explorer failed transaction. The constraint *exists* in the deployed program, but the demo doesn't *prove* it live — the judge has to trust the code.

**Skeptical judge says:** "Show me the failed transaction on Explorer. I don't care about your println."

### Point 2: Autonomous agent operation without human approval per step
**PARTIAL**

The 8-step demo loop (Trading proposes → Soulguard → Audit tribunal → ExecutePermit → execution) runs autonomously within a single `run_demo_loop()` call. The daemon (`rtp-daemon`) runs on a 6h cron. No human approves individual steps.

**However:** The daemon is a one-shot process that exits after each cycle. The "autonomous loop" is actually Railway's cron restarting the container every 6 hours. The Trading Wing's `handle_message()` for ExecutePermit tries to call the Python bridge binary, which returns `Err(BinaryNotFound)` in production, and falls through gracefully. The actual Flash Trade CPI execution is *not* wired into the Rust daemon loop — it's done via the TypeScript CLI. The "autonomous agent operation" demo is the Rust Coordinator routing messages in-memory, not an end-to-end autonomous trade.

**Skeptical judge says:** "Your agent runs one cycle and exits. That's a cron job, not autonomy."

### Point 3: Persistent memory across cycles
**PARTIAL**

The `Orchestrator` writes memory to `data/swarm-memory/project/proj-*.json` files. The two-cycle demo proves memory persists by reading from disk in cycle 2. This is real file-based persistence that survives process restarts.

**However:** The `memory/working/` and `memory/core/` directories mentioned in the architecture are empty shells. The Knowledge Wing is in-memory only (`Vec<KnowledgeEntry>`). The memory that actually persists is Orchestrator state snapshots — not a rich knowledge graph. A judge looking at `data/swarm-memory/` will find thin JSON files with scalar metrics, not the "realtime knowledge graph" the README claims.

**Skeptical judge says:** "You wrote JSON to disk and read it back. That's a log file, not memory."

### Point 4: Visible strategy adaptation or learning
**PARTIAL**

The Evolve Wing proposes strategy mutations via LLM (or deterministic fallback). The `propose_strategy_mutation()` function calls an LLM API and returns `StrategyMutation` objects. The demo prints these mutations with param changes (signal_threshold, tp_atr, etc.). The `soulguard_trade_check()` enforces the 20% position cap.

**However:** The mutations are *proposed*, not backtested or validated within the demo. There's no feedback loop showing that mutation X improved outcome Y. The Night Shift Python layer does real Darwinian evolution, but the Rust swarm side just prints proposals. A judge watching the demo sees parameter changes flash by — but no evidence that the system *learned* anything during the demo itself.

**Skeptical judge says:** "You called an LLM and printed the output. Where's the closed-loop evidence?"

### Point 5: Observable treasury state on dashboard
**PASS**

The dashboard (`page.tsx`) polls devnet RPC every 10 seconds, reads treasury PDA balance, displays it in SOL, and shows a live activity feed with wing statuses. The `/api/cycle` endpoint serves real daemon output. The `resilientprotocol.xyz` domain is live. A judge can open the URL and see the treasury balance.

**Caveat:** The dashboard falls back to `FALLBACK_FEED` static data when `/api/cycle` returns 404. Judges may see stale static content if the daemon hasn't run recently. The treasury balance shown is devnet SOL, not meaningful value.

---

## 2. CODE AUDIT — Anchor Program

### Instruction-by-Instruction Review

| # | Instruction | Authority Check | Frozen Guard | Overflow | PDA Seeds | Verdict |
|---|------------|----------------|-------------|----------|-----------|---------|
| 1 | `initialize` | N/A (creator) | N/A | checked_add | OK | OK |
| 2 | `withdraw_fees` | PDA-signed | OK | saturating_sub | OK | OK |
| 3 | `check_redistribute` | PDA-signed | OK | u128 math + sat_sub | OK | OK |
| 4 | `hydrate_swarm` | Anyone (permissionless) | OK | sat_add | OK | OK |
| 5 | `evolve_phase` | Anchor constraint | OK | Direct comparison | OK | OK |
| 6 | `verify_adoption` | N/A (read-only) | N/A | N/A | OK | OK |
| 7 | `create_swarm_vault` | Anyone | OK | N/A | OK | OK |
| 8 | `register_adopter` | Anyone | OK | N/A | OK | OK |
| 9 | `register_adopter_beta` | Anyone | OK | N/A | OK | OK |
| 10 | `record_fee_deposit` | Anyone | OK | checked_add | OK | OK |
| 11 | `end_beta` | Manual check in handler | OK | N/A | OK | OK |
| 12 | `register_strategy` | Manual check in handler | OK | N/A | OK | OK |
| 13 | `update_strategy_performance` | **No authority check** | OK | sat_add | OK | **WEAK** |
| 14 | `force_retire_strategy` | Manual check | OK | N/A | OK | OK |
| 15 | `freeze_treasury` | Anchor constraint | N/A | N/A | OK | OK |
| 16 | `unfreeze_treasury` | Anchor constraint | N/A | N/A | OK | OK |
| 17 | `open_flash_position` | Anyone (PDA signs CPI) | OK | u128 bounds | OK | OK |
| 18 | `close_flash_position` | Anyone (PDA signs CPI) | OK | sat_sub | OK | OK |
| 19 | `emergency_close_all_positions` | Manual check | Intentionally exempt | N/A | OK | OK |

### Known Gaps — Actual Severity

1. **`update_strategy_performance` — NO authority check (CONFIRMED CRITICAL)**: The account context has `pub authority: Signer<'info>` but the handler never checks `authority.key() == treasury.authority`. Any signer can write arbitrary PnL, Sharpe, drawdown, and soft_decay_strikes values. A malicious caller could set `rolling_pnl_bps = 99999`, `rolling_sharpe_x100 = 99999`, `consecutive_losses = 0`, `drawdown_24h_bps = 0`, `new_soft_strike = false` and keep a bad strategy Live forever. **This is the most serious on-chain vulnerability.**

2. **`AdopterRecord` — No treasury back-reference (CONFIRMED, LOW IMPACT)**: `AdopterRecord` has `token_mint` but no `treasury: Pubkey` field. However, the PDA seed is `["adopter", token_mint]`, which is unique per token, and the `RecordFeeDeposit` context has a validated `treasury` account. An orphaned record is possible but doesn't cause fund loss.

3. **No `remaining_accounts` ownership validation in `open_flash_position` (PARTIALLY FIXED)**: The code checks `remaining.len() >= 19` and maps account metas correctly. The close handler validates the position PDA derivation. The open handler does NOT validate the remaining accounts — it trusts the caller to pass correct Flash Trade accounts. This is acceptable because Flash Trade's own program validates its accounts, and a wrong account would cause the CPI to fail. Not a fund-extraction risk.

4. **`soft_decay_strikes` reset gameable (CONFIRMED, MEDIUM)**: The reset condition is `rolling_pnl_bps > 0 && rolling_sharpe_x100 > 0`. Since `update_strategy_performance` has no authority check, a malicious caller can pass positive values to reset strikes every call. Even with the authority check fixed, the *design* allows legitimate callers to game strikes by batching trades to show positive PnL in between losses. This is a design concern, not a code bug.

5. **Phase thresholds use raw vault balance (CONFIRMED, DOCUMENTED)**: The code explicitly notes this with TODO comments for oracle integration. The authority is trusted to verify before calling. This is an accepted launch risk.

### Flash Trade CPI Discriminator Verification

- `FLASH_OPEN_POSITION_DISC: [135, 128, 47, 77, 15, 152, 240, 49]` — claimed to match IDL v15.2.0
- `FLASH_CLOSE_POSITION_DISC: [191, 210, 137, 115, 145, 22, 230, 244]` — claimed to match mainnet close TX

**Risk:** If Flash Trade upgrades their program, these discriminators change and all CPI calls silently fail with `FlashCpiFailed`. There is no version pin or fallback. The program ID is hardcoded as a string constant, not a configurable parameter. This is a real operational risk — but acceptable for hackathon scope.

### `invoke_signed` Security Assessment

The Treasury PDA signs with `seeds = [TREASURY_SEED, mint.as_ref(), bump]`. The `invoke_signed` call correctly passes these seeds. No private key exists. The only way to extract funds from the treasury is through the 3 instructions that move tokens: `check_redistribute` (70/20/10 split), `hydrate_swarm` (to swarm vault, gated by Live strategy), and `open_flash_position` (CPI to Flash Trade, gated by Live strategy + position limits + runway floor).

**Bypass vectors:**
- An attacker cannot steal treasury funds because they cannot sign for the PDA.
- An attacker *could* grief by calling `open_flash_position` with bad oracle prices, losing SOL on Flash Trade. But this is gated by strategy Live status and position limits (max 3, max 20%).
- The `emergency_close_all_positions` intentionally bypasses the frozen check, which is correct for emergency unwinding.

### On-chain Security Rating: **7/10**

The architecture is sound (PDA-owned, CPI-only, frozen guards, overflow protection). The `update_strategy_performance` authority gap is the only serious hole. Fix it and this jumps to 8.5/10.

---

## 3. CODE AUDIT — Rust Swarm

### Is the orchestration loop autonomous?

**Partially.** The `rtp-daemon` binary runs a single cycle and exits. Railway cron restarts it every 6 hours. Within a cycle, the daemon:
1. Reads on-chain state (treasury, strategy)
2. Reads Night Shift results from `data/night_results/latest/summary.json`
3. Runs Orchestrator cycles (poll state → evaluate → heartbeat)
4. Writes memory files to `data/swarm-memory/`
5. Optionally calls LLM for strategy mutations

The daemon can run indefinitely *if* the Railway cron keeps restarting it. But it cannot run continuously on its own — it's designed as a one-shot, not a daemon. The `Orchestrator` has a `poll_interval_ms` config, but the daemon doesn't use it as a long-running loop.

**Can it run without human intervention?** Yes, between the cron schedule. But:
- Night Shift must produce valid `summary.json` files
- The fee-payer keypair must have SOL for gas
- The LLM API key must be valid
- No error recovery exists — a single failure in any step kills the cycle

### Memory Layer — Is it genuinely persistent?

**Yes, via files.** The `Orchestrator` writes to `data/swarm-memory/project/proj-<timestamp>.json`. The two-cycle demo reads this back from disk. This survives process restarts.

**But:** The Knowledge Wing is pure in-memory (`Vec<KnowledgeEntry>`). The `working/` and `core/` directories are empty. The "knowledge graph" the README describes doesn't exist on disk. The memory that persists is scalar state snapshots, not semantic knowledge.

### `bridge.rs` — Night Shift to Trading Wing

The bridge reads `data/night_results/<date>/summary.json` and parses it into `NightShiftSummary`. If the file doesn't exist, is malformed JSON, or has missing fields, the entire bridge fails with a descriptive error. The `best_night_shift_candidate()` function filters rejected candidates and returns the best survivor score.

**What breaks:** If Night Shift changes the `summary.json` schema (adds/removes fields), the Rust `NightShiftSummary` struct will fail to deserialize. There's no schema versioning or forward-compatibility. The bridge binary subprocess path (`cycle_report.bin`) is legacy and likely never used in production.

### `soulguard_trade_check()` — 20% Position Cap

```rust
pub fn soulguard_trade_check(size: &str, price: &str, vault_balance: f64) -> Result<(), String>
```

This is a Rust-side check that takes `size` and `price` as strings, parses them, and checks `notional <= vault_balance * 0.20`. **This can be bypassed** because:
1. It's not called in the `open_flash_position` CPI path — the Anchor program has its own on-chain check
2. It's only called in the Hyperliquid execution path (which is archived behind a feature flag)
3. The on-chain check in `open_flash_position` uses `vault.amount` (token balance) and `MAX_POSITION_SIZE_BPS` — this IS the real enforcement

The soulguard check is a defense-in-depth layer for the legacy HL path, not the primary enforcement.

### Swarm Autonomy Rating: **6/10**

The architecture is solid (Coordinator, Soulguard, typed messages, quality gates). But the actual autonomy is Railway cron + one-shot binary. The daemon doesn't self-heal, doesn't retry, and doesn't have a continuous event loop. The "6-wing swarm" is more architectural than operational — the wings handle messages reactively but don't independently initiate actions on a schedule.

---

## 4. CODE AUDIT — Flash Trade CPI Path

### Instruction Discriminators

The discriminators are hardcoded constants matching "IDL v15.2.0". These were verified against mainnet transactions (open: `2bLg1Fu...`, close: `dFqkoP2...`). If Flash Trade deploys a new version with different discriminators, all CPI calls fail.

**There is no version negotiation or fallback.** This is a single-point-of-failure but architecturally unavoidable for CPI — you either match the target program's IDL or you don't.

### Account Ordering

`open_flash_position`: 19 accounts in `remaining_accounts`. The account metas are correctly mapped with writable/signer flags matching Flash Trade's expected layout. Account 0 is the PDA signer (read-only signer), account 1 is fee_payer (writable signer).

`close_flash_position`: 18 accounts. The code validates the position PDA at `remaining[6]` against `find_program_address(["position", treasury, market], flash_program)` — this prevents closing positions owned by other treasuries.

**One concern:** The `open_flash_position` handler does NOT validate remaining accounts the way `close_flash_position` does. A caller could pass any accounts. The CPI would fail at the Flash Trade program level, but the error would be `FlashCpiFailed` with no diagnostic info.

### CU Usage

99,214 CU for open is well within Solana's 1.4M CU default limit. Even with Composability swap-and-open (800K CU), there's headroom. No CU risk under normal load. The `FLASH_CU_LIMIT: 600_000` constant is unused (dead code).

### Flash Trade CPI Verdict: **8/10**

Well-implemented. The close handler has position PDA validation. The open handler has all constraint checks. The main risk is discriminator version pinning, which is inherent to CPI.

---

## 5. DEMO FLOW REVIEW

`docs/demo-flow.md` does not exist. The demo is in `demo.rs` and `cli/bin/rtp.ts demo`.

### What Breaks First

Running `cargo run --bin rtp-demo`:
1. **Step 1 (register_wings)**: Passes — pure in-memory
2. **Step 2 (trading_proposes)**: Passes — message routing
3. **Step 3 (security_check)**: Likely passes with "no message received" — the Security Wing isn't in the routing path for proposals by default
4. **Step 4 (audit_receives_proposal)**: Passes — Coordinator routes to Audit
5. **Step 5 (audit_tribunal)**: Passes — tribunal approves
6. **Step 6 (audit_result_routed)**: Passes
7. **Step 7 (trading_receives_permit)**: Passes
8. **Step 8 (strategy_assessment)**: **FAILS** — Trading Wing's `handle_message()` for ExecutePermit tries the Python bridge, which doesn't exist, and returns an `Error` payload. The demo catches this and marks it as "passed" with a "Bridge not available" note.

The demo always "completes" because error steps are marked as `passed: true` with explanatory detail. A judge watching the terminal output would see green checkmarks for steps that actually failed.

### Is 3 minutes achievable?

**Yes, for `cargo run --bin rtp-demo`.** The Rust demo runs in under 30 seconds. But it shows in-memory message routing, not on-chain transactions.

**No, for a full end-to-end demo.** The full flow (deploy treasury, deposit fees, run Night Shift, promote strategy, open Flash Trade position, wait for yield, close position, redistribute) takes hours and requires mainnet SOL.

### Most Likely Demo Failure Point

The Flash Trade REST API query in `run_flash_trade_demo()`. If `flashapi.trade` is down, rate-limited, or returns unexpected JSON, the price query fails silently. The demo prints "Price query failed (non-fatal)" and continues. A judge watching this will see a gap in the demo narrative.

---

## 6. COMPETITIVE DIFFERENTIATION

Based on Colosseum Copilot searches across 5,400+ builder projects:

### Genuinely Novel

1. **Per-token PDA isolation with on-chain Flash Trade CPI execution** — No other project in the Colosseum corpus combines per-mint treasury PDAs with on-chain perps execution via `invoke_signed`. The closest competitors (Reflect Protocol — Radar Grand Prize, ZoneIn — Cypherpunk) do on-chain hedging or AI trading OS, but none have the "any token adopts, fees route to isolated PDA, PDA signs perps trades" model.

2. **Constitutional enforcement at TWO layers** — Soulguard (Rust) parses `SOULCONTRACT.md` and validates every message. The Anchor program has `require!` constraints mirroring the same invariants. No other project in the corpus has dual-layer constitutional enforcement.

3. **30K configs/night with 9-fold WFA** — The research engine is genuinely sophisticated. Most hackathon "AI trading" projects have a single backtest screenshot. RTP has a production-grade research pipeline with statistical validation.

### Strongest Single Claim

**"The Treasury PDA signs Flash Trade perps positions via invoke_signed — no human keypair exists for trading. The program IS the only authority."**

This is technically true, verifiable on mainnet, and no other project in the Colosseum corpus makes this claim.

### What a Judge Who Has Seen 200 Projects Would Say Is Derivative

1. **"AI agents trading on Solana"** — ZoneIn, Saffron Trade, Wyse all do this. The agent framework space is crowded.
2. **"On-chain treasury management"** — Kamino, Drift, and multiple hackathon winners have treasury/vault infrastructure.
3. **"Multi-agent coordination"** — The 6-wing architecture looks impressive in diagrams, but in practice it's message routing through channels with in-memory state. A cynical judge would see through this.
4. **"Constitutional governance"** — Realms DAO and multiple governance projects have on-chain constitution enforcement. The soulcontract is novel in combining Rust + on-chain, but the concept isn't new.

### Competitive Landscape (Colosseum Corpus)

| Project | Hackathon | Similarity | Key Differentiator |
|---------|-----------|-----------|-------------------|
| ZoneIn | Cypherpunk (Sep 2025) | Low | AI trading OS, portfolio optimization — consumer-facing, not infrastructure |
| Saffron Trade | Cypherpunk (Sep 2025) | Low | AI trading automation for retail — no on-chain treasury or PDA signing |
| Reflect Protocol | Radar (Sep 2024) — **Grand Prize** | Medium | On-chain delta-neutral hedging with LST collateral — won Grand Prize, now C2 accelerator |
| Wyse | Breakout (Apr 2025) | Low | AI yield agents for cross-protocol DeFi strategies — no per-token isolation |
| Vertigo | Breakout (Apr 2025) — **2nd Place Infra** | Low | Sniper-proof DEX for token launches — different problem (fairness vs yield) |

**No project in the corpus combines per-token PDA isolation + on-chain perps CPI + autonomous strategy research.** The closest architectural analog is Reflect Protocol (on-chain hedging), which won Grand Prize at Radar.

---

## 7. NARRATIVE AUDIT

### Is the pitch coherent?

**Mostly.** The README is well-structured and the value proposition is clear: "Any token project adopts RTP — their fees route to an autonomous swarm that generates yield and returns it to holders."

**Non-technical judge test (60 seconds):** "Your token fees don't sit in a wallet doing nothing. An AI swarm puts them to work trading on-chain, and the yield goes back to your community. No one can steal the funds because the program owns the treasury, not a person." — This works.

### Code vs. Pitch Mismatch

| Pitch Claim | Code Reality | Mismatch Severity |
|------------|-------------|------------------|
| "Autonomous swarm" | 6h cron + one-shot binary | **HIGH** |
| "Realtime knowledge graph" | In-memory Vec, files on disk | **MEDIUM** |
| "Self-funding economics" | Yield demonstrated on Hyperliquid testnet (archived), Flash Trade CPI shown once on mainnet | **MEDIUM** |
| "30K configs/night" | Real Python pipeline, validated | **NONE** |
| "On-chain constraint proof" | Real Anchor require! guards | **NONE** |
| "Flash Trade CPI execution" | Real mainnet TXs | **NONE** |
| "SOL never liquidated" | Composability swap-and-open, no SOL sold | **NONE** |

### Strongest Sentence in the README

> "Treasury PDA signs all execution via `invoke_signed` — no private key exists."

### Weakest Sentence in the README

> "Knowledge Wing — Realtime knowledge graph" (it's an in-memory Vec with no persistence)

---

## 8. PRIORITIZED FIX LIST (12 Days Remaining)

| # | Fix | Effort (hrs) | Judge Impact (1-5) | File/Line | Submission Blocker? |
|---|-----|-------------|-------------------|-----------|---------------------|
| 1 | **Add authority check to `update_strategy_performance`** | 0.5 | 5 | `lib.rs` ~line 972, add `require!(ctx.accounts.authority.key() == ctx.accounts.treasury.authority, TreasuryError::UnauthorizedStrategyOp)` | **YES** — any judge who reads the code will flag this |
| 2 | **Wire Flash Trade CPI into the daemon loop** — make the daemon actually call `open_flash_position` and `close_flash_position` on devnet (or show mainnet TXs as proof) | 8 | 5 | `rtp-daemon.rs`, `wings/trading/mod.rs` | **YES** — the demo must show on-chain execution, not just message routing |
| 3 | **Record a 3-minute demo video** — terminal recording of `cargo run --bin rtp-demo` + dashboard walkthrough + Explorer links. Judges review videos before code. | 4 | 5 | New file: `docs/demo-video.md` with Loom link | **YES** — Colosseum requires a video |
| 4 | **Replace `println!()` with `tracing` in the demo path** — 60+ println calls make the demo look amateur | 3 | 3 | `demo.rs`, `wings/trading/mod.rs`, all wings | No, but signals quality |
| 5 | **Populate `data/swarm-memory/working/` and `core/` with real content** — empty directories under "Memory" section will kill judge point 3 | 2 | 4 | `data/swarm-memory/` | No, but judges check filesystem |

### Submission Blockers

- **Missing demo video**: Colosseum submissions typically require a video. Without it, judges may not even review the code.
- **`update_strategy_performance` authority gap**: A security-focused judge (common at Colosseum) will catch this in 30 seconds and dock points.
- **No live on-chain execution in demo**: The demo shows message routing, not Flash Trade CPI. Judges want to see real transactions.

---

## 9. OVERALL VERDICT

### Demo-Readiness Score: **5.5/10**

The Rust demo runs cleanly and shows the 8-step pipeline. But it shows in-memory message routing, not on-chain execution. The Flash Trade CPI path is proven on mainnet (two TXs), but the demo doesn't exercise it live. The dashboard works but shows static fallback data. The "autonomous" loop is a cron job.

### Prize Probability: **MEDIUM**

The project has genuine technical depth — 308 Rust tests, real Anchor program, real Flash Trade CPI mainnet proofs, real 30K-config research pipeline. The on-chain security architecture is above-average for hackathon projects. The per-token PDA isolation model is genuinely novel.

**However:** The gap between what the README promises and what the demo delivers is significant. Judges who read code will find the authority-less `update_strategy_performance`. Judges who watch demos will see message routing, not trading. Judges who have seen 200 projects will recognize the "multi-agent swarm" pattern as overengineered for what it actually does (message routing through tokio channels).

### The One Fix That Most Moves the Needle

**Wire the Flash Trade CPI into the live demo.** Right now the demo has two disconnected proofs: (1) Rust Coordinator routes messages in-memory, (2) TypeScript CLI executes Flash Trade CPI. If you connect them — the daemon reads Night Shift output, the Trading Wing builds the Anchor instruction, the fee-payer submits it, and the position opens on mainnet — you have a complete loop. This turns "architectural promise" into "working system." Estimated 8 hours of focused work.

### The One Genuinely Impressive Thing

**The dual-layer constitutional enforcement.** Soulguard parses `SOULCONTRACT.md` at compile time, validates every message against parsed constraints, and the Anchor program independently enforces the same invariants with `require!` guards. This is not theater — it's real enforcement at two independent layers. A judge who understands governance will recognize this as genuinely thoughtful.

---

**Bottom line:** The bones are strong. The skeleton is real. But the demo doesn't make the skeleton walk. You have 12 days to make it dance.

---

*Audit conducted using Colosseum Copilot API v1.2.1 for competitive landscape research. Source: 5,400+ builder projects, crypto archives, hackathon analytics across 5 Colosseum editions (Hyperdrive through Cypherpunk).*
