================================================================================
  RTP — RESILIENT TOKEN PROTOCOL
  BUILD PLAN v3.0 — POST-AUDIT REMEDIATION
  Supersedes: BUILD_PLAN.md (v2.2) for the weekly schedule
  Audit ref:  docs/SECURITY_AUDIT_2026-04-07.md
  Status:     Weeks 2-4 complete on treasury/swarm path.
              CRITICAL GAP: Hyperliquid perps via Phantom not yet implemented.
================================================================================

CONTEXT:
  We completed the foundation (treasury program on devnet), the Coordinator +
  all 6 wings (Phases 1-3 of v2.2), and a full security audit. This plan
  replaces the v2.2 weekly schedule with one that fixes audit findings first,
  then completes the Hyperliquid execution path.

  EXECUTION VENUE (decided):
  The Trading Wing executes validated strategies as perpetuals trades on
  Hyperliquid, signed via Phantom Connect (agentic wallet). Yield (USDC)
  flows back to the Solana treasury PDA. This is the critical demo path.

  Key links:
    Hyperliquid API:     https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api
    Hyperliquid Rust SDK: https://github.com/hyperliquid-dex/hyperliquid-rust-sdk
    Hyperliquid Python SDK: https://github.com/hyperliquid-dex/hyperliquid-python-sdk
    Hyperliquid testnet: https://api.hyperliquid-testnet.xyz/exchange
    Phantom Connect:     https://docs.phantom.app/phantom-connect/introduction
    CASH stablecoin:     https://docs.phantom.app/phantom-connect/cash
    Solana devnet RPC:   https://api.devnet.solana.com
    Anchor docs:         https://www.anchor-lang.com/docs
    Squads Multisig:     https://docs.squads.so
    Swig wallets:        https://docs.swig.fi
    MoonPay Agents:      https://www.moonpay.com/developers/agents
    Colosseum:           https://arena.colosseum.org
    CORAL paper:         https://arxiv.org/pdf/2604.01658
    autoresearch:        https://github.com/karpathy/autoresearch

================================================================================
  WHAT'S ALREADY DONE
================================================================================

  [x] Repo scaffolded: rtp/swarm/, rtp/programs/rtp-treasury/
  [x] Treasury program (Anchor 1.0) — all CRITICAL/HIGH audit findings fixed
      - initialize, withdraw_fees, check_redistribute, hydrate_swarm,
        evolve_phase, verify_adoption, create_swarm_vault
      - 15 Anchor integration tests passing
  [x] Coordinator with multi-stage quality gate
      - Soulguard (soulcontract enforcement, spec parsing, drift detection)
      - Router (typed routing, exponential backoff, proposal→audit flow)
      - Lifecycle (spawn, heartbeat, health check, retire)
  [x] All 6 wings functional (not stubs)
      - Evolve Wing: assessor + proposer + rollback (5% degradation threshold)
      - Audit Wing: 3-agent tribunal (Skeptic/UserProxy/Optimizer), Byzantine consensus
      - Trading Wing: bridge-backed execution, 5 payload types, in-memory state
      - Security Wing: threat detection, rate-limiting, suspicious-proposal flagging
      - Knowledge Wing: in-memory knowledge graph, cross-wing queries
      - Futureproof Wing: deprecation monitoring, heartbeat
  [x] bridge.rs — typed Python↔Rust interface via subprocess
  [x] demo.rs — 8-step end-to-end demo loop
  [x] devnet-demo.ts — on-chain flow (initialize → fees → redistribute → evolve_phase)
  [x] CI: swarm-ci.yml + python-tests.yml + night_shift.yml
  [x] 264 tests passing, 0 warnings, 0 clippy warnings
  [x] Night shift pipeline operational — top candidate: SOL/USDT Survivor 2.69
  [x] Treasury deployed to devnet — Program 4LvsHbe9LLwgogcDbH7ieTsGcWZctjYFZkzZwaHDM8Ad
  [x] Treasury PDA initialized — FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF
  [x] 8/8 on-chain steps complete (mint, init, adopt, vault, fees, redistribute, hydrate, evolve rejected)
  [x] @phantom/server-sdk v2.0.0 installed — scripts/phantom_signer.ts sidecar ready
  [x] Phantom Portal app "RTP Trading Wing" registered — creds in configs/.env.phantom
  [x] Embedded agent wallet created (KMS-backed, sovereign identity)
  [x] HL testnet API connected (207 assets) + scripts/hl_testnet_demo.py DEPRECATED (EIP-191 wrong)
  [x] HL testnet funded (drip complete)
  [x] ETH keypair generated — configs/hl_testnet_key.json
  [x] Signing architecture decided: Phantom ServerSDK for Solana CPI, ETH keypair for HL EIP-712

  INVARIANT ENFORCEMENT: 9/10
  - Invariant 7 (soulguard reload sig) = documented stub, production TODO
  - All others enforced and tested

================================================================================
  DEMO COVERAGE STATUS (as of Apr 11)
================================================================================

  Judge must verify these 5 points in 3 minutes:

  | Point | Status | Gap |
  |-------|--------|-----|
  | 1. On-chain constraint rejected | PARTIAL | On-chain BelowThreshold exists; needs live validator |
  | 2. Autonomous operation | COVERED | rtp-demo binary runs 8-step pipeline |
  | 3. Persistent memory across cycles | MISSING | memory_promotion.rs built (23 tests) but not in demo binary |
  | 4. Visible adaptation/learning | MISSING | heartbeat.rs built (26 tests) but not in demo binary |
  | 5. Observable treasury state | COVERED (min) | Explorer link live. Dashboard (full) deferred to Phase 5. |

================================================================================
  AUDIT FINDINGS — ALL CRITICAL/HIGH FIXED
================================================================================

  [x] C-2/C-3: Recipient account validation — FIXED
  [x] C-1: Phase evolution threshold enforcement — FIXED
  [x] H-1: Self-referential authority — FIXED
  [x] H-2: Stale vault balance after CPI — FIXED
  [x] H-3: min_runway_balance silent default — FIXED
  [x] H-4: spec() calls unreachable!() — FIXED
  [x] H-5: exceeds_rollback_threshold() ignores spec — FIXED
  [x] M-1: initialize doesn't verify TransferFeeConfig — FIXED
  [x] M-3: Dead code in soulguard Rule 2 — FIXED
  [x] M-4: Router bypass — FIXED (router is pub(crate))
  [x] M-5: stub_review() auto-approves EvolveProposal — FIXED
  [x] I-2: Hardcoded test path — FIXED
  [ ] L-1: HydrateSwarm authority — ACCEPTED (permissionless hydration documented)
  [ ] Invariant 7: soulguard reload sig — DOCUMENTED STUB (production TODO)

================================================================================
  REVISED WEEKLY SCHEDULE
================================================================================

WEEKS 2-4 (Apr 8 – Apr 25): COMPLETE
─────────────────────────────────────────────────────────────────────
  [x] All CRITICAL/HIGH audit findings fixed (Anchor + Rust)
  [x] All 6 wings built and functional
  [x] bridge.rs + demo.rs working end-to-end
  [x] devnet-demo.ts: full on-chain flow demoable
  [x] 238 tests, 0 failures, 0 warnings
  [x] Repo cleaned: stale docs deleted, docs/ reorganised, RESOURCES.md created
  [x] Docs aligned: SESSION-CONTEXT, SOULCONTRACT, CLAUDE.md, BUILD_PLAN_v3, README

WEEK 5 (Apr 28 – May 2): HYPERLIQUID EXECUTION + DEMO POINTS 3/4/5
─────────────────────────────────────────────────────────────────────

  SETUP COMPLETE (Apr 11):
  [x] Phantom Portal app registered, creds in configs/.env.phantom
  [x] Phantom ServerSDK v2.0.0 installed, phantom_signer.ts created
  [x] Embedded agent wallet created for Trading Wing
  [x] HL testnet funded, scripts/hl_testnet_demo.py DEPRECATED (EIP-191 wrong, Rust EIP-712 is reference)
  [x] ETH keypair generated for HL EIP-712 signing
  [x] Treasury deployed to devnet, 8/8 on-chain steps complete
  [x] Explorer link live — judge point 5 covered at minimum

  Priority 1: Hyperliquid execution in Trading Wing (Days 1-3)
  ──────────────────────────────────────────────
  File: rtp/swarm/src/wings/trading/mod.rs
  Resources:
    Hyperliquid API:      https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api
    Hyperliquid Rust SDK: https://github.com/hyperliquid-dex/hyperliquid-rust-sdk
    Testnet endpoint:     https://api.hyperliquid-testnet.xyz/exchange
    Phantom Connect:      https://docs.phantom.app/phantom-connect/introduction

  Steps:
  [ ] Add reqwest + serde_json to rtp/swarm/Cargo.toml
  [ ] Define HyperliquidOrder struct:
        { asset: String, isBuy: bool, limitPx: f64, sz: f64,
          orderType: { limit: { tif: "Gtc" } }, reduceOnly: false }
  [ ] In handle_execute_permit(): build order from TradingConfig payload
  [ ] POST to https://api.hyperliquid-testnet.xyz/exchange
  [ ] Sign request via Phantom Connect agentic wallet API
  [ ] Parse fill response → emit YieldReport with realized PnL
  [ ] CPI transfer: yield USDC → treasury PDA via transfer_checked
  [ ] Test: submit paper trade on HL testnet, verify fill + YieldReport emitted

  Top strategy to execute: SOL/USDT Survivor 2.69
    signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop_atr=0.5

  Priority 2: Demo points 3 + 4 — memory + heartbeat (Day 3)
  ──────────────────────────────────────────────
  File: rtp/swarm/src/demo.rs
  [ ] Extend demo.rs to run TWO orchestrator cycles
  [ ] Cycle 1: execute strategy, emit YieldReport, persist to memory_promotion
  [ ] Cycle 2: load prior memory, reference cycle 1 yield data in log output
  [ ] Trigger heartbeat redirect in cycle 2 (simulate stagnation → redirect)
  [ ] Verify printed output shows: "[MEMORY] referencing cycle 1: ..."
                                   "[HEARTBEAT] redirect triggered: ..."
  This closes judge points 3 and 4 with ~2h of Rust work.

  Priority 3: Demo point 5 — observable treasury state (Days 4-5)
  ──────────────────────────────────────────────
  Option A (fast, ~2h): demo binary prints devnet explorer URL for treasury PDA
    Solana explorer: https://explorer.solana.com/address/{PDA}?cluster=devnet
  Option B (full, ~6h): single-page HTML dashboard
    - Rust demo binary dumps state.json
    - HTML reads state.json + one Solana RPC call (https://api.devnet.solana.com)
    - Shows: treasury balance, last yield event, redistribution splits
  Recommendation: do Option A first (covers judge point 5 minimally),
                  then Option B if time permits.

  DELIVERABLES:
    ✓ Trading Wing places real order on HL testnet via Phantom
    ✓ demo.rs shows two-cycle memory persistence + heartbeat redirect
    ✓ Treasury state visible (explorer URL minimum)

WEEK 6 (May 5‑8): POLISH + SUBMISSION
─────────────────────────────────────────────────────────────────────

  Day 1 (May 5):
  [ ] Demo rehearsal — run 3-minute script end-to-end
  [ ] Verify all 5 judge points covered
  [ ] Final security sweep

  Day 2-3 (May 6-7):
  [ ] Video recording of demo (if browser dashboard not complete)
  [ ] README final polish — demo section updated with actual outputs
  [ ] soulguard reload with signature verification (or confirm documented stub)

  Day 4 (May 8) — HARD DEADLINE:
  [ ] Register individually on Colosseum: https://arena.colosseum.org
      (DO THIS FIRST — blocks submission if missed)

  Day 5-11 (May 9-11):
  [ ] Submit to Colosseum
  [ ] Final anchor test run on devnet
  [ ] Final cargo test run
  [ ] Buffer for emergency fixes
  [ ] DEADLINE: May 11

  DELIVERABLES: All 5 judge points covered, demo recorded, submission in.

================================================================================
  INVARIANT ENFORCEMENT TRACKER
================================================================================

  | # | Invariant                      | Status     | Notes |
  |---|--------------------------------|------------|-------|
  | 1 | PDA owns treasury              | ✅          | — |
  | 2 | TransferFeeConfig immutable    | ✅          | M-1 fixed — verified in initialize |
  | 3 | CPI-only transfers             | ✅          | — |
  | 4 | Agent proposes, human approves | ✅          | H-1 + C-1 fixed |
  | 5 | No SOL liquidation             | ✅          | HL positions are USDC-margined |
  | 6 | Phase transitions irreversible | ✅          | — |
  | 7 | Soulcontract amend = human sig | 📝 STUB    | Production: ed25519 on reload(). Demo unaffected. |
  | 8 | Auto-rollback >5% degradation  | ✅          | H-5 fixed — reads from spec |
  | 9 | Self-hydration >90-day runway  | ✅          | — |
  | 10| Strategies black-boxed         | DEFERRED   | Repo private for collaboration |

  9/10 enforced. Target: 10/10 before May 11.

================================================================================
  RISK REGISTER
================================================================================

  | Risk | Probability | Impact | Mitigation |
  |------|-------------|--------|------------|
  | Hyperliquid testnet API changes | LOW | HIGH | Pin SDK version; test early in Week 5 |
  | Phantom agentic signing complexity | LOW | MEDIUM | ServerSDK v2.0.0 installed, Portal registered, wallet created. Solana CPI path clear. HL uses ETH keypair directly. |
  | demo points 3/4 not demoable | MEDIUM | HIGH | Pure Rust work; 2h estimate is conservative |
  | No dashboard by judging day | MEDIUM | MEDIUM | Explorer URL is minimum viable; covers point 5 |
  | Colosseum registration missed | LOW | CRITICAL | Hard deadline May 4 — calendar it now |
  | Anchor 1.0 breaking changes | LOW | MEDIUM | Pin dependencies in Cargo.toml |

================================================================================
END OF PLAN v3.0
================================================================================
