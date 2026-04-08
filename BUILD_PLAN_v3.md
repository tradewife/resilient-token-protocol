================================================================================
  RTP — RESILIENT TOKEN PROTOCOL
  BUILD PLAN v3.0 — POST-AUDIT REMEDIATION
  Supersedes: BUILD_PLAN.md (v2.2) for the weekly schedule
  Audit ref:  docs/SECURITY_AUDIT_2026-04-07.md
  Status:     Week 2+3 complete on the core treasury/swarm path; black-boxing deferred while the repo stays private for collaboration
================================================================================

CONTEXT:
  We completed the foundation (treasury program on devnet) and the
  Coordinator + Evolve Wing prototype (Phases 1-2 of v2.2). A full
  security audit on Day 1 found 3 CRITICAL, 5 HIGH, 5 MEDIUM, 4 LOW,
  and 3 INFO issues. This plan replaces the v2.2 weekly schedule with
  one that fixes the audit findings first, then continues feature work.

  Original BUILD_PLAN.md is preserved — this file only changes the
  SCHEDULE (Parts 5-6). Parts 1-4 and 7-10 from v2.2 still apply.

  SKILL DIRECTIVES: v2.2 mapped 27 skills to build phases. A deep
  audit of every installed skill (see Part A below) found that most
  are stubs, mocks, or documentation for uninstalled npm packages
  (claude-flow, agentic-flow, agentdb, ruv-swarm). This plan retains
  only the skills that provide real value — the rest are dropped to
  avoid wasting time loading tools that produce no output.

================================================================================
  WHAT'S ALREADY DONE (Phases 1-2 from v2.2)
================================================================================

  [x] Repo scaffolded: rtp/swarm/, rtp/programs/rtp-treasury/
  [x] Treasury program compiles (Anchor 1.0)
      - initialize, withdraw_fees, check_redistribute, hydrate_swarm,
        evolve_phase, verify_adoption, create_swarm_vault
  [x] Coordinator with multi-stage quality gate
      - Soulguard (soulcontract enforcement, spec parsing, drift detection)
      - Router (typed routing, exponential backoff, proposal→audit flow)
      - Lifecycle (spawn, heartbeat, health check, retire)
  [x] Evolve Wing complete
      - Assessor (treasury-native scoring, bottleneck detection)
      - Proposer (SPARC lifecycle, status machine)
      - Rollback (5% degradation threshold, Darwinian loop)
  [x] Audit Wing complete
      - 3-agent tribunal (Skeptic, UserProxy, Optimizer)
      - Byzantine/Majority/Weighted consensus
  [x] Types system: Message, Payload, WingId, Priority
  [x] 4 wing stubs: Trading, Security, Knowledge, Futureproof
  [x] 88 tests passing, cargo build clean

================================================================================
  AUDIT FINDINGS — PRIORITIZED FIX PLAN
================================================================================

  BLOCK 1: CRITICAL — Treasury Program (must fix before ANY demo)
  ─────────────────────────────────────────────────────────────────

  C-2/C-3: Recipient Account Validation [CRITICAL]
  ┌─────────────────────────────────────────────────────────────────┐
  │ Problem:                                                        │
  │ • holders_recipient is unchecked AccountInfo — anyone steals 70%│
  │ • dev_recipient constraint compares program owner (Token prog   │
  │   ID) vs wallet pubkey — semantically wrong, always fails       │
  │                                                                 │
  │ Fix:                                                            │
  │ 1. Add `holders_wallet: Pubkey` to Treasury state               │
  │ 2. Accept holders_wallet in initialize()                        │
  │ 3. Change CheckRedistribute:                                    │
  │    holders_recipient: InterfaceAccount<'info, TokenAccount>     │
  │      with token::mint = mint, token::authority = treasury.      │
  │           holders_wallet                                        │
  │    dev_recipient: same pattern with treasury.project_dev_wallet │
  │    ecosystem_recipient: same with treasury.ecosystem_wallet     │
  │ 4. Remove bare AccountInfo + wrong owner checks                 │
  │                                                                 │
  │ File: lib.rs lines 67-96 (state), 500-551 (accounts)           │
  │ Test: anchor test — check_redistribute with wrong recipient     │
  │       should fail                                               │
  └─────────────────────────────────────────────────────────────────┘

  C-1: Phase Evolution Threshold Enforcement [CRITICAL]
  ┌─────────────────────────────────────────────────────────────────┐
  │ Problem:                                                        │
  │ evolve_phase has NO balance check — can advance to Humanity     │
  │ with $0 in treasury. Thresholds are declared but dead_code.     │
  │                                                                 │
  │ Fix (devnet-safe, no oracle dependency):                        │
  │ 1. Add treasury_vault to EvolvePhase account context            │
  │ 2. Enforce vault balance against SUSTENANCE_CAP / ECOSYSTEM_CAP │
  │    (treating token balance as USDC-equivalent for devnet)       │
  │ 3. Remove #[allow(dead_code)] from the constants                │
  │ 4. TODO comment for production oracle integration               │
  │                                                                 │
  │ File: lib.rs lines 348-371 (handler), 599-621 (accounts)       │
  │ Test: evolve_phase with vault below threshold should fail       │
  └─────────────────────────────────────────────────────────────────┘

  H-1: Self-Referential Authority [HIGH]
  ┌─────────────────────────────────────────────────────────────────┐
  │ Problem:                                                        │
  │ treasury.authority = treasury.key() — PDA is its own authority. │
  │ evolve_phase requires PDA to be a Signer, which is impossible  │
  │ without a CPI path that doesn't exist. Dead instruction.        │
  │                                                                 │
  │ Fix:                                                            │
  │ 1. Change initialize: treasury.authority = authority.key()      │
  │    (the payer / initializer becomes the phase authority)        │
  │ 2. Document that production should use a Squads Multisig PDA   │
  │                                                                 │
  │ File: lib.rs line 138                                           │
  │ Test: evolve_phase should succeed when called by initializer    │
  └─────────────────────────────────────────────────────────────────┘

  BLOCK 2: HIGH — Treasury Program (fix same day as Block 1)
  ─────────────────────────────────────────────────────────────────

  H-2: Stale Vault Balance After CPI
  ┌─────────────────────────────────────────────────────────────────┐
  │ Problem:                                                        │
  │ vault.amount is deserialized at instruction entry. After        │
  │ withdraw_withheld_tokens_from_mint CPI, the balance is stale.  │
  │ Delta calculation always returns 0.                             │
  │                                                                 │
  │ Fix:                                                            │
  │ After CPI, add: ctx.accounts.treasury_vault.reload()?;          │
  │ Then compute: let withdrawn = ctx.accounts.treasury_vault       │
  │   .amount.saturating_sub(balance_before);                       │
  │                                                                 │
  │ File: lib.rs lines 195-200                                      │
  └─────────────────────────────────────────────────────────────────┘

  H-3: min_runway_balance = 0 Silent Default
  ┌─────────────────────────────────────────────────────────────────┐
  │ Fix: Change the if-block to require!(min_runway_balance > 0)    │
  │ or remove the misleading "reject 0 explicitly" comment.         │
  │ File: lib.rs lines 150-158                                      │
  └─────────────────────────────────────────────────────────────────┘

  BLOCK 3: HIGH — Swarm Runtime (fix after treasury)
  ─────────────────────────────────────────────────────────────────

  H-4: spec() calls unreachable!()
  ┌─────────────────────────────────────────────────────────────────┐
  │ Fix: Delete the spec() method. spec_snapshot() is the           │
  │ replacement and already exists.                                 │
  │ File: soulguard.rs lines 306-309                                │
  └─────────────────────────────────────────────────────────────────┘

  H-5: exceeds_rollback_threshold() ignores spec
  ┌─────────────────────────────────────────────────────────────────┐
  │ Fix: Store rollback_threshold as an AtomicF64 or std Mutex,     │
  │ update on reload(). Use stored value instead of hardcoded 0.05. │
  │ File: soulguard.rs lines 298-302                                │
  └─────────────────────────────────────────────────────────────────┘

  BLOCK 4: MEDIUM — Swarm Hardening (can defer to Week 3)
  ─────────────────────────────────────────────────────────────────

  M-1: initialize doesn't verify TransferFeeConfig
  Fix: Inline verify_adoption logic into initialize, or call it
       as a helper function.

  M-3: Dead code in soulguard Rule 2
  Fix: Remove Rule 2 (lines 131-144). Rule 1 covers it.

  M-4: Router bypass allows soulguard circumvention
  Fix: Make router pub(crate), or wrap route() to enforce soulguard.

  M-5: stub_review() auto-approves EvolveProposal
  Fix: Make stub_review() reject EvolveProposals (require full
       tribunal), or remove stub_review() entirely.

  BLOCK 5: LOW / INFO (fix opportunistically)
  ─────────────────────────────────────────────────────────────────

  L-1: HydrateSwarm authority not checked → anyone can trigger.
       For hackathon this is acceptable (permissionless hydration
       is documented). Add note in README.

  L-3: env!("CARGO_MANIFEST_DIR") path — fine for hackathon,
       production would use config.

  L-4: edition = "2024" — keep if toolchain supports it, downgrade
       to 2021 if CI breaks.

  I-2: Hardcoded test path — use env!("CARGO_MANIFEST_DIR").

================================================================================
  REVISED WEEKLY SCHEDULE
================================================================================

  Status: Weeks 1-2 from v2.2 are DONE (treasury + coordinator + evolve).
  This schedule covers the remaining 4.5 weeks.

WEEK 2 REMAINDER (Apr 8-11): AUDIT REMEDIATION
─────────────────────────────────────────────────────────────────────

  Day 1 (COMPLETED — Apr 7):
  [x] Full security audit completed
  [x] Audit report saved to docs/SECURITY_AUDIT_2026-04-07.md
  [x] Fix C-2/C-3: recipient account validation
      - Add holders_wallet to Treasury state
      - Fix all three recipient constraints in CheckRedistribute
      - Recompute Treasury::INIT_SPACE (added field)
  [x] Fix H-1: treasury.authority = authority.key()

  Day 2 (COMPLETED — Apr 8):
  [x] Fix C-1: phase evolution threshold enforcement
      - Add treasury_vault to EvolvePhase
      - Enforce SUSTENANCE_CAP / ECOSYSTEM_CAP against vault balance
      - Remove #[allow(dead_code)] from constants
  [x] Fix H-2: vault.reload() after CPI in withdraw_fees
  [x] Fix H-3: reject min_runway_balance == 0 explicitly
  [x] Fix M-1: verify TransferFeeConfig during initialize

  Day 3 (COMPLETED — Apr 9):
  [x] Fix H-4: delete spec() unreachable method
  [x] Fix H-5: read rollback threshold from spec
  [x] Fix M-3: remove dead Rule 2 in soulguard
  [x] Fix M-4: make router pub(crate)
  [x] Fix M-5: stub_review rejects EvolveProposal
  [x] Fix I-2: hardcoded test path

  Day 4 (COMPLETED — Apr 10):
  [x] Write Anchor integration tests for ALL fixed instructions:
      - initialize with valid/invalid mints
      - withdraw_fees with balance tracking
      - check_redistribute with correct/wrong recipients
      - evolve_phase with below/above threshold
      - hydrate_swarm with runway enforcement
  [x] anchor build — verify clean
  [x] anchor test — all passing (15 tests)

  Day 5 (COMPLETED — Apr 11):
  [x] Re-audit: run through all 18 findings, confirm fixed
  [x] Update invariant enforcement table in ONBOARDING.md
  [x] Checkpoint: treasury program is demo-safe
  [x] Code review (2026-04-08): no CRITICAL/HIGH findings
  [x] M-2 reviewed: safe by PDA derivation, no fix needed
  NOTE: No skills needed this week. This is pure Rust/Anchor bug-fixing.
        The audit findings are specific enough to fix without tooling.

  DELIVERABLES: All CRITICAL and HIGH findings fixed, Anchor tests
                passing (15/15), treasury program ready for demo path

WEEK 3 (Apr 14-18): KNOWLEDGE + SECURITY WINGS + BRIDGE ✅ COMPLETE
─────────────────────────────────────────────────────────────────────

  Day 1-2 (COMPLETED — Apr 8, ahead of schedule):
  [x] Create bridge.rs — typed Python↔Rust interface
      - BridgeRequest / BridgeResponse JSON schema (serde)
      - call_bridge() / call_bridge_with_bin() via std::process::Command
      - BridgeError: BinaryNotFound, ProcessFailed, ParseError
      - NIGHT_SHIFT_BIN constant for easy swap in Week 4
      - 10 unit tests (round-trips, mock binary success/failure/malformed)
  [x] Wire Trading Wing beyond stub:
      - Handles 5 payloads: TradingConfig, Proposal, ExecutePermit, YieldReport, Heartbeat
      - ExecutePermit calls bridge.rs → returns YieldReport on success
      - In-memory state: last proposal, last yield report, execution count
      - I-1 fix: unhandled payloads return Payload::Error

  Day 3-4 (COMPLETED — Apr 8):
  [x] Knowledge Wing: in-memory knowledge graph
      - HashMap-based append-only store (key → Vec of timestamped entries)
      - Case-insensitive search with optional context filter
      - Handles 4 payloads: KnowledgeQuery, YieldReport, Assessment, Heartbeat
  [x] Security Wing: threat detection + rate-limiting
      - Suspicious proposal detection: SoulcontractAmendment→Critical,
        RiskThresholdChange→High, PhaseTransition→Medium
      - Rate-limit: 10 proposals/wing/window, stored per-wing counters
      - Alert store with 1-hour expiry, suspicious detections audit-trailed
      - Handles 3 payloads: SecurityAlert, Proposal, Heartbeat
  [x] All 4 new wings wired to Coordinator (no router changes needed —
      default message.to delivery covers SecurityAlert, YieldReport, etc.)

  Day 5 (COMPLETED — Apr 8):
  [x] All 6 wings respond to ≥2 payload types each:
      Trading(5), Security(3), Knowledge(4), Evolve(3), Audit(2+), Futureproof(2)
  [x] Futureproof Wing: heartbeat + deprecation check (7 crates monitored)
  [x] I-1 fix: all wings return Payload::Error for unhandled types (no silent drops)
  [x] Code review: 4 findings fixed (TOCTOU, alert audit trail, ETXTBSY race,
      solana-sdk version string)
  [x] Integration tests: 6 new tests (full suite: 133 tests, 0 failures, 0 warnings)
  [x] Treasury program: still compiles clean after all swarm changes

  DELIVERABLES: bridge.rs working, all 6 wings functional (not stubs),
                full message loop demoable. 88→133 tests (+45).

WEEK 4 (Apr 21-25): FULL LOOP + BLACK-BOXING
─────────────────────────────────────────────────────────────────────

  Day 1-2:
  [ ] Black-box Python fractal-swarm: pyinstaller → night_shift.bin
  [ ] Encrypted configs (AES, build-time key)
  [ ] End-to-end: Python proposes → bridge.rs → Audit tribunal
      → Coordinator routes → Trading executes → YieldReport

  Day 3-4:
  [ ] Full loop demo on devnet:
      1. Token adopts RTP (initialize + verify_adoption)
      2. Fees accumulate (withdraw_fees)
      3. Threshold hit → check_redistribute (70/20/10)
      4. Swarm proposes strategy → audit approves → execute
      5. Self-hydration (hydrate_swarm with runway check)
  [ ] MoonPay Agents integration (if time permits)
  [ ] Phase evolution demo (evolve_phase with threshold enforcement)

  Day 5:
  [ ] GitHub Actions CI: cargo build + cargo test + anchor build
      SKILL: Load github-workflow-automation — extract the GitHub
      Actions YAML templates for Rust/Anchor CI. Ignore all
      ruv-swarm and claude-flow references in the skill. You only
      need the workflow YAML structure for:
        - matrix: [cargo build, cargo test, anchor build]
        - cache: cargo registry + target dir
        - timeout: 30 min
  [ ] Weekly checkpoint: full end-to-end loop demoable on devnet

  DELIVERABLES: Complete loop, source-visible research pipeline, devnet demo

WEEK 5 (Apr 28 - May 2): POLISH + HARDENING
─────────────────────────────────────────────────────────────────────

  Day 1-2:
  [ ] Demo flow rehearsed (3 minutes):
      1. Token adopts RTP — TransferFeeConfig enabled
      2. Trading fees auto-route to Treasury PDA
      3. Swarm researches, validates, executes yield strategy
      4. Reserves hit threshold → live redistribution tx
      5. Verify: project + holders receive yield, SOL untouched
      SKILL: Load walkthrough (builtin) — generate Mermaid diagrams
      of the 3-layer architecture and fee flow for README and demo
      slides. This is the one skill that directly helps judges
      understand the system.
  [ ] soulguard reload with signature verification
      (or document as production TODO)

  Day 3-4:
  [ ] Final security sweep
      SKILL: Load code-review (builtin) — run formal review on the
      full treasury program diff since the audit. This catches
      regressions. Do NOT load red-team-tribunal — it returns fake
      hardcoded results. Amp's built-in code-review is the real tool.
  [ ] README polished — invariant table shows enforced status
  [ ] third-party-disclosure.md updated
  [ ] Video recording of demo

  Day 5:
  [ ] Colosseum Copilot final check
  [ ] Buffer for last-minute fixes

  DELIVERABLES: Polished demo, hardened treasury, submission-ready

WEEK 6 (May 5-11): SUBMISSION
─────────────────────────────────────────────────────────────────────

  Day 1-2:
  [ ] Register individually (deadline May 4 — DO THIS FIRST)
  [ ] Final anchor test run on devnet
  [ ] Final cargo test run

  Day 3-4:
  [ ] Submit to Colosseum
  [ ] Buffer for emergency fixes

  Day 5 (May 11):
  [ ] DEADLINE — submission must be in

================================================================================
  INVARIANT ENFORCEMENT TRACKER
================================================================================

  Track progress fixing the 10 invariants. Goal: all ✅ by Week 4.

  | # | Invariant                    | Status | Fix Planned |
  |---|------------------------------|--------|-------------|
  | 1 | PDA owns treasury            | ✅     | —           |
  | 2 | TransferFeeConfig immutable  | ✅     | M-1: FIXED — verify in initialize |
  | 3 | CPI-only transfers           | ✅     | —           |
  | 4 | Agent proposes, human approves | ✅   | H-1 + C-1: FIXED — authority + thresholds |
  | 5 | No SOL liquidation           | ✅     | —           |
  | 6 | Phase transitions irreversible | ✅    | —           |
  | 7 | Soulcontract amend = human sig | ⚠️    | Week 5: reload signature check |
  | 8 | Auto-rollback >5% degradation | ✅     | H-5: FIXED — read from spec |
  | 9 | Self-hydration >90-day runway | ✅     | —           |
  | 10| Strategies black-boxed       | DEFERRED | repo kept private for collaboration |

  Target: 9/10 enforced now. Invariant 7 is Week 5.
  M-2 (has_one on mint in Initialize) reviewed 2026-04-08: safe by PDA derivation.
  No fix needed.

  Full skill audit: ~/tabs/SKILL_AUDIT_2026-04-07.md
  (50+ skills/plugins audited — 3 useful, rest dropped)

================================================================================
  RISK REGISTER
================================================================================

  | Risk | Probability | Impact | Mitigation |
  |------|-------------|--------|------------|
  | Treasury bugs block demo | HIGH (now) | CRITICAL | Block 1-2 fixes (Days 1-2) |
  | bridge.rs not ready | MEDIUM | HIGH | Week 3 Day 1-2 priority |
  | Anchor 1.0 breaking changes | LOW | MEDIUM | Pin dependencies in Cargo.toml |
  | Python binary fails on judge machine | MEDIUM | HIGH | Test on clean Ubuntu, include fallback |
  | soulguard bypass via Router | MEDIUM | HIGH | M-4 fix (Day 3) |
  | CI breaks on edition 2024 | LOW | LOW | Downgrade to 2021 if needed |

================================================================================
  DAILY STANDUP CHECKLIST
================================================================================

  Every morning:
  1. cargo test (swarm) — must be 88+ passing
  2. anchor build (treasury) — must compile clean
  3. Check this file — what's today's block?
  4. After fixes: run tests again, update [ ] → [x]

================================================================================
  SKILL QUICK REFERENCE (replaces v2.2 Part 6)
================================================================================

  WEEK 2 (remediation):  No skills. Write Rust.
  WEEK 3 (wings+bridge): No skills. Write Rust.
  WEEK 4 Day 5 (CI):     github-workflow-automation → YAML templates only
  WEEK 5 Day 1-2 (demo):  walkthrough (builtin) → Mermaid diagrams
  WEEK 5 Day 3-4 (review): code-review (builtin) → formal diff review

  That's it. 3 skill loads across 5 weeks. Everything else is Rust.

================================================================================
END OF PLAN v3.0
================================================================================
