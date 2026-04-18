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
- The Trading Wing executes validated strategies as **perpetuals trades on Hyperliquid**, signed via Phantom MCP agent wallet (EVM for HL EIP-712, Solana for CPI).
- **Capital flow**: SOL in → Phantom MCP swap → USDC on HL → yield → Phantom MCP swap → SOL back to treasury PDA. Single asset on-chain, USDC only in-flight.
- The redistribution split (70/20/10) is enforced on-chain.
- The swarm accumulates memory, distills strategy knowledge, and improves over repeated market cycles.
- Core claim: agent operations are bounded by on-chain invariants, fully auditable, and designed for token survival over time.
- The B2B integration point is the SDK: launchpads call `createRTPToken()` to create a Token-2022 mint with per-mint treasury PDA in one function call. No RTP token exists — RTP is pure infrastructure.

**Product story (never change this regardless of architecture depth):**
> A launch platform integrates RTP with one function call. Every token it launches gets a program-enforced treasury. An autonomous agent swarm manages that treasury forever under hard on-chain constraints — executing perps strategies on Hyperliquid, returning yield to holders. The agents remember prior cycles, improve strategy over time, and cannot rug because the program forbids it. There is no RTP token — RTP is infrastructure.

---

## 2. Execution Venue — The Hyperliquid + Phantom Path

The execution path is **fully implemented**. BUY→fill→SELL→fill→PnL round-trip verified from Rust. Yield deposits to treasury PDA confirmed on devnet.

### Why Hyperliquid
- Highest-liquidity perps DEX with a documented REST + WebSocket API
- No KYC for programmatic access; supports USDC-margined perpetuals
- API: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api
- Python SDK: https://github.com/hyperliquid-dex/hyperliquid-python-sdk
- Rust SDK (community): https://github.com/hyperliquid-dex/hyperliquid-rust-sdk

### Why Phantom
- Sponsored hackathon resource: https://docs.phantom.com/introduction
- **Phantom MCP Server (`@phantom/mcp-server`)** — gives the AI trading agent a dedicated Phantom wallet with 27 tools.
  - Device-code auth (browser sign-in) — no Portal app ID or API keys needed
  - Agent gets its own wallet — separate from personal wallet, funded independently
  - Session persisted at `~/.phantom-mcp/session.json`
  - **Agent wallet addresses (authenticated Apr 18):**
    - Solana: `AxRWo1N4xjyUN3fbmRpUVwP4WQcEPakdECThyx93CxkR`
    - Ethereum: `0xc1c3b483ec26f5aece1aa25b74de5180fd6dbff8` (used for HL EIP-712 signing)
    - Bitcoin: `bc1qqcy88s30k05q0j2l4x4xzvl2usda6ruhvlq4sd`
    - Sui: `0x9e204a740df615d83b5f1da1f4d5caa47e2fcc36abacf972da6109f08b4ae22a`
  - Key MCP tools for RTP: `buy` (SOL↔USDC swap, fee-free), `perps_deposit` (bridge to HL), `perps_open`, `perps_close`, `perps_withdraw`, `wallet_balances`
  - 28 tools total (discovered via tools/list). Tool names: `buy`, `wallet_addresses`, `perps_deposit`, `perps_withdraw`, `perps_account`, `perps_positions`, `perps_orders`, `perps_markets`, `perps_open`, `perps_close`, `perps_leverage`, `perps_transfer`, etc.
  - **Phantom MCP Rust client** (`phantom_mcp.rs`): starts `@phantom/mcp-server` as subprocess, JSON-RPC over stdio. Provides `quote_sol_to_usdc()`, `swap_sol_to_usdc()`, `quote_deposit_to_hl()`, `deposit_to_hl()`, `withdraw_from_hl()`, `get_perps_account()`, `get_perps_positions()`.
  - Replaces `scripts/phantom_signer.ts` (was based on `@phantom/server-sdk`, now obsolete)
- **Phantom Connect SDK** — for the dashboard's browser extension wallet connection.
  - Portal App ID: `2fbef7dc-7975-4378-ba2b-ff8018ad2325` (registered at https://phantom.app/portal)
  - Dashboard uses `@solana/wallet-adapter-react` + Phantom adapter — works today
- CASH stablecoin (sponsored) is the settlement currency for treasury yield flows

### Execution Flow (target state for demo)
```
Night Shift (Python)
  └── validated strategy config (SOL/USDT Survivor 2.69)
        │
        ▼ bridge.rs (JSON)
Trading Wing (Rust)
  └── ExecutePermit payload
        │
        ▼ Phantom MCP: swap SOL → USDC (fee-free)
        │
        ▼ Phantom MCP: deposit USDC to Hyperliquid
        │
        ▼ Phantom MCP: open_perp_position (SOL/USDT)
        │
        ▼ Phantom MCP: close_perp_position (take profit)
        │
        ▼ Phantom MCP: withdraw USDC from Hyperliquid
        │
        ▼ Phantom MCP: swap USDC → SOL (fee-free)
        │
        ▼ deposit_sol_yield_to_treasury() → Treasury PDA (Solana)
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
| Treasury deployed to devnet (8/8 steps) | ✅ DONE | Program `8rt6yi...`, PDA `FNQbK1...` |
| Phantom ServerSDK v2.0.0 installed + sidecar | ✅ SUPERSEDED | Replaced by Phantom MCP server (`@phantom/mcp-server`). `scripts/phantom_signer.ts` removed. |
| HL testnet API connectivity | ✅ DONE | 207 assets, SOL idx 0, order payload built |
| Phantom MCP Rust client (phantom_mcp.rs) | ✅ DONE | Subprocess MCP client, 28 tools. Swap + bridge quotes working. Perps write 403 (server-side). |
| MCP bridge demo in rtp-demo | ✅ DONE | Swap quote (0.5 SOL → 44.50 USDC) + HL deposit quote (43.14 USDC) via Relay. |
| HL Python integration script (fallback) | ✅ DONE | `scripts/hl_testnet_demo.py` — EIP-712 via web3.py (fallback) |
| Phantom Portal app registered | ✅ DONE | Portal App ID `2fbef7dc-...` for Connect SDK. Agent wallet uses MCP (no Portal creds needed). |
| Unified signing via Phantom MCP | ✅ DONE | `@phantom/mcp-server` installed. Agent wallet authenticated. 27 tools including HL perps + swaps. |
| HL testnet funded | ✅ DONE | ~89.9 USDC in perps clearinghouse. Faucet deposited 100 USDC to spot; transferred 90 to perps via usdClassTransfer. |
| Hyperliquid API call in Trading Wing (Rust) | ✅ DONE | EIP-712 + msgpack signing. Full round-trip verified: BUY 0.12 SOL → fill → SELL → fill → PnL (-$0.004). `serde_json preserve_order` fix was the key. |
| YieldReport PnL calculation | ✅ DONE | Opening: `realized_pnl_usdc = None`. Closing: real PnL computed from entry/exit. |
| PositionState tracking | ✅ DONE | In-memory HashMap, `process_fill()` opens/closes positions, wired into `handle_execute_permit` HL path. |
| Treasury CPI transfer (build tx) | ✅ DONE | `build_treasury_deposit_tx()` builds real SPL `transfer_checked` on devnet. Token-2022 compatible. Manual ATA derivation, manual instruction builder (avoids zeroize conflict). |
| Treasury CPI transfer (sign) | ✅ DONE | Path C: `sign_and_send_local()` signs with devnet keypair (`~/.config/solana/id.json`), submits via JSON-RPC. Signing cascade: Phantom KMS → local keypair → manual fallback. 274 tests 0 failures. |
| Deposit wired into execution path | ✅ DONE | `deposit_sol_yield_to_treasury()` converts USDC PnL to SOL at oracle price, builds native SOL `system_program::transfer` to treasury PDA. Replaces prior SPL token path for yield returns. Phantom → local keypair signing cascade. |
| devnet end-to-end | ✅ DONE | TX builds + signs + submits to devnet. Signature confirmed on-chain: `45DrjL8q...` |
| Phantom wallet connect (dashboard) | ✅ DONE | `@solana/wallet-adapter-react` + Phantom adapter wired on /, /launch, /docs. Live token launch on /launch. |

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

### Night Shift Research Output (live — Apr 12 run, confirmed)
- SOL/USDT candidate #1: Survivor score 2.69 (+2.46 over baseline)
- OOS Sharpe +3.96, 100% consistency (9/9 folds profitable), fragility 0.29, 47 trades/fold
- Config: signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop_atr=0.5
- Status: STRONG RECOMMEND — this is the strategy the Trading Wing executes on Hyperliquid
- Apr 12 night shift (9,888s, 9 folds) CONFIRMED same recommendation — strategy is stable.
- SOL/USDT ADX trend: FALLING (40.6) — monitor for regime transition. Strategy valid while TREND holds.
- BTC overfitting warning: configs with tp_atr=6.0, sl_atr=3.0 flagged overfitting_score=0.57 > threshold.

---

## 5. MVP Boundary

The MVP **is**:
- One constrained Anchor treasury program (done)
- One autonomous orchestration loop (done)
- One bounded swarm coordination mechanism (done)
- One persistent memory layer (built, needs demo wiring)
- **One live Hyperliquid perps trade signed via ETH keypair** (✅ done — round-trip verified on testnet)
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
| Hyperliquid testnet vs mainnet for demo | **DECISION: Testnet** | Safer for hackathon. Same API interface as mainnet. Judges care about the flow working end-to-end. |
| Phantom signing architecture | **DECISION: Path C for demo** | Phantom KMS for production. Local devnet keypair for demo. Signing cascade: Phantom → local → manual. |
| Phantom Portal registration | DONE | App "RTP Trading Wing" registered. Creds in `configs/.env.phantom` (values empty — deferred). |
| Phantom signing scope | DECISION: Solana-focused | ServerSDK for Solana CPI. ETH keypair for HL. Other chains post-hackathon. |

---

## 8. Session Status

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
| MCP config with Portal App ID | `~/.factory/mcp.json` | `PHANTOM_APP_ID=2fbef7dc-...` added to env |

**MCP tools status (this session):**

| Tool | Status | Notes |
|------|--------|-------|
| `buy` (swap) | ✅ Quotes work | 3 routes: OKX, Jupiter, DFlow. Fee-free. Execution needs mainnet SOL. |
| `perps_deposit` (bridge) | ✅ Quotes work | 0.5 SOL → ~43 USDC via Relay. Execution needs mainnet SOL. |
| `wallet_addresses` | ✅ Works | Returns all chain addresses |
| `wallet_balances` | ✅ Works | Token balances with USD prices |
| `perps_account` | ✅ Works | HL account balance (0.0 unfunded) |
| `perps_positions` | ✅ Works | Open positions (empty) |
| `perps_orders` | ✅ Works | Open orders (empty) |
| `perps_markets` | ❌ 403 | `invalid_client` — server-side issue |
| `perps_open/close/leverage` | ❌ 403 | Same server-side issue |

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
| CI push trigger re-enabled | `.github/workflows/swarm-ci.yml` | `on: push: [main]` + `pull_request: [main]` |

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
- **39 anchor tests (5 new beta tests), 306 Rust tests, 18/18 devnet integration tests, 0 failures**
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
- **306 tests (anchor: 34 passing), 0 failures, 0 clippy warnings**
- **SDK signing bug fixed, Phantom wallet wired to dashboard, /docs interactive**
- Demo-Readiness Score: 9.5/10

**SDK fixes (this session):**

| Change | File | Detail |
|--------|------|--------|
| WalletAdapter sendRawTransaction fix | `sdk/index.ts` | Replaced `sendAndConfirmTransaction(connection, signed, [])` with `sendRawTransaction` + `confirmTransaction`. New `sendTx()` helper handles both Keypair and WalletAdapter paths. |
| WalletAdapter overload | `sdk/index.ts` | `withdrawAndRedistribute()` now accepts `Keypair \| WalletAdapter` — mirrors `createRTPToken()` pattern. |
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
- **306 tests (anchor: 34 passing), 0 failures, 0 clippy warnings**
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

**Key design decision:** Yield returns as native SOL (system_program::transfer) to the treasury PDA, not SPL tokens. This matches the mainnet flow: HL USDC profit → Phantom bridge → SOL → treasury PDA. The old SPL token path (`deposit_yield_to_treasury`) is preserved for RTP token distributions.

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
- Devnet addresses: Mint `2JN8Qr9Q...`, Vault `DKuC9Q3F...`, Payer `Driyi8Sw...`
- Dependencies added: `solana-sdk = "2"`, `bincode = "1"`, `base64 = "0.22"`
- `libssl-dev` installed (required by `solana-secp256r1-program` transitive dep)

**Devnet verification:**
- Real blockhash fetched: `8Smg9GWNpxcq99frYwBKgvw36iXKmw6tw6kJFq98xKJZ`
- TX account keys verified: payer [signer], from_ata, treasury_vault, Token-2022, RTP mint
- Phantom sidecar fails (empty creds in `configs/.env.phantom`) — falls back to logging unsigned tx

**Architecture discovery (from RESOURCES.md):**
- Phantom × Hyperliquid native perps: SOL → HL in single Solana tx, no bridge, no EVM wallet
- Phantom MCP Server v0.2.4: 13 tools (swap, sign, manage addresses)
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
- Treasury PDA: `FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF`
- Treasury Vault: `DKuC9Q3FXS28C32k3Grur8QtBLrN5BR5nDsujFkhs3kM`
- Swarm Vault: `E8k82YihuxmX`
- Explorer: https://explorer.solana.com/address/FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF?cluster=devnet
- **All 8 steps completed on-chain:**
  1. ✅ Token-2022 mint with TransferFeeConfig created
  2. ✅ Treasury initialized (phase: sustenance)
  3. ✅ Adoption verified
  4. ✅ Swarm hydration vault created
  5. ✅ 10 simulated trades → fees withdrawn (10,000 tokens)
  6. ✅ Redistribution: 70.0% holders / 20.0% dev / 10.0% ecosystem
  7. ✅ Swarm hydrated (runway invariant enforced)
  8. ✅ Phase evolution correctly rejected (BelowThreshold)
- Redistribution tx: https://explorer.solana.com/tx/9HzWgBfwYxs5ModdjF5mT6gdTfayQq8mMYipopyHfGPmYqk6KESHFqgDrc9Mcie573ttcdPqMHSyJP5nNBKK3bR?cluster=devnet
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

*Last updated: 2026-04-18 (Phantom MCP Rust client built: phantom_mcp.rs subprocess MCP client, 28 tools. MCP bridge wired into Trading Wing + demo. Swap quote (0.5 SOL → 44.50 USDC) + HL deposit quote (43.14 USDC via Relay) working. Perps write 403 server-side issue. 307 tests, 0 failures. Next: fund mainnet wallet, investigate perps 403.)*
*Update this file after each session that changes canonical decisions or resolves open decisions.*
