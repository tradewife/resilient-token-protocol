# RTP Session Context

> **How to use this file:** Paste the relevant sections at the top of every fresh agent session. Do not paste the full papers or full repo. This file is the compressed institutional memory of the project. Update it after each significant session.

**Last updated:** 2026-05-12 — Trader watchdog + HTTP timeouts, dashboard mobile fixes, BSL-1.1 rebrand, night-shift Dockerfile fix. 331 unit + 5 integration tests pass.
**Current state:** Live autonomous trader (rtp-trader) running 24/7 on Railway with 9x leverage config: thresh=0.25, tp=5.0, sl=2.7, trail=0.14, align=3. **Trader watchdog:** 120s cycle timeout, 30s HTTP timeouts on all API calls, consecutive error tracking with exponential backoff. `RTP_TRADER_LEVERAGE=9.0` on Railway (was accidentally 1.0 — fixed). Dashboard mobile-responsive (intel panels stack, code block wraps). All 7 Railway services green. License: BSL-1.1.

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
- The Trading Wing executes validated strategies as **on-chain perpetuals via Flash Trade CPI**, signed by the **Treasury PDA via `invoke_signed`** (no human keypair).
- **Capital flow**: SOL in → Treasury PDA invoke_signed → Flash Trade CPI (on-chain) → position opened/closed on Solana → SOL returned to treasury PDA. Single asset, single chain, no cross-chain bridge.
- **Fee-payer wallet**: A funded keypair pays Solana transaction gas (< 0.001 SOL/tx). Has zero authority over treasury funds. Losing this key means losing gas money, not treasury funds.
- The redistribution split (70/20/10) is enforced on-chain.
- The swarm accumulates memory, distills strategy knowledge, and improves over repeated market cycles.
- Core claim: agent operations are bounded by on-chain invariants, fully auditable, and designed for token survival over time.
- The B2B integration point is the SDK: launchpads call `registerWithRTP()` to register a Token-2022 mint with a per-mint treasury PDA in one function call. No RTP token exists — RTP is pure infrastructure.

**Product story (never change this regardless of architecture depth):**
> Token projects route trading fees to RTP → RTP generates yield via on-chain perps → yield flows back to holders. A launch platform integrates RTP with one function call. Every token it launches gets a program-enforced treasury that generates yield via Flash Trade on-chain CPI (invoke_signed, no human key), returning it to holders (70/20/10 on-chain split). The swarm researches, validates, and executes strategies autonomously — funded by its own yield, forever. There is no RTP token. RTP is infrastructure.

---

## 2. Execution Venue — The Flash Trade CPI Path

The execution path is **fully implemented** (M0–M5 complete). Treasury PDA invoke_signed into Flash Trade Perpetuals program confirmed on mainnet. The Hyperliquid/Phantom MCP path is archived behind `#[cfg(feature = "hyperliquid")]`.

### Why Flash Trade
- On-chain Solana perps DEX — no cross-chain bridge, no off-chain signing
- CPI via `invoke_signed` — Treasury PDA signs, no human keypair exists
- Pool-to-peer model, up to 100x leverage, Pyth oracle pricing
- REST API for queries (prices, positions, markets) — execution is CPI only
- Program: `FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn` (mainnet)
- SDK: `flash-sdk` (NPM), reference docs in `flash-trade/` folder

### Why Hyperliquid was replaced
- **Trust liability**: ETH keypair and Phantom MCP session were centralised failure points
- **Verifiability gap**: Trade authorisation happened off-chain (EIP-712), not auditable on Solana
- **Custody mismatch**: Treasury funds had to leave the Solana PDA to reach Hyperliquid
- Flash Trade eliminates all three: PDA signs, execution is on Solana, funds never leave Solana
- **Solana Wallet Adapter** — `@solana/wallet-adapter-react` for browser wallet connection. Supports Phantom, Solflare, Backpack, and any Solana wallet.
  - Dashboard uses `@solana/wallet-adapter-react` + Phantom adapter — works today
- CASH stablecoin (sponsored) — not currently used. Treasury uses USDC for settlement.
- **Phantom MCP (archived)**: gated behind `#[cfg(feature = "hyperliquid")]`, not compiled by default. Historical details in session logs (§8, sessions 2026-04-18/22).

### Execution Flow (target state for demo)
```
Night Shift (Python)
  └── validated strategy config (SOL/USDT Survivor 2.69)
        │
        ▼ bridge.rs (JSON)
Trading Wing (Rust)
  └── ExecutePermit payload
        │
        ▼ Read StrategyRecord on-chain (status must be Live)
        │
        ▼ Read treasury vault balance (must satisfy runway after commit)
        │
        ▼ Build Anchor instruction for open_flash_position
        │   (pre-computed Flash Trade account addresses)
        │
        ▼ Submit tx with fee-payer wallet (gas only)
        │   Treasury PDA signs for the CPI automatically via invoke_signed
        │
        ▼ Flash Trade CPI: position opened on Solana
        │
        ▼ close_flash_position (invoke_signed) → SOL returned
        │
        ▼ update_strategy_performance with realized PnL
        │
        ▼ check_redistribute (on-chain)
           70% to holders wallet / 20% project dev / 10% ecosystem
```

### Current State of Execution Path
| Step | Status | Gap |
|------|--------|-----|
| Strategy validated (SOL/USDT Survivor 2.69) | ✅ DONE | — |
| bridge.rs wires Python → Rust | ✅ DONE | — |
| Trading Wing handles ExecutePermit | ✅ DONE | Flash Trade CPI path wired |
| Treasury deployed to devnet (8/8 steps) | ✅ DONE | Program `8rt6yi...`, PDA `7oZTJW...` |
| Flash Trade CPI viability verified (M0) | ✅ DONE | owner: Signer<'info> confirmed — PDA CPI works |
| Flash Trade CPI mainnet proof (M1) | ✅ DONE | Open TX `2bLg1Fu...` (99,214 CU), Close TX `dFqkoP2...` |
| Flash Trade CPI instructions in lib.rs (M2) | ✅ DONE | open/close/emergency_close_all, 6 errors, 3 events, 3 StrategyRecord fields |
| Trading Wing rewired for Flash Trade (M3) | ✅ DONE | flash_trade_client.rs REST API, phantom_mcp.rs archived behind feature flag |
| Flash Trade CPI integration tests (M4) | ✅ DONE | 9/9 tests passing (frozen, strategy gate, position limits, authority) |
| Flash Trade demo script (M5) | ✅ DONE | scripts/flash-trade-demo.ts + run_flash_trade_demo() in demo.rs |
| HL/Phantom MCP execution path | ✅ ARCHIVED | Gated behind #[cfg(feature = "hyperliquid")], not compiled by default |
| Phantom wallet connect (dashboard) | ✅ DONE | @solana/wallet-adapter-react + Phantom adapter wired on /, /launch, /docs |

**Execution path complete. Remaining work: SDK polish, demo rehearsal, submission.**

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
- memory_promotion.rs: 23+ tests, fully wired into demo binary via Orchestrator::new_for_demo()
- Demo now persists real JSON files under `/tmp/rtp-demo-memory` (project, overview, working), directly visible to judges
- All 4 tiers written and read in the demo — no stubs or hardcoded strings

### Execution Venue (decided)
- **Perps:** Flash Trade (on-chain Solana CPI, invoke_signed)
- **Signing:** Treasury PDA (invoke_signed — no private key exists)
- **Settlement:** SOL throughout — no USDC, no cross-chain bridge
- **On-chain:** Solana mainnet treasury PDA executes Flash Trade CPI

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
- Apply same loop to the Flash Trade CPI execution layer: propose position → simulate → submit via invoke_signed if passes soulguard

### Night Shift Research Output (live — May 5 leverage optimization)
- SOL/USDT 9x Calmar-optimized: Calmar ratio 44.89, +554% compounded return, 12.3% max DD
- Config: signal_threshold=0.25, tp_atr=5.0, sl_atr=2.7, max_hold=36h, trailing_stop_atr=0.14, min_alignment=3, leverage=9.0
- 100% consistency (all WFA folds profitable), 0 liquidations across all 16,228 candidates
- Flash Trade fee model: 0.06% open + 0.06% close + 0.0042%/hr borrow, 20% position sizing
- **Deployed to Railway rtp-trader** — live autonomous trading at 9x leverage
- Robustness testing: Monte Carlo DD p95=32.1%, PBO=33.3% (elevated — optimization may have some overfitting at 9x)
- Strategy exploration: 5 plugins (S02 BB Breakout, S04 RSI Exhaustion, S06 Vol Squeeze, S10 Momentum Divergence, S13 ADX Trend)
- S10 at 9x: +535% PnL, S02 at 9x: +124% PnL — potential alternatives to Survivor
- BTC overfitting warning: configs with tp_atr=6.0, sl_atr=3.0 flagged overfitting_score=0.57 > threshold.

---

## 5. MVP Boundary

The MVP **is**:
- One constrained Anchor treasury program (done)
- One autonomous orchestration loop (done)
- One bounded swarm coordination mechanism (done)
- One persistent memory layer (built, needs demo wiring)
- **One live Flash Trade CPI position signed via Treasury PDA** (✅ done — mainnet-proven, M0–M5 complete)
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

### Current Coverage (as of Apr 14 — post-dashboard telemetry polish)

| Point | Status | Score | How Verified |
|---|---|---|---|
| 1. On-chain constraint rejected | ✅ COVERED | 8/10 | BelowPriceFloor in demo. Dashboard footer: clickable "Rejection proof ↗" (devnet tx) + "BelowThreshold test ↗" (GitHub). demo.sh exits hard if program GC'd. |
| 2. Autonomous operation | ✅ COVERED | 8/10 | 7 devnet cycles committed. rtp-demo 8-step pipeline autonomous. Dashboard: "7 Autonomous Cycles" + last-run timestamp from cycle.json. |
| 3. Persistent memory | ⚠️ PARTIAL | 5/10 | swarm-memory/ has 4 tiers. cycle.json lists 14 files. Dashboard shows memory file count. But working/ and core/ directories are empty — memory is architectural, not yet fully populated on disk. |
| 4. Visible adaptation | ✅ COVERED | 8/10 | Dashboard feed reads from cycle.json — shows real mutations_accepted (3), param diffs, LLM model label. Dynamic wings: Evolve "Active (3 mutations)". No longer hardcoded. |
| 5. Observable treasury state | ✅ COVERED | 9/10 | Treasury SOL live (10s devnet polling). Program liveness badge (green dot). Explorer link. Deployed at resilientprotocol.xyz. Auto-rebuilds every 6h when devnet loop commits. |

**All 5 judge points covered. Point 3 (memory) is honest partial — judges can verify the architecture but not yet rich file content.**

---

## 7. Open Decisions

| Decision | Status | Notes |
|---|---|---|
| Trust model for agent execution | OPEN | Multisig? Optimistic challenge? ZK? Not required for MVP demo. |
| Demo UX | **DECISION: Browser dashboard** | `@solana/wallet-adapter-react` + Phantom. Wallet connect wired in topbar (/), /launch, /docs. Live token launch flow on /launch. |
| Invariant 7 (soulguard reload sig) | CLOSED (documented) | Production TODO: ed25519 on reload(). Comment in soulguard.rs. Demo path unaffected. |
| Hyperliquid testnet vs mainnet for demo | **SUPERSEDED** | Replaced by Flash Trade CPI (on-chain Solana perps). Mainnet CPI proofs: Open TX `2bLg1Fu...`, Close TX `dFqkoP2...`. |
| Phantom signing architecture | **DECISION: Path C for demo** | Phantom KMS for production. Local devnet keypair for demo. Signing cascade: Phantom → local → manual. |
| Phantom Portal registration | DONE | App "RTP Trading Wing" registered. Creds in `configs/.env.phantom` (values empty — deferred). |
| Phantom signing scope | DECISION: Solana-focused | ServerSDK for Solana CPI. ETH keypair for HL. Other chains post-hackathon. |

---

## 8. Session Status

**Session 2026-05-05 — Live Autonomous Trader + Dashboard Real-Time Updates**

State as of May 5:
- **325 unit + 5 integration Rust tests, 0 failures**
- **Live autonomous trader running on Railway (rtp-trader service)**
- **Dashboard at resilientprotocol.xyz showing real-time trader status**
- **7/7 Railway services green**

**What was done (this session):**

| Category | Change | Files |
|----------|--------|-------|
| **New: rtp-trader binary** | Always-on trader running Survivor 2.69, polls Flash Trade every 5 min, executes SOL LONG positions via REST API | `rtp/swarm/src/trader/mod.rs` |
| **HTTP status server** | `start_status_server()` spawns tokio TCP listener on configurable port, serves GET /state (TraderState JSON), GET /health, CORS headers | `rtp/swarm/src/trader/mod.rs` |
| **Shared state** | `Arc<Mutex<TraderState>>` between trading loop and HTTP handler | `rtp/swarm/src/trader/mod.rs` |
| **Dashboard API route** | `/api/trader-status/route.ts` — fetches from trader via Railway private networking, falls back to static file | `dashboard/src/app/api/trader-status/route.ts` |
| **Dashboard Live Trader section** | LIVE badge, position status, trade history, PnL, confirmed mainnet TX links | `dashboard/src/app/page.tsx` |
| **Demo Step 9** | Reads trader-state.json, reports live status in demo output | `rtp/swarm/src/demo.rs` |
| **Dockerfile.trader** | EXPOSE 8080, RTP_TRADER_HTTP_PORT env var | `rtp/swarm/Dockerfile.trader` |
| **Docs** | 3-minute demo script, README Live Trading section, prebuild-data.sh update | `docs/demo-flow.md`, `README.md`, `dashboard/scripts/prebuild-data.sh` |

**Confirmed mainnet transactions (live trader):**

| TX | Type | Detail |
|----|------|--------|
| `MQNU7AbR...` | Open (REST API) | score=0.400, 3 bullish TFs |
| `55BrK7Fi...` | Open (REST API) | Post-redeploy position |
| `4KYd36f9...` | Open (REST API) | Additional position |
| `YtGKq46w...` | Open (REST API) | Listed in dashboard |
| `56PLUQA...` | Close (REST API) | SOL returned |

**Railway services (all 7 green):**

| Service | Status | Notes |
|---------|--------|-------|
| rtp-dashboard | SUCCESS | 200 OK, serving live with /api/trader-status |
| rtp-devnet-loop | SUCCESS | Auto-deployed from push |
| rtp-night-shift | SUCCESS | Cron |
| rtp-swarm-ci | SUCCESS | CI validated |
| rtp-fee-crank | SUCCESS | Hourly |
| rtp-promote-strategy | SUCCESS | Cron |
| rtp-trader | SUCCESS | Always-on, HTTP status server on :8080, live positions |

---

**Session 2026-04-29(iv) — P0 Remediation (5 tasks, real daemon execution)**

State as of Apr 29:
- **312 unit + 5 integration Rust tests, 0 failures** (was 308 — chain_client added 4)
- **Daemon now builds and submits real Flash Trade open/close instructions**
- **5/6 Railway services green** (fee-crank transient crash — devnet RPC issue, not our code)
- **Dashboard live at resilientprotocol.xyz (200 OK)**

**P0 remediation tasks completed (RTP-REMEDIATION-SPEC.md):**

| Task | Detail |
|------|--------|
| P0 #1: Flash Trade CPI account validation | remaining[16] = Flash program, remaining[15] = event authority, remaining[13–14] = system/token programs. 7 new anchor tests. (Done in prior session.) |
| P0 #2: Daemon actually executes transactions | New `chain_client.rs` (625 lines): ChainConfig from env, ExecutionMode simulate/devnet/mainnet, PDA derivation, Anchor instruction builders for open/close, submit_or_simulate with retry. Daemon wired: loads ChainConfig, checks frozen on-chain, builds real open/close IXs, handles stale position close. Demo loop kept as in-process fallback. |
| P0 #3: Remove hardcoded treasury PDAs | Daemon derives all PDAs from ChainConfig::from_env(). Zero hardcoded operational addresses. |
| P0 #4: AdopterRecord.treasury everywhere | Added `AdopterTreasuryMismatch` constraint to HydrateSwarm, RecordFeeDeposit, EndBeta. 3 cross-treasury rejection tests + 1 unauthorized fee attribution test. |
| P0 #5: Fee attribution non-gameable | `record_fee_deposit` now requires `authority.key() == treasury.authority`. Added `UnauthorizedFeeAttribution` error. Random signers cannot inflate adopter contributions. |

**Files changed:**
- `rtp/swarm/src/chain_client.rs` — new file (625 lines)
- `rtp/swarm/src/bin/rtp-daemon.rs` — real execution path
- `rtp/swarm/src/lib.rs` — chain_client module
- `rtp/programs/rtp-treasury/programs/rtp-treasury/src/lib.rs` — 2 new errors, 4 new constraints
- `rtp/programs/rtp-treasury/tests/treasury.ts` — 4 new tests
- `sdk/idl.ts` + `dashboard/src/lib/sdk/idl.ts` — IDL regenerated (v15.3.0)
- `RTP-REMEDIATION-SPEC.md` — all P0 tasks marked done

**Railway deployment status:**
| Service | Status | Notes |
|---------|--------|-------|
| rtp-dashboard | SUCCESS | 200 OK, serving live |
| rtp-devnet-loop | SUCCESS | Auto-deployed from push |
| rtp-night-shift | SUCCESS | Cron, last ran Apr 28 |
| rtp-swarm-ci | SUCCESS | CI validated |
| rtp-promote-strategy | SUCCESS | Cron |
| rtp-fee-crank | CRASHED | Transient devnet RPC issue — not related to our changes |

---

**Session 2026-04-29(iii) — Colosseum Audit Remediation (16 fixes, 5 phases)**

State as of Apr 29:
- **308 unit + 5 integration Rust tests, 0 failures**
- **Anchor program compiles clean with all v1.2 hardening**
- **5/6 Railway services green (fee-crank has pre-existing signTransaction issue)** — **FIXED in follow-up commit**: duck-typing `isKeypair()` replaces `instanceof Uint8Array`. All 6 green.
- **Dashboard live at resilientprotocol.xyz (200 OK)**

**On-chain security fixes (Anchor program — lib.rs):**

| Fix | Severity | Detail |
|-----|----------|--------|
| `update_strategy_performance` authority gate | CRITICAL | Now requires `treasury.authority`. Previously any signer could write arbitrary metrics. |
| `recovery_counter` on StrategyRecord | MEDIUM | Strikes only reset after 3 consecutive positive updates (`MIN_RECOVERY_TRADES`). Single lucky trade cannot clear strikes. |
| `AdopterRecord.treasury` back-reference | LOW | Links adopter records to their treasury for cross-validation. |
| Anchor constraints replace manual `require!` | LOW | `end_beta`, `register_strategy`, `force_retire_strategy`, `update_strategy_performance` use account-level constraints. |
| `open_flash_position` remaining accounts validation | LOW | Validates Flash Trade program ID at `remaining[15]` and treasury PDA at `remaining[0]`. |

**Code quality fixes (Rust swarm):**

| Fix | Detail |
|-----|--------|
| Tracing migration | 233 `println!()` → `tracing::info!()` across 6 files. `tracing-subscriber` added as dependency. |
| Three-state demo status | `StepStatus { Passed, Skipped(reason), Failed(reason) }` replaces boolean `passed`. Bridge-not-available now shows [SKIP] not [PASS]. |
| Bridge file read | `call_bridge()` reads Night Shift `summary.json` first, falls back to subprocess. Works without Python binary installed. |
| Daemon retry layer | `run_cycle_with_retry(3)` with exponential backoff (30s, 60s, 90s). `CycleHealth` + `retry_count` in output. Exit 0 even on failure. |
| Watchdog mode | `RTP_WATCHDOG=1` env flag. Daemon loops forever with configurable `RTP_CYCLE_INTERVAL_SECS` (default 21600 = 6h). |
| Knowledge Wing persistence | JSON file-backed store. Loads from `data/swarm-memory/knowledge/wing-state.json` on startup, persists after every write. |
| FlashTradeClient price caching | Caches last-known prices with timestamp. Graceful degradation when API unavailable. |
| Integration tests | 5 tests in `rtp/swarm/tests/coordinator_integration.rs`: demo loop, knowledge persistence, demo step status, two-cycle coverage. |

**Documentation alignment:**
- "Autonomous swarm" → "Cron-driven autonomous agent swarm" (README.md)
- "Realtime knowledge graph" → "Persistent knowledge store" (README.md, CLAUDE.md)
- Soft decay reset description updated to 3-trade recovery gate (CLAUDE.md)
- `update_strategy_performance` moved from permissionless to authority-gated in trust model (CLAUDE.md)
- Test count updated: 308 unit + 5 integration (CLAUDE.md)
- Daemon description updated with watchdog mode + retry layer (CLAUDE.md)
- Knowledge Wing description updated to "persistent knowledge store (JSON file-backed)" (CLAUDE.md)
- Known remaining gaps updated with FIXED strikethroughs (SESSION-CONTEXT.md)

**Railway deployment (this session):**
| Service | Status | Notes |
|---------|--------|-------|
| rtp-dashboard | SUCCESS | Auto-deployed from push. 200 OK at resilientprotocol.xyz |
| rtp-devnet-loop | SUCCESS | Built with new daemon (tracing, retry, watchdog, knowledge persistence) |
| rtp-swarm-ci | SUCCESS | CI validated (last ran Apr 28) |
| rtp-night-shift | SUCCESS | Cron, last ran Apr 28 |
| rtp-promote-strategy | SUCCESS | Cron, last ran Apr 28 |
| rtp-fee-crank | SUCCESS | Fixed: duck-typing `isKeypair()` replaces `instanceof Uint8Array` for ESM/CJS compat |

---

**Session 2026-04-29(ii) — Operator CLI (`cli/`) + Documentation Alignment**

State as of Apr 29:
- **308 Rust tests, 0 failures**
- **Operator CLI built: 14 commands, interactive onboarding, replaces demo.sh**
- **All 6 Railway services green (dashboard, devnet-loop, night-shift, fee-crank, promote-strategy, swarm-ci)**
- **All root .md files aligned with current architecture**

**What was done (this session):**

| Category | Change | Files |
|----------|--------|-------|
| **New: Operator CLI** | `cli/` directory — Commander.js CLI with 14 commands across 7 groups | `cli/bin/rtp.ts`, `cli/src/commands/*.ts`, `cli/src/config.ts`, `cli/src/keypair.ts`, `cli/src/format.ts`, `cli/src/errors.ts`, `cli/src/lib/railway.ts`, `cli/src/lib/rpc.ts`, `cli/src/lib/safety.ts` |
| **CLI commands** | init, deploy treasury/program, register adopter/strategy, crank fees/redistribute, strategy list/promote/retire, freeze, unfreeze, accounts derive/show, status, status services, demo | All in `cli/src/commands/` |
| **Script refactoring** | Exported async functions from 4 scripts for CLI import: `exportSweepFees`, `exportPromoteStrategy`, `exportFreezeTreasury`/`exportUnfreezeTreasury`/`exportFreezeStatus`, `exportDeriveAccounts` | `scripts/fee-crank.ts`, `scripts/promote-strategy.ts`, `scripts/emergency-freeze.ts`, `scripts/derive_flash_accounts.ts` |
| **Script guards** | All 4 refactored scripts guard `main()` — only runs when executed directly, not when imported | Same 4 files |
| **Archived** | `demo.sh` → `scripts/archive/demo.sh`, `flash-trade-demo.ts` → `scripts/archive/flash-trade-demo.ts` | `scripts/archive/` |
| **Tests** | Unit tests for config resolution, keypair loading, output formatting | `cli/tests/config.test.ts`, `cli/tests/keypair.test.ts`, `cli/tests/format.test.ts` |
| **Docs: CLAUDE.md** | Added CLI commands section (full command reference), CLI key files table, archived script notes | `CLAUDE.md` |
| **Docs: README.md** | Added Operator CLI section, updated Quick Demo (`rtp demo`), added `cli/` to project structure, added CLI to Quick Start and "What We Already Have" table | `README.md` |
| **Docs: SESSION-CONTEXT.md** | Updated last-updated, added this session entry | `SESSION-CONTEXT.md` |
| **Docs: RESOURCES.md** | Added CLI reference section | `docs/RESOURCES.md` |

**CLI architecture:**
- Runtime: `npx tsx cli/bin/rtp.ts <command>` (tsx resolves TypeScript natively)
- Config: `~/.rtp/config.json` (created by `rtp init`), 5-tier resolution (flag > env > local > global > default)
- Cross-directory imports: `../../../sdk/index.ts` and `../../../scripts/*.ts` with `.d.ts` type declarations
- TypeScript compiles clean (`tsc --noEmit`)

**Railway status (all 6 services SUCCESS, pre-rtp-trader):**
| Service | Last Deploy |
|---------|------------|
| rtp-dashboard | 2026-04-29 03:04 UTC |
| rtp-devnet-loop | 2026-04-29 03:04 UTC |
| rtp-fee-crank | 2026-04-29 04:04 UTC |
| rtp-promote-strategy | 2026-04-28 20:03 UTC |
| rtp-night-shift | 2026-04-28 06:31 UTC |
| rtp-swarm-ci | 2026-04-28 06:30 UTC |

---

**Session 2026-04-28 — Full Stack Security Audit + Remediation + Railway Deployment**

State as of Apr 28:
- **308 Rust tests, 0 failures**
- **Security audit remediated: 2 CRITICAL on-chain fixes, 4 panic fixes, async HTTP with retry**
- **All 4 Railway services deployed and healthy**
- **Custom domain resilientprotocol.xyz live (200 OK)**

**What was done (this session):**

| Category | Fix | Files |
|----------|-----|-------|
| **CRITICAL on-chain** | Added PDA seed constraints to `RecordFeeDeposit.treasury` and `RegisterAdopter.treasury` | `rtp/programs/.../lib.rs` |
| **HIGH on-chain** | `size_amount as u64` bounds check before u128→u64 truncation | `rtp/programs/.../lib.rs` |
| **HIGH on-chain** | `FlashSide::None` rejected for open/close with `InvalidFlashSide` error | `rtp/programs/.../lib.rs` |
| **MEDIUM on-chain** | `soft_decay_strikes` resets on recovery (positive PnL + positive Sharpe) | `rtp/programs/.../lib.rs` |
| **CRITICAL Rust** | Replaced 4x `.unwrap()` panics with proper error handling | `trading/mod.rs`, `evolve/mod.rs`, `rtp-daemon.rs` |
| **HIGH Rust** | FlashTradeClient converted to async with retry (3 attempts, exponential backoff) | `flash_trade_client.rs` |
| **CRITICAL Dashboard** | WalletProvider switched from mainnet to devnet RPC | `WalletContextProvider.tsx` |
| **CRITICAL Dashboard** | Frozen state check replaced hardcoded byte offset 225 with SDK `fetchTreasuryState()` | `page.tsx`, `launch/page.tsx` |
| **HIGH Dashboard** | Yield scan reduced from 100→20 signatures, uses explicit devnet connection | `page.tsx` |
| **HIGH Dashboard** | Wallet errors now logged instead of silently swallowed | `WalletContextProvider.tsx` |
| **MEDIUM Dashboard** | Research page shows error state instead of infinite loading | `research/page.tsx` |

**Railway deployment:**
- All 4 services redeployed successfully: dashboard (Online), swarm-ci (Completed), devnet-loop (Online), night-shift (Online)
- Custom domain resilientprotocol.xyz verified and working (200 OK)
- **WARNING**: Do NOT use `railway up` CLI command for redeployment — it wipes custom domain registrations. Use Railway dashboard redeploy button instead. If domains are lost, re-add via GraphQL `customDomainCreate` + `customDomainUpdate`.

**Known remaining gaps (not blocking for hackathon):**
- ~~`update_strategy_performance` accepts arbitrary metrics from any signer~~ **FIXED (v1.2): authority-gated**
- Phase evolution thresholds use raw vault balance, not oracle-denominated USD (acknowledged, post-launch: Pyth/Switchboard oracle)
- ~~No remaining_accounts ownership validation in `open_flash_position`~~ **FIXED (v1.2): validates Flash Trade program ID + treasury PDA**
- ~~`AdopterRecord` has no treasury back-reference~~ **FIXED (v1.2): `treasury: Pubkey` field added**
- Static JSON data files in `public/data/` are 2+ weeks stale (not rebuilt by CI)
- SDK missing wrappers for 8 of 18 instructions (non-essential ones)
- ~~60+ `println!()` in Rust swarm instead of `tracing`~~ **FIXED: replaced with tracing framework**
- ~~Integration test directory is empty~~ **FIXED: 5 integration tests in `tests/coordinator_integration.rs`**

**Priority for next session:**
1. Demo rehearsal — run through docs/demo-flow.md end-to-end
2. Register on Colosseum before May 4 if not done
3. Polish any remaining dashboard rough edges
4. Final submission prep

---

**Session 2026-04-18(ii) — Phantom MCP Rust Client + Bridge Integration**

State as of Apr 18:
- **307 Rust tests (311 with devnet feature), 0 failures**
- **Phantom MCP Rust client built and integrated into Trading Wing**
- **MCP bridge demo working: swap quote (0.5 SOL → 44.50 USDC) + HL deposit quote (43.14 USDC via Relay)**
- Demo-Readiness Score: 9.5/10

**Phantom MCP Rust client (this session):**

| Change | File | Detail |
|--------|------|--------|
| `PhantomMcpClient` | `rtp/swarm/src/wings/trading/phantom_mcp.rs` | Starts `@phantom/mcp-server` as subprocess, JSON-RPC over stdio. 28 tools discovered via `tools/list`. |
| `quote_sol_to_usdc()` | phantom_mcp.rs | Fee-free swap quote via Phantom routing (Jupiter/OKX/DFlow) |
| `swap_sol_to_usdc()` | phantom_mcp.rs | Execute SOL → USDC swap |
| `quote_deposit_to_hl()` | phantom_mcp.rs | Cross-chain bridge quote to HL via Relay |
| `deposit_to_hl()` | phantom_mcp.rs | Execute bridge to HL |
| `withdraw_from_hl()` | phantom_mcp.rs | Withdraw from HL to Solana |
| `get_perps_account()` | phantom_mcp.rs | HL perps account balance |
| `get_perps_positions()` | phantom_mcp.rs | Open perps positions |
| MCP bridge in ExecutePermit | `trading/mod.rs` | New `execution_venue: "phantom_mcp"` triggers MCP bridge before HL trading |
| `mcp_bridge_flow()` | `trading/mod.rs` | Standalone function: swap quote → deposit quote → account check |
| `run_mcp_bridge_demo()` | `demo.rs` | MCP bridge demo step in rtp-demo binary |
| MCP config with Portal App ID | `~/.factory/mcp.json` | `PHANTOM_APP_ID=2fbef7dc-...` added to env (later removed — MCP doesn't use it) |

**MCP tools status (this session):**

| Tool | Status | Notes |
|------|--------|-------|
| `buy` (swap) | ✅ Quotes work | 3 routes: OKX, Jupiter, DFlow. Fee-free. All functions take `di: u32` for per-token isolation. |
| `perps_deposit` (bridge) | ✅ Quotes work | 0.5 SOL → ~43 USDC via Relay. Execution needs mainnet SOL. |
| `wallet_addresses` | ✅ Works | Returns all chain addresses per derivationIndex |
| `wallet_balances` | ✅ Works | Token balances with USD prices |
| `perps_account` | ✅ Works | HL account balance (0.0 unfunded) |
| `perps_positions` | ✅ Works | Open positions (empty) |
| `perps_orders` | ✅ Works | Open orders (empty) |
| `perps_markets` | ❌ 403 | `invalid_client` — server-side issue |
| `perps_open/close/leverage` | ❌ 403 | Same server-side issue |
| `transfer` | ✅ Wrapper built | Rust wrapper for yield distribution |
| `perps_transfer` (spot→perps) | ✅ Wrapper built | Rust wrapper for moving USDC |
| `simulate` | ✅ Wrapper built | Rust wrapper for tx simulation |
| `evm_send` / `solana_send` | ✅ Wrapper built | Rust wrappers for chain-specific txs |

**Known issue:** Perps write operations return 403 `invalid_client`. This is a server-side MCP configuration issue. HL trading via Rust EIP-712 (testnet) continues to work. MCP handles bridging, EIP-712 handles trading.

**Agent wallet funding:**
- Devnet: 2 SOL transferred to `AxRWo1N4xjyUN3fbmRpUVwP4WQcEPakdECThyx93CxkR`
- Mainnet: 0 SOL — needs funding for live MCP execution

**Next session priority:**
1. Fund agent mainnet wallet with SOL for live MCP swap + bridge execution
2. Investigate perps 403 — may need Phantom support escalation
3. Window 2 tasks: Bags.fm integration script, Colosseum team outreach
4. Demo rehearsal with live MCP flow

---

**Session 2026-04-18 — Phantom MCP + Beta SDK + Unified Launch Plan**

State as of Apr 18:
- **307 Rust tests (311 with devnet feature), 0 failures**
- **TypeScript compiles clean (tsc --noEmit)**
- **Phantom MCP agent wallet authenticated — replaces phantom_signer.ts**
- Demo-Readiness Score: 9.5/10

**Phantom MCP agent wallet (this session):**

| Component | Status |
|-----------|--------|
| `@phantom/mcp-server` installed | ✅ `~/.factory/mcp.json` |
| Agent wallet authenticated | ✅ Device-code flow completed |
| Agent Solana address | `AxRWo1N4xjyUN3fbmRpUVwP4WQcEPakdECThyx93CxkR` |
| Agent EVM address (for HL) | `0xc1c3b483ec26f5aece1aa25b74de5180fd6dbff8` |
| Session file | `~/.phantom-mcp/session.json` |
| `scripts/phantom_signer.ts` removed | ✅ Obsolete — MCP replaces it |
| Portal App ID (Connect SDK) | `2fbef7dc-7975-4378-ba2b-ff8018ad2325` |

**Beta adopter SDK (this session):**

| Change | File | Detail |
|--------|------|--------|
| `registerAdopterBeta()` | `sdk/index.ts`, `dashboard/src/lib/sdk/index.ts` | Wraps on-chain `register_adopter_beta` with expiry timestamp |
| `endBeta()` | both SDK copies | Wraps `end_beta` instruction |
| `fetchAdopterState()` | both SDK copies | Reads AdopterRecord PDA — returns beta/permanent status, expiry, deposits |
| `AdopterState` type | both SDK copies | tokenMint, feesContributed, betaExpiresAt, betaEnded, isBeta |
| `deriveAdopterPDA()` | both SDK copies | Seeds: `["adopter", mint]` |
| Beta toggle on /launch | `dashboard/src/app/launch/page.tsx` | Checkbox: "Colosseum Beta — free until May 18". On by default for RTP Direct. Calls `registerAdopterBeta` after mint creation. |
| Adopter state display | `dashboard/src/app/launch/page.tsx` | Post-launch: shows "Beta Adopter" card with expiry date or "Permanent Adopter" |
| Beta CTA banner on home | `dashboard/src/app/page.tsx` | "Colosseum Builders — Try RTP Free" banner with link to /launch |
| CI push trigger (now paused) | `.github/workflows/swarm-ci.yml` | Was `on: push: [main]` + `pull_request: [main]` — now `workflow_dispatch` only (Apr 18) |

**Unified launch plan saved:** `/home/kt/.factory/specs/2026-04-17-rtp-unified-hackathon-mainnet-launch-plan.md`
- Window 1 (now→May 11): SDK beta functions ✅ DONE, dashboard toggle ✅ DONE, CI ✅ DONE
- Window 2 (May 11→18): Colosseum team outreach + Bags.fm integration
- Window 3 (May 12→25): Post-hackathon mainnet deployment (5 phases)

**Next session priority:**
1. Test Phantom MCP tools in fresh session (`get_wallet_addresses`, `buy_token`, `deposit_to_hyperliquid`, `open_perp_position`)
2. Fund agent Solana wallet with SOL for devnet testing
3. Wire MCP tool calls into Trading Wing execution flow
4. Window 2 tasks: Bags.fm integration script, outreach

---

**Session 2026-04-17(ii) — Beta Adopter Lifecycle + Mainnet Audit**

State as of Apr 17:
- **39 anchor tests (5 new beta tests), 307 Rust tests, 18/18 devnet integration tests, 0 failures**
- **Beta adopter lifecycle shipped, mainnet audit completed**
- Demo-Readiness Score: 9.5/10

**Beta adopter lifecycle (this session):**

| Change | File | Detail |
|--------|------|--------|
| AdopterRecord extended | `lib.rs` | Added `beta_expires_at: i64` + `beta_ended: bool` |
| register_adopter_beta | `lib.rs` | New instruction with expiry timestamp, rejects past dates |
| end_beta | `lib.rs` | Authority-gated sunset, sets beta_ended=true, emits BetaEnded |
| hydrate_swarm beta gate | `lib.rs` | Checks beta_expires_at + beta_ended, refuses expired betas |
| HydrateSwarm account | `lib.rs` | Added adopter_record account for beta check |
| Redistribution event | `lib.rs` | check_redistribute now emits Redistribution { mint, excess, holders, dev, ecosystem, ts } |
| New errors | `lib.rs` | BetaExpired, UnauthorizedBetaOp |
| New events | `lib.rs` | BetaEnded, Redistribution |
| Devnet integration tests | `scripts/devnet-beta-test.ts` | 11 scenarios, 18 assertions, all passing on devnet |
| Updated tests | `tests/treasury.ts`, `tests/strategy-lifecycle.ts` | HydrateSwarm now requires adopterRecord account |

**Mainnet audit (this session):**
- Permissionless model confirmed for recording instructions (withdraw_fees, record_fee_deposit, update_strategy_performance, register_adopter) — aligned with trustless design
- Authority-gated for irreversible actions (evolve_phase, register_strategy, force_retire_strategy, end_beta)
- Accepted for launch: oracle-less phase thresholds (C-1), no adopter-treasury linkage constraint (M-1)
- Fixed: redistribution audit event (M-3)
- Program deployed to devnet: slot 456040003, 404,832 bytes
- Trust Model section added to CLAUDE.md — documents permissionless vs authority-gated split

**Key design decision:** Permissionless recording + authority-gated irreversible actions. The PDA owns all treasury assets. Permissionless instructions move funds INTO the PDA or record accounting — never extract. Real enforcement is on-chain via authority checks and status gates.

---

**Session 2026-04-17 — SDK Audit Fixes + Phantom Wallet Integration + Dashboard /docs**

State as of Apr 17:
- **307 tests (anchor: 34 passing), 0 failures, 0 clippy warnings**
- **SDK signing bug fixed, Phantom wallet wired to dashboard, /docs interactive**
- Demo-Readiness Score: 9.5/10

**SDK fixes (this session):**

| Change | File | Detail |
|--------|------|--------|
| WalletAdapter sendRawTransaction fix | `sdk/index.ts` | Replaced `sendAndConfirmTransaction(connection, signed, [])` with `sendRawTransaction` + `confirmTransaction`. New `sendTx()` helper handles both Keypair and WalletAdapter paths. |
| WalletAdapter overload | `sdk/index.ts` | `withdrawAndRedistribute()` now accepts `Keypair \| WalletAdapter` — mirrors `registerWithRTP()` pattern. |
| IDL bundled inline | `sdk/idl.ts` | New file: IDL JSON exported as const. Eliminates `require()` file dependency — works as npm package. |
| anchor.Wallet ESM fix | `sdk/index.ts` | Replaced `import * as anchor` with named imports (`AnchorProvider`, `BorshCoder`, `Program`). Added `kpWallet()` to avoid `anchor.Wallet` not found in ESM build. |

**Dashboard integration (this session):**

| Change | File | Detail |
|--------|------|--------|
| /launch live token flow | `dashboard/src/app/launch/page.tsx` | Full rewrite: wallet connect, form → confirm → Phantom signing → live mint creation on devnet. Shows mint/treasuryPDA/vaultPDA with explorer links. |
| /docs "Try it live" | `dashboard/src/app/docs/page.tsx` | Interactive section: enter mint address → fetch live TreasuryState from devnet. Renders phase, balances, distributions as table. |
| /docs + /launch wallet connect | Both pages | Topbar shows "Connect Wallet" button → wallet modal → connected pill with truncated address. |
| Dashboard footer fix | `dashboard/src/app/page.tsx` | `4LvsHb...M8Ad` → `8rt6yi...2RB` (correct program ID). |
| SDK local copy | `dashboard/src/lib/sdk/` | Copy of SDK with ESM-compatible imports. Needed because Turbopack can't resolve modules from symlinked external directories. |
| Dependencies added | `dashboard/package.json` | `@coral-xyz/anchor`, `@solana/spl-token` for SDK functions. `@resilient-protocol/sdk` as file: link. |

**Key design decision:** Dashboard uses `@solana/wallet-adapter-react` (already installed) for all wallet interactions. The `WalletContextProvider` wraps all pages via `layout.tsx`. Phantom and Solflare adapters configured. This is the standard Solana dApp pattern — no custom Phantom MCP needed for browser-side flows.

---

**Session 2026-04-15(ii) — SOL Yield Return Path + Demo Wiring**

State as of Apr 15:
- **307 tests (anchor: 34 passing), 0 failures, 0 clippy warnings**
- **SOL yield return path + execution_venue wiring + dashboard balance fix**
- Demo-Readiness Score: 9/10

**SOL yield return path (this session):**

| Change | File | Detail |
|--------|------|--------|
| `build_sol_transfer_tx` | `wings/trading/mod.rs` | Builds unsigned native SOL transfer (system_program) from devnet wallet to treasury PDA. Same base64/bincode pattern as existing SPL path. |
| `deposit_sol_yield_to_treasury` | `wings/trading/mod.rs` | Converts USDC PnL to SOL at oracle price → builds SOL transfer → Phantom/local signing cascade → devnet RPC submit. Guards zero-lamport edge. |
| ExecutePermit wiring | `wings/trading/mod.rs` | Replaced `deposit_yield_to_treasury` (SPL token) with `deposit_sol_yield_to_treasury` (native SOL) in the HL fill handler. |
| Demo proposal wiring | `demo.rs` | `execution_venue: "hyperliquid"` + SOL/USDT Survivor 2.69 params in demo loop proposal. Coordinator-mediated path now hits live HL testnet. |
| Dashboard balance | `dashboard/src/app/page.tsx` | Hero balance now polls devnet wallet (`Driyi8Sw...`) instead of treasury PDA (0.0024 SOL rent minimum). Shows ~17.5 SOL. |
| HL account funded | `0xCDe5f236...` | 900 USDC transferred from spot to perps via `usdClassTransfer`. Total: ~989 USDC. |
| 5 new tests | `wings/trading/mod.rs` | `build_sol_transfer_tx_produces_valid_transaction`, `deposit_sol_yield_rejects_zero_lamports`, `deposit_sol_yield_converts_usdc_to_sol_correctly`, `deposit_sol_yield_rejects_negative_pnl`, `deposit_sol_yield_rejects_zero_price`. All passing. |

**Key design decision (historical — pre-Flash Trade):** Yield returns as native SOL (system_program::transfer) to the treasury PDA, not SPL tokens. In the current Flash Trade CPI architecture, positions are opened/closed on Solana and SOL returns directly via CPI — no bridge needed.

---

**Session 2026-04-15 — Strategy Lifecycle + Promotion Gates**

State as of Apr 15:
- **305 tests (anchor: 34 passing), 0 failures, 0 clippy warnings**
- **On-chain strategy lifecycle enforcement + Python promotion/retirement gates**
- Demo-Readiness Score: 9/10

**On-chain strategy lifecycle:**

| Change | File | Detail |
|--------|------|--------|
| StrategyRecord PDA account | `rtp/.../lib.rs` | New account: seeds `[STRATEGY_SEED, treasury, strategy_id]`, fields: status, promoted_at, rolling_pnl_bps, consecutive_losses, soft_decay_strikes, drawdown_24h_bps, total_trades, promotion_sharpe_x100, rolling_sharpe_x100 |
| StrategyLifecycleStatus enum | `rtp/.../lib.rs` | Live, Suspended, Retired |
| RetirementReason enum | `rtp/.../lib.rs` | HardDrawdown, ConsecutiveLosses, RollingSharpeLow, SoftDecayStrikes, AuthorityForced |
| register_strategy instruction | `rtp/.../lib.rs` | Authority-gated promotion: validates strategy_id 1–16 chars, initializes Live, emits StrategyPromoted |
| update_strategy_performance | `rtp/.../lib.rs` | Updates rolling metrics, auto-enforces hard stops (10% DD, 5 losses, Sharpe < 0.5 → Suspended) + soft decay (3 strikes → Retired), emits StrategyPerformanceUpdated + StrategyRetired |
| force_retire_strategy instruction | `rtp/.../lib.rs` | Emergency retirement by treasury authority, emits StrategyRetired(AuthorityForced) |
| hydrate_swarm modified | `rtp/.../lib.rs` | **Critical gate**: requires strategy_record.status == Live. Treasury cannot fund a dead/suspended strategy. |
| On-chain threshold constants | `rtp/.../lib.rs` | HARD_DRAWDOWN_24H_BPS=1000, HARD_CONSECUTIVE_LOSSES=5, HARD_ROLLING_SHARPE_MIN_X100=50, SOFT_STRIKE_THRESHOLD=3 — mirrors Python RetirementGate |
| 3 new events | `rtp/.../lib.rs` | StrategyPromoted, StrategyPerformanceUpdated, StrategyRetired |
| 5 new errors | `rtp/.../lib.rs` | StrategyNotLive, HardStopBreached, SoftDecayRetirement, InvalidStrategyId, UnauthorizedStrategyOp |
| 17 new anchor tests | `tests/strategy-lifecycle.ts` | Register (4), update (6), hydrate gate (3), force retire (2), existing hydrate updated (2). All 34 pass. |

**Python promotion & retirement gates (same session):**

| Change | File | Detail |
|--------|------|--------|
| PromotionGate + RetirementGate | `research/promotion_criteria.py` | 10 promotion thresholds + 3 hard stops + 6 soft signals |
| DecayMonitor | `research/validation/decay_monitor.py` | Rolling window, hard stops + soft decay, returns StrategyStatus |
| Promotion checker | `research/validation/promotion_checker.py` | `check_promotion_eligibility()` → PROMOTE/CONDITIONAL/REJECT |
| Wired into validation | `research/validation/validate_night_shift.py` | Prints PROMOTION ELIGIBILITY block |
| Test suite | `research/validation/test_decay_monitor.py` | 7 pytest tests — all passing |

**Key design decision:** `hydrate_swarm` requiring a Live `StrategyRecord` is the linchinpin — it makes the entire lifecycle system load-bearing rather than advisory. The Python DecayMonitor detects decay; this Rust account enforces the consequence. Together they form the full invariant chain.

---

**Session 2026-04-14(iv) — Multi-Token Attribution Layer**

State as of Apr 14:
- **305 tests (anchor: 19 passing), 0 failures, 0 clippy warnings**
- **Multi-token fee attribution layer added to Anchor treasury program**
- Demo-Readiness Score: 9/10

**Multi-token attribution (this session):**

| Change | File | Detail |
|--------|------|--------|
| AdopterRecord PDA | `rtp/.../lib.rs` | New account: seeds `["adopter", token_mint]`, tracks per-adopter fee contributions |
| register_adopter instruction | `rtp/.../lib.rs` | Creates AdopterRecord PDA for a token mint (once per adopting project) |
| record_fee_deposit instruction | `rtp/.../lib.rs` | Increments per-adopter fees + treasury total_fees_received_lamports |
| Treasury extended | `rtp/.../lib.rs` | Added `total_fees_received_lamports: u64` (pro-rata denominator) |
| Events | `rtp/.../lib.rs` | AdopterRegistered, FeeDepositRecorded |
| Errors | `rtp/.../lib.rs` | ZeroAmount, Overflow (checked_add throughout) |
| Attribution helper | `scripts/compute_adopter_yield_share.ts` | Pure TS: `(fees_contributed * yield_pool) / total_fees` |
| 4 new anchor tests | `tests/treasury.ts` | Registration, deposit, 25%/75% pro-rata, zero rejection. All 19 tests pass. |
| Scaling architecture doc | `dashboard/MULTI_TOKEN_SCALING.md` | Account layout, formula, phase roadmap |
| README updated | `README.md` | Fee Routing section: multi-token attribution design |
| DESIGN.md unchanged | `DESIGN.md` | Reverted — scaling notes moved to dashboard/ |

**Pro-rata formula:** `adopter_yield_share = (fees_contributed / total_fees_received) × yield_pool`

**Phase 1 demo unchanged:** single adopter, single treasury PDA, full redistribution cycle proven on devnet.
**Phase 2 architecture proof:** register_adopter + record_fee_deposit instructions live, AdopterRecord queryable, attribution formula tested.

---

**Session 2026-04-14(iii) — Dashboard Telemetry Polish + Static Deploy**

State as of Apr 14:
- **301 tests, 0 failures, 0 clippy warnings**
- **Dashboard deployed to resilientprotocol.xyz — all CI green**
- **3/3 CI workflows passing: Node.js Build, Deploy Dashboard, Swarm CI**
- Demo-Readiness Score: 9/10

**Dashboard telemetry overhaul (this session):**

| Change | File | Detail |
|--------|------|--------|
| Live cycle feed | `dashboard/src/app/page.tsx` | Replaced hardcoded FEED_LINES with dynamic feed from `/data/cycle.json`. Shows real mutations, param diffs, LLM model. |
| Dynamic wings | `dashboard/src/app/page.tsx` | Wings status derived from cycle data: Evolve shows "Active (3 mutations)", Knowledge shows file count. |
| Liveness badge | `dashboard/src/app/page.tsx` | Green/red dot next to Program ID — client-side devnet RPC check every 30s. |
| Constraint proof links | `dashboard/src/app/page.tsx` | Footer: "Rejection proof ↗" (devnet tx explorer) + "BelowThreshold test ↗" (GitHub source). |
| Cycle + memory metrics | `dashboard/src/app/page.tsx` | Hero section: "7 Autonomous Cycles", last-run timestamp, memory file count. |
| "How it works" accordion | `dashboard/src/app/page.tsx` | Collapsible 3-step pitch for judges, each with explorer/source links. |
| Static data pipeline | `dashboard/scripts/prebuild-data.sh` | Generates `public/data/cycle.json` + `memory.json` from repo data before build. |
| Deploy auto-rebuild | `.github/workflows/deploy-dashboard.yml` | Triggers on `data/**` changes + runs prebuild script. Site refreshes every 6h with new cycle data. |
| Fallback HTML | `dashboard/public/fallback.html` | Self-contained static page with live treasury balance + liveness check. Works with no server. |
| demo.sh hardened | `demo.sh` | Exits on program GC (was silent warning). Added node/npm prereqs. Timestamped summary footer. |
| Live data on static site | All above | Treasury balance + liveness = truly live (client-side RPC). Cycle data = baked at build, auto-refreshes every 6h. |

**Data flow for static export:**
```
devnet-loop.yml (6h cron)
  → commits data/devnet-cycles/latest/cycle.json
  → triggers deploy-dashboard.yml (path filter: data/**)
  → prebuild-data.sh copies to dashboard/public/data/
  → next build (output: "export") bakes into static site
  → GitHub Pages serves updated resilientprotocol.xyz
```

**Client-side live data (no server needed):**
- Treasury SOL balance: `fetch(devnet RPC getBalance)` every 10s
- Program liveness: `fetch(devnet RPC getAccountInfo)` every 30s
- Cycle feed: `fetch(/data/cycle.json)` at page load
- Memory stats: `fetch(/data/memory.json)` at page load

**Previous session — Continual Evolution Infrastructure:**

**Session 2026-04-13(ii) — Continual Evolution Infrastructure**

State as of Apr 13:
- **301 tests, 0 failures, 0 clippy warnings**
- **Devnet loop daemon running autonomously on 6h CI cron**
- **Continual evolution infrastructure built and operational**
- Demo-Readiness Score: 9.5/10

**Continual evolution infrastructure (this session):**

| Component | File | Status |
|-----------|------|--------|
| Strategy library (15 cards) | `research/strategy_library.md` | ✅ 15 strategies: 5 trend, 4 MR, 2 carry, 3 vol, 1 volume |
| Dead ends log | `research/dead_ends.md` | ✅ 9 pre-populated entries (BTC overfitting, XRP dropped, BB failure, etc.) |
| From-scratch prompt | *(removed in audit — was unused)* | N/A |
| Sensitivity sweep | `research/simulation/sensitivity_sweep.py` | ✅ CLI: `python -m research.simulation.sensitivity_sweep --strategy sol_survivor_2_69` |
| Sweep CSV output | `research/data/sensitivity_sol_survivor_2_69.csv` | ✅ 37 rows (baseline + 7 params × 5 steps) |
| Sweep chart | `research/data/sensitivity_sol_survivor_2_69.png` | ✅ 6-panel chart for judge demo |

**SOL Survivor 2.69 sensitivity verdict: ROBUST**
- Average Sharpe range across parameters: **0.30** (target: <1.0 for "flat")
- 5/7 parameters are completely flat (max_hold_hours, time_decay_hours, stop_loss_atr, take_profit_atr, trailing_stop_atr)
- signal_threshold is "peaked" but still 2.98 Sharpe / 7/9 folds at +20% deviation
- `score_flip_delay_hrs` confirmed zero-impact — can be removed from parameter space going forward
- This is demo evidence: "not a lucky backtest, a robust system"

**Strategy library composition (priority-ranked):**
- Priority 1 (implement next): S01 Momentum Persistence, S02 Breakout-Band Expansion, S03 Funding Rate Carry, S04 RSI Exhaustion, S05 BB Bounce, S06 Volatility Squeeze
- Priority 2: S07 Dual MA Cross, S08 MR Band Walk, S09 Funding Momentum, S10 Momentum Divergence, S11 ATR Channel, S12 Multi-TF RSI
- Priority 3: S13 ADX Trend Filter, S14 Vol Regime Switch, S15 CVD Proxy

**Dead ends pre-populated from existing validation data:**
1. BTC wide TP + wide SL overfitting (overfitting_score=0.57)
2. XRP dropped from active symbols (net negative)
3. ETH production baseline marginal (56% consistency)
4. BNB production baseline inconsistent (56% consistency)
5. SOL production baseline suboptimal (resolved by Survivor 2.69)
6. BB Mean Reversion broad failure (trending regime mismatch)
7. High signal threshold >0.45 (over-filters, reduces sample)
8. Long max hold + tight SL (high stop-out rate)
9. SOL production fragility baseline (resolved by Survivor 2.69)

**Execution loop status** (from prior sessions, unchanged):
- HL testnet round-trip verified: BUY → fill → SELL → fill → PnL from Rust
- Treasury CPI transfer to devnet PDA confirmed on-chain
- Full loop: strategy validates → treasury allocates → HL executes → yield returns to PDA → YES, signed HL orders land on testnet from Rust
- Signing: ETH keypair EIP-712 for HL, local devnet keypair for Solana CPI (Path C)

**Session 2026-04-13(i) — Devnet Loop + Autonomous LLM Evolution**

**Devnet loop daemon:**
- `rtp-daemon` binary: single-cycle daemon, loads prior config → orchestrator cycle → LLM/deterministic mutation → apply → persist → exit 0
- `StrategyConfig` + `apply_mutations()` in Trading Wing (3 unit tests)
- `data/devnet-cycles/{timestamp}/cycle.json` — auditable trail
- `data/devnet-cycles/latest/config.json` — config chains between runs
- `devnet-loop.yml` — cron every 6h + workflow_dispatch, `permissions: contents: write`
- LLM secrets configured: `LLM_API_BASE_URL`, `LLM_API_KEY`, `LLM_MODEL`

**Session 2026-04-12 — Full Audit Close-Out + HL Round-Trip**

State as of Apr 12:
- **298 tests, 0 failures, 0 clippy warnings** (now 301)
- **HL testnet funded and verified: BUY → fill → SELL → fill → PnL round-trip from Rust code**
- All 7 audit gaps closed
- Demo-Readiness Score: ~9/10 (was 7/10)

**HL round-trip (this session):**
- Root cause found: `serde_json` default `BTreeMap` sorts keys alphabetically, but HL server re-msgpacks from JSON → different hash than signed. Fix: `preserve_order` feature on serde_json
- Secondary fix: `parse_fill_response` used `avg_px`/`total_sz` (snake_case) but HL returns `avgPx`/`totalSz` (camelCase)
- HL testnet funded: 100 USDC from faucet to spot, 90 USDC transferred to perps via `usdClassTransfer`
- `test_hl_testnet_order` now performs full BUY→fill→SELL→fill→PnL round-trip
- HL account: `0xCDe5f2369f0cE9A8F31E0001dabD3a5A979d1625`, ~89.9 USDC in perps

**Audit close-out (this session):**
- 🔴 #1: Constraint rejection now references real devnet tx (evolve_phase BelowThreshold + redistribution tx explorer link)
- 🔴 #2: Memory loaded from disk in cycle 2 (`fs::read_to_string` on `proj-*.json`), printed as `[MEMORY] ✅ loaded from disk`
- 🟡 #3: `validate_mutation_bounds()` in Evolve Wing — rejects LLM mutations outside soulcontract bounds, 7 new tests
- 🟡 #4: `soulguard_trade_check()` in Trading Wing — enforces 20% position size cap before HL orders, 6 new tests
- 🟡 #5: Live HL vault balance printed in demo output
- 🟢 #6: `sign_action()` deprecated, duplicate `TOKEN_PROGRAM_ID` comment fixed
- 🟢 #7: Audit Wing `stub_review` threshold raised from 0.5 → 0.7, 2 new threshold tests

**Previous session — Two-Cycle Demo (All 5 Judge Points Covered):**

State as of Apr 11:
- **284 tests, 0 failures, 0 clippy warnings**
- Invariant enforcement: 9/10 (Invariant 7 documented stub)
- **All 5 judge points covered in demo binary output**

**Previous session — Agentic Treasury Signing (Path C implemented):**

State as of Apr 11:
- **274 tests, 0 failures, 0 clippy warnings**
- Invariant enforcement: 9/10 (Invariant 7 documented stub)

**Treasury CPI transfer signing (this session — Path C):**
- Path A blocked: Phantom Portal creds empty, `phantom_signer.ts` has TS compilation error (`Property 'name' does not exist on type 'CreateWalletResult'`)
- Path C implemented: `sign_and_send_local()` reads `~/.config/solana/id.json` → signs tx → submits via JSON-RPC
- `load_devnet_keypair()`: loads keypair, verifies pubkey matches `DEVNET_WALLET`
- Signing cascade in `deposit_yield_to_treasury()`: Phantom KMS (production) → local keypair (demo) → manual fallback
- ATA for payer created on devnet: `2Mr35Drmhjrq4xkXoAe2D8QYQV8JhQyQpqcsUpDSWGVB`
- Devnet signature confirmed: `45DrjL8qhP7cpYZyabPa2a8DLfUoJTj55RTcLJWf4x7ThNBT7CBHZRSQszmaTtU4yD3xsFFqAWimTCgMVu1CPk4m`
- Explorer: https://explorer.solana.com/tx/45DrjL8qhP7cpYZyabPa2a8DLfUoJTj55RTcLJWf4x7ThNBT7CBHZRSQszmaTtU4yD3xsFFqAWimTCgMVu1CPk4m?cluster=devnet
- RESOURCES.md corrected: Phantom × HL is "UI feature only, not a programmatic API"
- Demo narrative: "In production, the agent wallet is Phantom KMS-backed. For this demo, we use a devnet keypair to show the same flow."
- `build_treasury_deposit_tx()`: builds `transfer_checked` (Token-2022) instruction, fetches real blockhash from devnet RPC, serializes unsigned tx to base64
- Manual ATA derivation via `Pubkey::find_program_address` (avoids spl-associated-token-account zeroize conflict)
- Manual `transfer_checked` instruction builder (discriminator 12 + amount u64 + decimals u8)
- `call_phantom_signer()`: subprocess call to `ts-node phantom_signer.ts sign-sol <base64>`
- `get_phantom_solana_address()`: parses Solana address from sidecar `addresses` command
- `deposit_yield_to_treasury()`: orchestrates build → sign → send, wired into `handle_execute_permit`
- Devnet addresses: Mint `3yMH4kCB...`, Vault `Fa5Mrv9n...`, Payer `Driyi8Sw...`
- Dependencies added: `solana-sdk = "2"`, `bincode = "1"`, `base64 = "0.22"`
- `libssl-dev` installed (required by `solana-secp256r1-program` transitive dep)

**Devnet verification:**
- Real blockhash fetched: `8Smg9GWNpxcq99frYwBKgvw36iXKmw6tw6kJFq98xKJZ`
- TX account keys verified: payer [signer], from_ata, treasury_vault, Token-2022, RTP mint
- Phantom sidecar fails (empty creds in `configs/.env.phantom`) — falls back to logging unsigned tx

**Architecture discovery (from RESOURCES.md):**
- Phantom × Hyperliquid native perps: SOL → HL in single Solana tx, no bridge, no EVM wallet
- Phantom MCP Server v1.2.x: 28+ tools (swap, sign, perps trading, yield distribution, balance queries)
- This means: yield stays on Solana, no cross-chain bridging needed
- Dependencies added: `reqwest` (rustls-tls), `sha3`, `secp256k1`, `rmp`, `rmp-serde`

**YieldReport PnL calculation** — ✅ DONE
- `parse_fill_response()` calculates realized PnL when entry_price provided
- Opening fill: `realized_pnl_usdc = None`, `entry_price = fill_price`
- Closing fill: Long `(exit - entry) * size`, Short `(entry - exit) * size`
- Mock fill test verified: Open@142.50 → Close@160.00 → PnL = $0.175 USDC

**PositionState tracking** — ✅ DONE
- `PositionState { symbol, side, entry_price, size, opened_at }` in TradingState
- `process_fill()`: opens position on first fill, closes + returns PnL on second fill
- `has_open_position()`, `get_entry_price()` for querying state
- `handle_execute_permit` HL path: checks existing position → passes entry_price → updates position after fill
- Tests: open/close long, open/close short, multiple symbols, loss scenarios

**Mock fill testing** — ✅ DONE
- `mock_fill_response()` helper constructs realistic HL fill JSON
- `mock_fill_opening_then_closing()`: full open→close cycle with PnL verification
- `mock_fill_short_close_with_loss()`: verifies negative PnL on losing short
- No network required — exercises full parse path without HL connectivity

**hl_testnet_demo.py** — ✅ DEPRECATED
- Header clearly states EIP-191 is wrong, points to Rust EIP-712 implementation
- Kept as historical reference for action payload structure

**Anchor treasury deployed to devnet 2026-04-11:**
- Program ID: `8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB`
- Treasury PDA: `7oZTJWYBDjzqmbfRs5YkTv53CDa6vESAzfyjK3yhYshc`
- Treasury Vault: `Fa5Mrv9nTgk46XABZFxSow3RvbX7BJHrksFqBuHMo5ZZ`
- Explorer: https://explorer.solana.com/address/7oZTJWYBDjzqmbfRs5YkTv53CDa6vESAzfyjK3yhYshc?cluster=devnet
- **All 8 steps completed on-chain:**
  1. ✅ Token-2022 mint with TransferFeeConfig created
  2. ✅ Treasury initialized (phase: sustenance)
  3. ✅ Adoption verified
  4. ✅ Swarm hydration vault created
  5. ✅ 10 simulated trades → fees withdrawn (10,000 tokens)
  6. ✅ Redistribution: 70.0% to holders wallet / 20.0% dev / 10.0% ecosystem
  7. ✅ Swarm hydrated (runway invariant enforced)
  8. ✅ Phase evolution correctly rejected (BelowThreshold)
- Init TX: https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet
- Remaining SOL: ~7.51 SOL

**Phantom integration:**
| Component | Status |
|-----------|--------|
| `@phantom/mcp-server` | ✅ Installed + authenticated (replaces server-sdk) |
| Agent wallet (Solana) | ✅ `AxRWo1N4xjyUN3fbmRpUVwP4WQcEPakdECThyx93CxkR` |
| Agent wallet (EVM) | ✅ `0xc1c3b483ec26f5aece1aa25b74de5180fd6dbff8` |
| Portal App ID (Connect SDK) | ✅ `2fbef7dc-7975-4378-ba2b-ff8018ad2325` |
| Local devnet signing (Path C) | ✅ Working — `sign_and_send_local()` signs with `~/.config/solana/id.json` |
| Devnet signature confirmed | ✅ `45DrjL8q...` on-chain |

**Hyperliquid testnet:**
| Item | Status |
|------|--------|
| API connectivity | ✅ Live — 207 perp assets, SOL idx 0 |
| Integration script | ✅ `scripts/hl_testnet_demo.py` — DEPRECATED (EIP-191 wrong; Rust EIP-712 is reference) |
| ETH keypair for EIP-712 | ✅ `configs/hl_testnet_key.json` |
| Order payload built | ✅ SOL/USDT Survivor 2.69 |
| Mock fill testing | ✅ No network required, exercises full parse + PnL path |
| Testnet funded | ✅ ~89.9 USDC in perps clearinghouse |
| Round-trip trade (Rust) | ✅ BUY 0.12 SOL → fill → SELL → fill → PnL verified |
| serde_json key ordering fix | ✅ `preserve_order` feature — IndexMap preserves insertion order |
| parse_fill_response | ✅ Fixed avgPx/totalSz camelCase field names |

**Decisions resolved:**
- Demo UX → Browser dashboard with `@phantom/browser-sdk`
- Testnet vs mainnet → Testnet
- Phantom signing → Solana-focused (ServerSDK for CPI, ETH keypair for HL)

**Priority order for next session (demo rehearsal + submission):**
1. Demo rehearsal — run 3-minute script end-to-end, verify all 5 judge points
2. Register individually on Colosseum before May 4: https://arena.colosseum.org
3. GitHub Pages dashboard for treasury state (stretch — enhances judge point 5)
4. Video recording of demo (if needed)
5. Final security sweep

---

## 9. Key Links (always include these — LLMs go stale without them)

| Resource | URL |
|----------|-----|
| This repo | https://github.com/tradewife/resilient-token-protocol |
| Flash Trade REST API | https://flashapi.trade |
| Flash Trade SKILL.md | `flash-trade/SKILL.md` (in repo) |
| Flash Trade Program (mainnet) | `FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn` |
| Flash Trade Program (devnet) | `FTPP4jEWW1n8s2FEccwVfS9KCPjpndaswg7Nkkuz4ER4` |
| Solana Wallet Adapter | https://github.com/solana-labs/wallet-adapter |
| Squads Multisig | https://docs.squads.so |
| Anchor docs | https://www.anchor-lang.com/docs |
| Solana devnet RPC | https://api.devnet.solana.com |
| Colosseum hackathon | https://arena.colosseum.org |
| CORAL paper | https://arxiv.org/pdf/2604.01658 |
| karpathy/autoresearch | https://github.com/karpathy/autoresearch |

**Legacy (archived behind `#[cfg(feature = "hyperliquid")]`):**
| Resource | URL |
|----------|-----|
| Hyperliquid API docs | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api |
| Hyperliquid Python SDK | https://github.com/hyperliquid-dex/hyperliquid-python-sdk |

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
Flash Trade        = execution venue      (on-chain CPI, PDA-signed)
Treasury PDA       = signing layer        (invoke_signed, no private key)
Demo               = proof the institution persists without founder trust
```

---

---

**Session 2026-04-28 — Documentation Update for Flash Trade Architecture**

State as of Apr 28:
- **308 Rust tests, 0 failures**
- **9/9 Flash Trade CPI tests, 32/32 treasury tests**
- **All root .md files updated to reflect Flash Trade CPI architecture**
- **Hyperliquid/Phantom MCP path archived behind `#[cfg(feature = "hyperliquid")]`**

**Documentation updates (this session):**

| File | Changes |
|------|---------|
| `CLAUDE.md` | Execution venue → Flash Trade CPI. Signing architecture → invoke_signed. Integration resources → Flash Trade + legacy HL. Devnet limitations → Pyth oracle mainnet-only. Key invariants updated (15 state-mutating instructions). |
| `README.md` | Architecture diagram → Flash Trade CPI. Capital flow → single-chain SOL. Trading Wing table updated. Third-party components updated. Demo flow updated. |
| `SESSION-CONTEXT.md` | Execution venue → Flash Trade CPI Path. Execution flow diagram → 6-step CPI. Architecture decisions → PDA signing. Mental model updated. Key links updated. |
| `SOULCONTRACT.md` | Execution venue note → Flash Trade CPI. Capital model → on-chain SOL. Execution constraints → invoke_signed. |
| `docs/RESOURCES.md` | Flash Trade section added. Phantom MCP marked archived. HL marked legacy. |
| `docs/third-party-disclosure.md` | Flash Trade program added. Phantom MCP marked archived. |

**Key architecture addresses:**
| Item | Address |
|------|---------|
| Flash Trade Program (mainnet) | `FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn` |
| Flash Trade Program (devnet) | `FTPP4jEWW1n8s2FEccwVfS9KCPjpndaswg7Nkkuz4ER4` |
| Composability Program (mainnet) | `FSWAPViR8ny5K96hezav8jynVubP2dJ2L7SbKzds2hwm` |
| Perpetuals PDA | `7DWCtB5Z8rPiyBMKUwqyC95R9tJpbhoQhLM9LbK3Z5QZ` |
| Crypto.1 Pool | `HfF7GCcEc76xubFCHLLXRdYcgRzwjEPdfKWqzRS8Ncog` |
| SOL Long Market | `3vHoXbUvGhEHFsLUmxyC6VWsbYDreb1zMn9TAp5ijN5K` |
| SOL Oracle (INT) | `DXqtMo8qRBfHcK11kBnSaCSXkWKk1huMf94R6sAxLHtf` |
| Transfer Authority | `81xGAvJ27ZeRThU2JEfKAUeT4Fx6qCCd8WHZpujZbiiG` |
| Event Authority | `9qb3KAyARHqhVGQjJmzSVJ1hTm3KDR2QL8EBW5paXkUB` |
| M1 Open TX | `2bLg1Fu...` (99,214 CU) |
| M1 Close TX | `dFqkoP2...` |

---

**Session 2026-04-21 — Documentation Audit + Per-Token Isolation Architecture**

State as of Apr 21:
- **307 Rust tests (311 with devnet feature), 0 failures**
- **TypeScript compiles clean**
- **Comprehensive documentation audit completed — 9 inconsistencies fixed**
- **Per-token isolation architecture documented across all surfaces**
- Demo-Readiness Score: 9.5/10

**Documentation audit findings (all fixed):**

| Finding | Severity | Fix |
|---------|----------|-----|
| SOULCONTRACT.md capital flow table said "Token" instead of "SOL" at steps 1 & 5 | T1 | Changed to "SOL" — creator fees are SOL, not the token itself |
| README code example showed `createRTPToken()` — doesn't exist in SDK | T1 | Updated to `registerWithRTP()` with correct API signature |
| Consistency metric mixed: 78% (production) vs 9/9 (optimized) unexplained | T1 | Clarified: "78% → **100%** (optimized)" with footnote |
| Docs architecture box said "8 instructions" — actual count is 14 | T1 | Fixed to 14 |
| Homepage "Phantom perps execution" label — perps are on Hyperliquid, not Phantom | T2 | Changed to "Hyperliquid execution" |
| Homepage capital flow: "Phantom Perps" → should be "HL Perps" | T2 | Fixed labels + "Creator Fees (SOL)" |
| SOULCONTRACT "Phantom signing only" contradicts demo reality (ETH keypair, local keypair) | T2 | Fixed to acknowledge production vs demo path |
| Launch page platform integrations presented as operational but untested on mainnet | T2 | Added "(mainnet)" to all three platform descriptions |
| SDK shown as `npm install` but not published to npm | T3 | Noted — available from GitHub |

**Per-token isolation architecture (new — woven across all surfaces):**

| Surface | Change |
|---------|--------|
| README.md | Rewrote "Multi-Token Attribution" → "Per-Token Isolation — No Shared Pool, No Honeypot" with full copy-trade flow diagram, 4 reasons, Phase 1/Phase 2. Added to architecture invariant list. |
| CLAUDE.md | Added "Per-token isolation" as key invariant #2, renumbered 1-11. Fixed consistency table to match README. |
| SOULCONTRACT.md | Added "Per-token isolation" as constitutional invariant #2 with PDA seeds. Fixed capital flow table SOL. Fixed signing constraint wording. |
| Homepage (page.tsx) | New invariant: "Every token gets its own treasury PDA — no shared pool, no honeypot". New "What We Built" item. Updated "How it works" step 1. |
| Docs overview | New "Why This Is Different" bullet #2. Updated "How It Works" steps. Added "Exploit blast radius?" row to comparison table. |
| Docs Treasury PDA | Expanded Per-Mint Isolation with PDA seeds + callout: "Why Per-Token Isolation Matters". |
| Docs Fee Routing | Added "Per-token isolation" as first Capital Safety property. |
| Docs Security | Added "Isolation" row to security table. Added "Per-token isolation" to on-chain invariants list. |

**Architecture decision:** Per-token isolation with copy-trading. Each token gets its own Treasury PDA + vault (`seeds: ["treasury", mint]`). The swarm copy-trades the same validated strategy (Survivor 2.69) for each token with isolated capital. No shared pool = no honeypot. On-chain already supports it (per-mint seeds). Production scaling: Trading Wing iterates over registered adopters sequentially.

**Key narrative correction confirmed (historical — pre-Flash Trade):** Creator fees from platforms (Pump.fun, Bags.fm, Raydium) are **SOL** — not the token. The current cycle: SOL in → Treasury PDA invoke_signed → Flash Trade CPI → SOL returned. The on-chain `TransferFeeConfig` path is a secondary/supplementary mechanism.

---

**Session 2026-04-18(iii) — Security Fix + Mobile Responsive + Doc Refresh**

State as of Apr 18:
- **307 Rust tests, 0 failures**
- **Security: leaked Pinata JWT removed from launch page** — `PINATA_JWT_FALLBACK` set to empty string
- **Swarm CI disabled (push/PR triggers removed)** — workflow_dispatch only, to conserve Actions minutes
- Demo-Readiness Score: 9.5/10

**Changes this session:**

| Change | File | Detail |
|--------|------|--------|
| Remove leaked Pinata JWT | `dashboard/src/app/launch/page.tsx` | `PINATA_JWT_FALLBACK = ""` — was hardcoded JWT token |
| Rename RTP Direct → RTP DIY | `dashboard/src/app/launch/page.tsx` | Platform name + comment updated |
| Remove "Instant" branding | `dashboard/src/app/launch/page.tsx` | Title: "Token Deploy", removed INSTANT badges + "instantly" copy |
| Mobile responsive CSS | `dashboard/src/app/globals.css` | `.research-section` and `.docs-content`: full width, reduced padding, overflow-x |
| Responsive tables | `dashboard/src/app/globals.css` | `.research-table`: `display: block; overflow-x: auto` |
| Favicon for mobile | `dashboard/src/app/layout.tsx` | Simplified icon config to `/icon.svg` only |
| Favicon move | `dashboard/src/app/favicon.png` → `public/favicon.ico` | Correct location for static export |
| Swarm CI triggers paused | `.github/workflows/swarm-ci.yml` | Removed `push` + `pull_request` triggers, `workflow_dispatch` only |

**Security incident:** A Pinata JWT was hardcoded in `launch/page.tsx` as `PINATA_JWT_FALLBACK`. This was discovered during the commit review. The JWT has been invalidated in Pinata and the fallback set to empty string. The `uploadImageToPinata()` function now requires the Pinata JWT to be passed at runtime (env var or user input), never from a hardcoded fallback.

**CI status:** Swarm CI was the only workflow with active push/PR triggers — it fired (and failed) on every push to main, burning Actions minutes. Now disabled. Re-enable with `gh workflow enable 257875783` before May 11 for one final green CI run.

---

**Session 2026-04-22 — Per-Token Wallet Isolation via derivationIndex**

State as of Apr 22:
- **307 Rust tests (311 with devnet feature), 0 failures**
- **Per-token Phantom wallet isolation implemented via derivationIndex**
- Demo-Readiness Score: 9.5/10

**Per-token wallet isolation (this session):**

| Change | File | Detail |
|--------|------|--------|
| All MCP functions take `di: u32` | `phantom_mcp.rs` | Every function injects `"derivationIndex": di` into MCP tool calls. Per-token wallet isolation from a single auth session. |
| 10 new MCP tool wrappers | `phantom_mcp.rs` | `transfer_spot_to_perps()`, `open_perp_position()`, `close_perp_position()`, `cancel_perp_order()`, `update_perp_leverage()`, `get_token_balances()`, `get_perp_orders()`, `get_perp_trade_history()`, `transfer_tokens()`, `send_solana_transaction()` |
| `TradingState` struct | `trading/types.rs` | `token_wallet_map: HashMap<String, u32>`, `next_derivation_index: u32`, `register_token()`, `get_derivation_index()` |
| Updated MCP callers | `trading/mod.rs` | `mcp_bridge_flow()` and inline MCP bridge both pass `di` (defaulting to 0 with TODO for per-token lookup) |
| Removed `PHANTOM_APP_ID` | `phantom_mcp.rs` | MCP doesn't use it — was dead code |

**Verified: derivationIndex gives separate wallets:**
| Index | Solana | EVM |
|-------|--------|-----|
| 0 | `AxRWo1N4xjyUN3fbmRpUVwP4WQcEPakdECThyx93CxkR` | `0xc1c3b483ec26f5aece1aa25b74de5180fd6dbff8` |
| 1 | `GZa8CuVmdHjbdZQtLzcz7t8LLUqV7sBZXPtnPqz6Q2FP` | `0x5f5da29713bf8e02d8ffe554b0f47bb63ba11066` |
| 2 | `QBM7XE3bN9TQ4FeKXJAtFxgDKUn9VkQeJ4UkcH84BSq` | `0xb7eb912322b8f24ec41daea12cd78ac282ea8849` |

**Next session priority:**
1. Live test: fund index 1 wallet, run full swap→deposit→trade loop
2. Wire `token_wallet_map` into the execute_permit path (currently hardcoded `di: 0`)
3. Persist `TradingState` across daemon restarts (swarm memory or on-chain)
4. Demo rehearsal with per-token isolation visible in output

---

**Session 2026-04-26 — Security Hardening (Freeze + Zero-Address Guard)**

State as of Apr 26:
- **307 Rust tests, 0 failures. Anchor build passes.**
- **Security hardening Phase 1 (on-chain) complete. Squads/Hydra deferred post-hackathon.**
- Demo-Readiness Score: 9.5/10

**On-chain security hardening (this session):**

| Change | File | Detail |
|--------|------|--------|
| Zero-address guard | `lib.rs` | `reject_zero_address()` on `initialize` — rejects `Pubkey::default()` for authority, mint, all 3 wallets |
| Emergency freeze/unfreeze | `lib.rs` | `freeze_treasury` + `unfreeze_treasury` instructions, authority-gated. Events emitted. |
| Frozen flag on Treasury | `lib.rs` | `frozen: bool` field. All 12 state-mutating instructions check it. |
| 4 new errors | `lib.rs` | `ZeroAddressRejected`, `TreasuryFrozen`, `AlreadyFrozen`, `NotFrozen` |
| 2 new events | `lib.rs` | `TreasuryFrozen { mint, authority, timestamp }`, `TreasuryUnfrozen { ... }` |
| SPENDING_LIMIT_EXCEEDED logging | `phantom_mcp.rs` | `tracing::error!()` in `call_tool()` when spending limit hit |
| SDK freeze/unfreeze | `sdk/index.ts` | `freezeTreasury()`, `unfreezeTreasury()`, `isTreasuryFrozen()` functions |
| IDL regenerated | `sdk/idl.ts` | 16 instructions (was 14), includes freeze/unfreeze |
| Dashboard freeze banner | `page.tsx` | Red banner when treasury frozen, polls devnet account data |
| Squads/Hydra deleted | `trading/` | Created then deleted during audit — shelfware, not wired, unvalidated APIs |

**Audit findings that drove the cleanup:**
- Squads program ID in spec was wrong (`a6Eg` should be `aD6E`, `CfH` should be `Cf`)
- Hydra crank had invented serialization — wouldn't produce valid transactions
- Neither module was wired into any execution path (daemon, demo, coordinator)
- Only 4 of 12 instructions had frozen guard — now all 12 do
- Dashboard frozen polling was hardcoded `setIsFrozen(false)` — now reads account data
- Squads/Hydra are post-launch infrastructure, not hackathon requirements

**Docs updated:** CLAUDE.md, SOULCONTRACT.md, RESOURCES.md, SECURITY-HARDENING-SPEC.md

**Next session priority:**
1. Deploy updated program to devnet (`anchor deploy`) — requires re-initializing treasury PDA
2. Dashboard dev server test — verify freeze banner works with real devnet data
3. Demo rehearsal with freeze/unfreeze visible

---

*Last updated: 2026-04-26 (Security hardening Phase 1 complete: freeze/unfreeze, zero-address guard, 12 frozen guards. Squads/Hydra deferred. 307 tests, 0 failures.)*

---

**Session 2026-04-28 — Audit Remediation + Railway CI/CD Migration**

State as of Apr 28:
- **308 Rust tests, 0 failures. Cargo clippy clean. Anchor build clean.**
- **Deep audit completed: 2 P1, 8 P2, 15 P3 findings — all P1/P2 fixed**
- **All 4 services deployed to Railway — CI/CD migrated from GitHub Actions**
- Demo-Readiness Score: 9.5/10

**Audit remediation (this session):**

| Finding | Severity | Fix |
|---------|----------|-----|
| Close discriminator mismatched | P1 | `FLASH_CLOSE_POSITION_DISC` changed from `[123,134,81,0,49,68,98,98]` to `[191,210,137,115,145,22,230,244]` in `lib.rs` — matches mainnet-verified demo script |
| Close data layout wrong (37 bytes → 29 bytes) | P1 | Corrected to disc+OraclePrice+sizeUsd+privilege matching Flash Trade IDL |
| No slippage validation | P2 | `require!(slippage_bps <= 10000)` in both `open_flash_position` and `close_flash_position` |
| No leverage cap | P2 | `require!(leverage_bps <= 1_000_000)` in `open_flash_position` |
| Stale blockhash in tx builder | P2 | `build_treasury_deposit_tx` and `build_sol_transfer_tx` use `Message::new_with_blockhash()` with fetched blockhash |
| Float precision in conversions | P2 | `.round()` added to SOL lamport and token amount float-to-int conversions |
| UTF-8 unsafe error formatting | P2 | `chars().take(200)` replaces byte slicing `&body[..200]` |
| Silent fallback in `get_sol_index` | P2 | Returns `Err("SOL not found")` instead of silent `Ok(0)` |

**Railway CI/CD migration (this session):**

All 4 GitHub Actions workflows paused (triggers commented out, `workflow_dispatch` only). Migrated to Railway:

| Service | Dockerfile | Schedule | Image | Status |
|---------|-----------|----------|-------|--------|
| rtp-dashboard | `dashboard/Dockerfile` | Always-on | SSR Next.js 16 (standalone) | Online |
| rtp-devnet-loop | `rtp/swarm/Dockerfile.daemon` | `0 */6 * * *` | Rust rtp-daemon (1.88) | Cron |
| rtp-night-shift | `research/Dockerfile` | `0 14 * * *` | Python 3.12 | Cron |
| rtp-swarm-ci | `rtp/Dockerfile.ci` | Manual | Rust + Anchor CI | One-shot |

**Railway project details:**
- Project: `resilient-token-protocol` (ID: `11004852-2ba7-46d9-aeb5-ab9558e965a0`)
- Environment: production (ID: `986bee12-1028-4016-aa42-ba0a174233b4`)
- Account: katejcooper.atelier@gmail.com
- Region: Southeast Asia
- API: GraphQL endpoint `https://backboard.railway.com/graphql/v2`
- Cron schedules configured via Railway dashboard Settings (or GraphQL `serviceInstanceUpdate` mutation)

**Devnet-loop Dockerfile fixes:**
- Build context is repo root (Dockerfile Path set in Railway dashboard Settings)
- COPY paths prefixed with `rtp/swarm/` (e.g. `COPY rtp/swarm/Cargo.toml rtp/swarm/Cargo.lock ./`)
- Rust 1.85 → 1.88 (serde_with 3.18, icu_properties_data 2.2 require newer Rust)
- Added `pkg-config` + `libssl-dev` for openssl-sys build dependency
- Verified locally: `docker build -f rtp/swarm/Dockerfile.daemon -t rtp-daemon-test .` passes

**Commits this session:**
- `c9cf27f` fix: audit remediation — 20 actions from AUDIT-REPORT-2026-04-27
- `3d67584` fix: audit fixes — clippy warnings, blockhash, close disc, float precision, utf8 safety
- `1e3aca2` fix: clippy -D warnings (needless borrows) + cargo fmt
- `a6cea5d` feat: Railway deployment — 3/4 services live, GitHub Actions paused
- `840b7bc` feat: Flash Trade CPI integration + audit fixes + CI re-enable (#1)
- `6b0c11a` fix: Dockerfile.daemon — use repo-root COPY paths, rust 1.88, add pkg-config for OpenSSL

---

**Session 2026-04-29 — Pipeline Integrity Audit (Stages 1–9) + Railway Fixes**

State as of Apr 29:
- **308 Rust tests, 0 failures**
- **6 Railway services all green** (dashboard, devnet-loop, fee-crank, promote-strategy, night-shift, swarm-ci)
- **Pipeline integrity spec executed** — all 10 stages audited, critical gaps fixed
- **Dashboard rendering correctly** at resilientprotocol.xyz

**Pipeline integrity audit results (PIPELINE-INTEGRITY-SPEC.md):**

| Stage | Gap | Fix |
|-------|-----|-----|
| **1+6: Fee Crank** | No automated fee withdrawal/redistribution | Created `scripts/fee-crank.ts` + Dockerfile — Railway cron hourly with 0-30min jitter, $5 threshold |
| **3: Strategy Promotion** | No automated strategy registration on-chain | Created `scripts/promote-strategy.ts` + Dockerfile — reads Night Shift summary.json, evaluates against calibrated gate (Sharpe≥2.5, Cons≥70%, Trades/fold≥15, Fragility≤0.40), calls `register_strategy` on-chain |
| **2: Night Shift Handoff** | Bridge subprocess, no shared filesystem between Railway containers | Added `NightShiftSummary`/`NightShiftCandidate` types to `bridge.rs`; daemon reads latest results directly; Night Shift git commits results back to repo |
| **4+5: Daemon Fixes** | Hardcoded byte offset 225 for frozen check; no stale position monitor; no execution mode flag | Replaced with `TreasuryAccount` struct + bincode deserialization; added `check_stale_positions()` with `max_hold_hours × 1.1` timeout; added `RTP_MAINNET_EXECUTE` env flag |
| **9: Emergency Controls** | No cold-start emergency halt path | Created `scripts/emergency-freeze.ts` — CLI freeze/unfreeze/status with local keypair signing |

**SDK fix (isKeypair):**
- `instanceof Keypair` fails across ESM/CJS module boundaries when SDK is dynamically imported
- Replaced with duck-typing: `isKeypair(payer)` checks `secretKey` property instead
- Fixed fee-crank crash (`payer.signTransaction is not a function`)

**Railway infrastructure fixes:**
- Dashboard root directory was `/dashboard` — changed to `/` via GraphQL API (Dockerfile needs `sdk/` from outside `dashboard/`)
- Dashboard Dockerfile static asset paths were wrong — `.next/static` not `dashboard/.next/static`
- Added `RAILPACK_DOCKERFILE_PATH=Dockerfile.dashboard` env var
- Workspace API token stored at `.secrets/railway-workspace-token` (gitignored)
- All service configs verified: root `/`, correct dockerfiles, RAILPACK builder

**Railway services (all green):**

| Service | Type | Schedule | Status |
|---------|------|----------|--------|
| rtp-dashboard | Always-on SSR | — | Online |
| rtp-devnet-loop | Cron | `0 */6 * * *` | Online |
| rtp-fee-crank | Cron | `0 * * * *` | Online |
| rtp-night-shift | Cron | `0 14 * * *` | Completed |
| rtp-promote-strategy | Cron | `30 14 * * *` | Online |
| rtp-swarm-ci | Manual | — | Completed |

**Commits this session:**
- `89a5339` feat: add fee crank cron service
- `c7b82b5` chore: trigger fee-crank rebuild on Railway
- `7798519` feat: add strategy promotion pipeline
- `7c4b719` docs: add rtp-promote-strategy to railway.toml
- `2dd3b96` feat: wire Night Shift → daemon
- `e1d0c90` fix: daemon Stage 4+5 — frozen decode, stale check, mainnet flag
- `14f641b` fix: dashboard static asset paths
- `ff9dd55` chore: bust Docker layer cache
- `20fb95d` docs: Railway config updates
- `258ab65` docs: Railway workspace token location
- `ffb0595` fix: SDK isKeypair() duck-typing
- `9aa6242` feat: add emergency freeze CLI script
