================================================================================
  RTP — RESILIENT TOKEN PROTOCOL
  BUILD PLAN v3.1 — POST-AUDIT REMEDIATION + DEVNET LOOP
  Supersedes: BUILD_PLAN_v3.md (v3.0)
  Audit ref:  docs/SECURITY_AUDIT_2026-04-07.md
  Status:     ALL CRITICAL PATHS COMPLETE.
              HL round-trip verified. Devnet loop running autonomously (6h cron).
              301 tests, 0 failures, 0 clippy warnings.
================================================================================

CONTEXT:
  We completed the foundation (treasury program on devnet), the Coordinator +
  all 6 wings, a full security audit, the Hyperliquid execution path
  (BUY→fill→SELL→fill→PnL round-trip verified), and an autonomous devnet
  loop daemon that runs on a 6h CI schedule with LLM-driven strategy
  evolution. This plan tracks the remaining polish and submission work.

  EXECUTION VENUE (DONE):
  The Trading Wing executes validated strategies as perpetuals trades on
  Hyperliquid (EIP-712 signed). Yield flows back to the Solana treasury PDA
  via CPI transfer. Treasury deposit TX confirmed on devnet.

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
  [x] CI: swarm-ci.yml + python-tests.yml + night_shift.yml + devnet-loop.yml
  [x] 301 tests passing, 0 failures, 0 clippy warnings
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
  [x] HL round-trip verified: BUY 0.12 SOL → fill → SELL → fill → PnL from Rust code
  [x] serde_json preserve_order fix for HL msgpack key ordering
  [x] YieldReport with realized PnL calculation (long + short, entry/exit tracking)
  [x] PositionState tracking (open/close positions, in-memory HashMap)
  [x] Treasury CPI transfer: build + sign + submit to devnet (TX confirmed on-chain)
  [x] Signing cascade: Phantom KMS → local devnet keypair → manual fallback
  [x] LLM proposer in Evolve Wing: OpenAI-compatible API, deterministic fallback
  [x] validate_mutation_bounds() — soulcontract bounds enforced on LLM proposals
  [x] StrategyConfig + apply_mutations() — config mutation with validation
  [x] Devnet loop daemon (rtp-daemon): single-cycle binary, exits 0, chains config
  [x] devnet-loop.yml: cron every 6h + workflow_dispatch, commits cycle output
  [x] LLM integration live on CI: used_llm: true confirmed
  [x] data/devnet-cycles/ auditable trail whitelisted in .gitignore

  INVARIANT ENFORCEMENT: 9/10
  - Invariant 7 (soulguard reload sig) = documented stub, production TODO
  - All others enforced and tested

================================================================================
  DEMO COVERAGE STATUS (as of Apr 13)
================================================================================

  Judge must verify these 5 points in 3 minutes:

  | Point | Status | Gap |
  |-------|--------|-----|
  | 1. On-chain constraint rejected | ✅ COVERED | `simulate_below_threshold_withdrawal()` + devnet tx link in demo output |
  | 2. Autonomous operation | ✅ COVERED | rtp-demo binary runs 8-step pipeline. Devnet loop runs on 6h cron with zero human input. |
  | 3. Persistent memory across cycles | ✅ COVERED | Two-cycle demo writes real memory files to disk. Devnet loop persists config between runs. |
  | 4. Visible adaptation/learning | ✅ COVERED | LLM proposer suggests mutations, Evolve Wing applies them, devnet loop shows config evolution across cycles. |
  | 5. Observable treasury state | ✅ COVERED | Explorer link live. Devnet deposit TX confirmed. Dashboard = GitHub Pages (next). |

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
  [x] 301 tests, 0 failures, 0 clippy warnings
  [x] Repo cleaned: stale docs deleted, docs/ reorganised, RESOURCES.md created
  [x] Docs aligned: SESSION-CONTEXT, SOULCONTRACT, CLAUDE.md, BUILD_PLAN_v3, README

WEEK 5 (Apr 28 – May 2): COMPLETE
─────────────────────────────────────────────────────────────────────

  [x] HL round-trip verified: BUY→fill→SELL→fill→PnL from Rust
  [x] serde_json preserve_order fix for HL msgpack key ordering
  [x] parse_fill_response fixed (avgPx/totalSz camelCase)
  [x] YieldReport with realized PnL (long + short)
  [x] PositionState tracking (HashMap, process_fill)
  [x] Treasury CPI transfer built + signed + submitted to devnet
  [x] Signing cascade operational (Phantom → local keypair → fallback)
  [x] Deposit wired into handle_execute_permit
  [x] Two-cycle demo with real memory persistence
  [x] All 7 audit gaps closed
  [x] LLM proposer in Evolve Wing (OpenAI-compatible + fallback)
  [x] validate_mutation_bounds() for soulcontract enforcement
  [x] Devnet loop daemon (rtp-daemon) running on 6h CI cron
  [x] LLM integration live on CI (used_llm: true confirmed)
  [x] 301 tests, 0 failures, 0 clippy warnings

  Top strategy executing: SOL/USDT Survivor 2.69
    signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h, trailing_stop_atr=0.5

  Priority 2: GitHub Pages dashboard (stretch)
  ──────────────────────────────────────────────
  [ ] Static site showing treasury state + devnet cycle history
  [ ] Reads data/devnet-cycles/ JSON for cycle evolution charts
  [ ] Devnet explorer links for treasury + deposit TX
  [ ] Deployed via GitHub Pages (no Vercel, no third-party CI)

  DELIVERABLES:
    ✓ Trading Wing places real order on HL testnet
    ✓ demo.rs shows two-cycle memory persistence + heartbeat redirect
    ✓ Treasury state visible (explorer link + deposit TX)
    ✓ Devnet loop running autonomously with LLM-driven evolution

WEEK 6 (May 5‑8): POLISH + SUBMISSION
─────────────────────────────────────────────────────────────────────

  Remaining items:
  [ ] Demo rehearsal — run 3-minute script end-to-end, verify all 5 judge points
  [ ] GitHub Pages dashboard for treasury state (stretch)
  [ ] Video recording of demo (if dashboard not complete)
  [ ] Final security sweep
  [ ] Register individually on Colosseum before May 4: https://arena.colosseum.org
  [ ] Submit to Colosseum by May 11

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
  | Colosseum registration missed | LOW | CRITICAL | Hard deadline May 4 — calendar it now |
  | No dashboard by judging day | MEDIUM | MEDIUM | Explorer URL covers point 5; dashboard is stretch |
  | Anchor 1.0 breaking changes | LOW | MEDIUM | Pin dependencies in Cargo.toml |
  | Devnet RPC rate limits | LOW | LOW | Daemon runs 4x/day, single cycle per run |

================================================================================
END OF PLAN v3.1
================================================================================
