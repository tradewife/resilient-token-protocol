================================================================================
  RTP — RESILIENT TOKEN PROTOCOL
  FULL-SCOPE BUILD PLAN v3.0
  "Post-governance, commitment-enforced token longevity layer"
================================================================================

HACKATHON: Solana Frontier (Colosseum × Canteen)
DEADLINE:  May 11, 2026 (5 weeks from Apr 6)
PRIZES:    $300k total — $30k Grand Champion
REGISTER:  Individual by May 4 — https://arena.colosseum.org/register
RULES:     https://colosseum.com/legal/Solana%20Frontier%20Hackathon%20Rules.pdf
RESOURCES:  https://colosseum.com/frontier/resources
COPILOT:    https://arena.colosseum.org/copilot

================================================================================
  PART 1: THE PRODUCT
================================================================================

RTP is an unruggable launch standard for Solana tokens.

THE PROBLEM:
  "Don't rug" is a social promise. Social promises are cheap.
  Every day, tokens rug on Solana. Not because tech fails — because
  trust is not a security guarantee.

THE SOLUTION:
  Make "don't rug" a structural property of the token, enforced by code.
  Any token project adopts RTP via TransferFeeConfig — fees auto-route
  to a PDA-owned treasury that is constrained, circuit-breaker-protected,
  and fully verifiable. The rug vector is eliminated at the mint level.

THE PRICE EQUATION:
  Non-RTP token price = SOL macro + founder risk + rug risk + narrative
  RTP token price     = SOL macro + narrative

THREE GENERATIONS OF TRUST:
  Gen 1: Trust people     → rugs
  Gen 2: Trust voting     → DAO capture, inefficiency
  Gen 3: Trust commitments → RTP (rigidity is acceptable tradeoff)

CORE PRIMITIVES (all on-chain, all enforced):
  1. Fee Routing        — TransferFeeConfig → Treasury PDA (immutable)
  2. Price Floor        — TWAP oracle + circuit breaker → autonomous buyback
  3. Correlated Hedging — SOL-short via Drift (structurally reliable)
  4. Circuit Breakers   — cooldown + epoch cap + velocity limit
  5. Yield Deployment   — idle capital → Kamino/Marginfi
  6. Verification       — every action on-chain, provable, auditable
  7. Redistribution     — above threshold: 70% holders / 20% dev / 10% ecosystem

THREE FLYWHEELS:
  Fee Revenue → Hedge Yield → Yield/Arbitrage → compounds → buyback pressure
       ▲                                                        │
       └────────────────────────────────────────────────────────┘

  Key insight: RTP tokens eliminate founder/rug noise → higher SOL correlation
  → correlated hedges are more reliable → self-reinforcing property.

AGENT ROLES:
  Allocator  — reads inflows, routes funds per immutable rules
  Executor   — swaps (Jupiter), hedging (Drift), yield (Kamino/Marginfi)
  Verifier   — publishes proof of every action on-chain

================================================================================
  PART 2: TECH STACK
================================================================================

ON-CHAIN:
  Solana, Anchor (Rust), Token-2022 (TransferFeeConfig), Pyth Network (TWAP)

DEFI INTEGRATIONS:
  Jupiter Aggregator — swap execution for buybacks + yield routing
  Drift Protocol     — perpetual futures for correlated SOL hedging
  Kamino Finance     — yield deployment for idle treasury capital
  Marginfi           — yield deployment for idle treasury capital

MULTISIG:
  Squads Protocol v4 — treasury PDA upgrade authority

AGENT SWARM:
  Rust, typed message bus, soulcontract enforcement, WASM sandbox

RESEARCH (Python, gitignored):
  night_shift.py — 30K configs/night, 9-fold WFA, Darwinian
  paper_trader.py — live Binance validation
  future_blind_simulator.py — 0.1% fees, 10bps slippage, ground truth

CORE FRAMEWORKS:
  atlas-gic (#1)           — https://github.com/chrisworsey55/atlas-gic
                              Darwinian loop for strategy evolution
  karpathy/autoresearch    — https://github.com/karpathy/autoresearch
                              Modify/Verify/Keep specification
  uditgoenka/autoresearch  — https://github.com/uditgoenka/autoresearch
                              Claude-native implementation

SPONSORED HACKATHON RESOURCES:
  Phantom Connect + CASH   — https://docs.phantom.app/phantom-connect/introduction
  Squads Multisig          — https://docs.squads.so
  MoonPay Agents           — https://www.moonpay.com/developers/agents
  Solana MCP               — https://github.com/solana-developers/solana-mcp
  Arcium                   — https://docs.arcium.com (stretch)

NOT USING: World Coin (toxic sentiment)

================================================================================
  PART 3: JUDGING CRITERIA → RTP STRENGTH MAPPING
================================================================================

| Criterion        | RTP Delivers                                           |
|------------------|--------------------------------------------------------|
| Functionality    | Live: token adopts RTP → fee → buyback → verify on devnet|
| Potential Impact | Every Solana token can adopt — unruggable standard       |
| Novelty          | Post-governance commitment enforcement, correlated hedge |
| UX               | 3-min demo: adopt → trade → defend → verify             |
| Open-source      | Full swarm + treasury program (MIT)                     |
| Business Plan    | Fee routing → self-funding → ecosystem flywheel          |

BLACK-BOX / OPEN-SOURCE SPLIT:
  ✅ OPEN: swarm architecture, treasury program, soulcontract, agent interfaces
  ❌ BOXED: Python yield brain (binary), encrypted configs, loss function

================================================================================
  PART 4: CLDCDE SKILL MAPPING
================================================================================

Only skills with direct applicability to the token longevity product:

TIER 1: CRITICAL (needed Week 1)
  swarm-orchestration       — Coordinator message bus design
  hive-mind-advanced        — Consensus topology for agent coordination
  spec-lock                 — soulcontract = spec. Implementation never drifts.
  red-team-tribunal         — 3-agent adversarial review for Verifier agent
  compound-engineering      — Meta-orchestration of audit pipeline
  verification-quality      — Truth scoring + automatic rollback (Verifier core)

TIER 2: HIGH (core agent functionality, Weeks 1-2)
  agentdb-memory-patterns   — Institutional memory for trades + decisions
  agentdb-advanced          — Distributed knowledge coordination
  reasoningbank-agentdb     — Adaptive learning from outcomes
  agentdb-learning          — RL algorithms for strategy improvement
  sparc-methodology         — soulcontract amendment format
  ultra-planner              — Strategic planning for architecture proposals
  debt-sentinel             — Anti-pattern detection → circuit breaker model
  swarm-advanced            — Distributed workflow patterns for agents
  stream-chain              — Sequential pipeline: proposal → audit → execute
  fpef-analyzer             — Find-Prove-Evidence-Fix for incident response

TIER 3: MEDIUM (supporting infrastructure, Weeks 2-3)
  hooks-automation          — Message routing as hooks (pre/post check)
  performance-analysis      — Agent benchmarking + self-assessment
  github-workflow-automation — CI with Rust tests + Anchor builds
  github-release-management  — Release with rollback capability
  ae-ltd-skill-builder      — Build custom RTP skills for Claude Code
  flow-nexus-swarm          — Event-driven wing proposal processing
  mcp-universal-manager     — Monitor MCP servers (Phantom, Solana, MoonPay)

TIER 4: LOW (polish, Weeks 4-5)
  prologue                  — Ecosystem navigation during final dev
  ae-proof-agent            — Competitive analysis for hackathon positioning
  agentic-jujutsu           — Strategy lifecycle version control
  skill-builder             — Build additional custom skills
  multi-platform-architect  — Cross-platform if extending beyond Claude Code

================================================================================
  PART 5: 5-WEEK BUILD PLAN
================================================================================

WEEK 1: TOKEN LONGEVITY CORE + TREASURY
───────────────────────────────────────
  Day 1-2:
  [ ] Register at https://arena.colosseum.org/register (by May 4)
  [ ] Set up dev environment:
      - Anchor: https://www.anchor-lang.com/docs/introduction
      - Solana CLI: https://solana.com/docs/installation
      - Rust: https://www.rust-lang.org/tools/install
  [ ] Scaffold rtp/ directory structure per README
  [ ] CLDCDE: Use swarm-orchestration + hive-mind-advanced to design
      Coordinator architecture (message bus, soulguard, lifecycle)

  Day 3-4:
  [ ] Treasury program on devnet:
      - deposit_usdc (receive fees from TransferFeeConfig)
      - check_floor (Pyth TWAP oracle → price floor calculation)
      - execute_buyback (Jupiter CPI when price < floor × discount)
      - Circuit breaker stub (cooldown + epoch cap)
      - Anchor IDL: https://github.com/solana-developers/program-examples
  [ ] Phantom Connect integration:
      - https://docs.phantom.app/phantom-connect/introduction
      - CASH stablecoin: https://phantom.app/cash
  [ ] Squads Multisig → PDA upgrade authority:
      - https://docs.squads.so
  [ ] CLDCDE: Use spec-lock to define soulcontract + enforcement

  Day 5:
  [ ] Fee routing demo: mock token → TransferFeeConfig → Treasury PDA
  [ ] Price floor check + buyback trigger on devnet
  [ ] CLDCDE: Use red-team-tribunal to adversarial-review treasury program
  [ ] Weekly checkpoint: Treasury receives fees, defends floor, buyback works

  DELIVERABLES: Treasury program on devnet, fee routing, floor defense demoable

WEEK 2: AGENTS + HEDGING + VERIFIER
────────────────────────────────────
  Day 1-2:
  [ ] Rust agent swarm skeleton:
      - Coordinator (message bus + soulguard)
      - Allocator (inflow routing per rules)
      - Executor (Jupiter swap CPI)
      - Verifier (on-chain proof publication)
  [ ] Fork ATLAS (https://github.com/chrisworsey55/atlas-gic) → adapt
      loss function for treasury-native metric
  [ ] CLDCDE: Use sparc-methodology for agent proposal format

  Day 3-4:
  [ ] Drift Protocol integration — correlated SOL-short hedge:
      - Drift SDK: https://drift.trade
      - Hedge when: treasury drawdown > threshold
      - Unwind when: drawdown recovers + profit target hit
  [ ] Pyth Network TWAP oracle integration:
      - https://pyth.network
      - Feed price to floor check CPI
  [ ] CLDCDE: Use compound-engineering to orchestrate audit pipeline

  Day 5:
  [ ] Full loop: fee → floor check → buyback OR hedge → verify
  [ ] Verifier publishes proof of every action on-chain
  [ ] CLDCDE: Use verification-quality for truth-scoring + rollback
  [ ] Weekly checkpoint: Agents operate treasury autonomously on devnet

  DELIVERABLES: Agent swarm skeleton, hedging integrated, Verifier operational

WEEK 3: YIELD + CIRCUIT BREAKERS + KNOWLEDGE
────────────────────────────────────────────
  Day 1-2:
  [ ] Kamino/Marginfi yield integration:
      - Idle treasury capital → yield protocol
      - Yield compounds reserves → increases buyback capacity
  [ ] Circuit breaker full implementation:
      - Cooldown (min time between ops)
      - Epoch cap (max USDC per epoch)
      - Velocity limit (max depletion rate)
  [ ] CLDCDE: Use debt-sentinel for circuit breaker anti-pattern model

  Day 3-4:
  [ ] Regime detection (yield brain informs hedge weights)
  [ ] Knowledge Wing: institutional memory for trades + decisions
  [ ] CLDCDE: Use agentdb-memory-patterns + agentdb-advanced

  Day 5:
  [ ] Three flywheels operational: fees → hedges → yield → compounds
  [ ] Redistribution above threshold (70/20/10 split)
  [ ] CLDCDE: Use fpef-analyzer for incident response methodology
  [ ] Weekly checkpoint: All three flywheels demoable

  DELIVERABLES: Yield deployment, circuit breakers, flywheels operational

WEEK 4: FULL LOOP + BLACK-BOXING + PHASE EVOLUTION
───────────────────────────────────────────────────
  Day 1-2:
  [ ] Black-box Python yield brain: pyinstaller → night_shift.bin
  [ ] Rust FFI bridge: Executor calls Python binary, receives typed JSON
  [ ] Encrypted configs (AES, build-time key)
  [ ] CLDCDE: Use github-workflow-automation for CI with Rust + Anchor

  Day 3-4:
  [ ] Phase evolution logic on-chain (Sustenance → Ecosystem → Humanity)
  [ ] Ecosystem auto-invest: excess → Jupiter CPI → LP top RTP tokens
  [ ] Self-hydration: yield → sustenance PDA → fund swarm ops
  [ ] MoonPay Agents: https://www.moonpay.com/developers/agents

  Day 5:
  [ ] End-to-end demo: adopt → fee → floor → hedge → yield → redistribute
  [ ] CLDCDE: Use performance-analysis for agent benchmarking
  [ ] Weekly checkpoint: Full product demoable

  DELIVERABLES: Complete loop, black-boxed strategies, full demo flow

WEEK 5: POLISH + HACKATHON SUBMISSION
──────────────────────────────────────
  Day 1-2:
  [ ] Demo flow rehearsed (3 minutes per docs/demo-flow.md)
  [ ] Stress test: circuit breakers, rapid withdrawal, oracle manipulation
  [ ] CLDCDE: Use prologue for ecosystem navigation

  Day 3:
  [ ] third-party-disclosure.md finalized
  [ ] README polished for submission
  [ ] Video recording of demo
  [ ] CLDCDE: Use ae-proof-agent for competitive positioning
  [ ] Colosseum Copilot: https://arena.colosseum.org/copilot

  Day 4-5:
  [ ] Submit to Colosseum
  [ ] Buffer for fixes
  [ ] CLDCDE: Use red-team-tribunal for final adversarial review

  DELIVERABLES: Polished demo, submission package, disclosure doc

================================================================================
  PART 6: KEY INVARIANTS (enforced on-chain)
================================================================================

  1. PDA owns treasury (no private key risk)
  2. TransferFeeConfig immutable from mint (no fee revocation)
  3. All transfers via CPI (atomic, verifiable)
  4. Circuit breakers: cooldown + epoch cap + velocity limit (no drain)
  5. Price floor enforced by TWAP oracle (not a cron, a trigger)
  6. Every agent action verified on-chain (Verifier publishes proof)
  7. Agent proposes, human approves irreversible actions
  8. No SOL liquidation (USDC-only flows)
  9. Phase transitions irreversible (Sustenance → Ecosystem → Humanity)
 10. soulcontract amendments require human signature + 24h monitoring
 11. Auto-rollback if performance degrades > 5% post-amendment
 12. Yield brain strategies remain black-boxed (competitive moat)

================================================================================
  PART 7: REVENUE MODEL
================================================================================

  INFLOW:
  ├── TransferFeeConfig: per-trade fee from every RTP-adopting token
  │   └── pump.fun: 0.05% creator fee (SOL) per PumpSwap trade
  ├── Hyperliquid: USDC yield from perp strategies (execution venue)
  └── Ecosystem LP: yield from auto-invested positions

  OUTFLOW:
  ├── < threshold: 100% reinvest (floor defense + yield + hedge)
  ├── = threshold: 70% holders / 20% dev / 10% ecosystem
  ├── 10% yield carve-out → sustenance PDA (ops funding)
  └── Ecosystem excess → auto-invest top RTP tokens (Jupiter CPI)

  BREAK-EVEN: ~$5k reserves covers 90-day ops runway

================================================================================
  PART 8: DISCLOSURE + BLACK-BOX STRUCTURE
================================================================================

OPEN-SOURCE (judges see, MIT):
  rtp/
  ├── swarm/                          # Agent swarm (Allocator, Executor, Verifier)
  │   ├── coordinator/                # Message bus + soulguard
  │   ├── agents/                     # Agent implementations
  │   ├── skills/                     # Atomic skill definitions
  │   └── lib.rs
  ├── programs/rtp-treasury/          # Anchor program (full source)
  ├── soulcontract.md                 # Governance invariants
  ├── docs/demo-flow.md
  └── third-party-disclosure.md

BLACK-BOXED (proprietary binary/configs, not in repo):
  ├── scripts/                        # Python yield brain (gitignored locally)
  │   ├── night_shift.py
  │   ├── paper_trader.py
  │   └── future_blind_simulator.py
  ├── data/                           # OHLCV, results, state (gitignored)
  └── (ships as): night_shift.bin, configs/encrypted/

THIRD-PARTY DISCLOSURE:
  atlas-gic (MIT)          — https://github.com/chrisworsey55/atlas-gic
  karpathy/autoresearch    — https://github.com/karpathy/autoresearch
  uditgoenka/autoresearch  — https://github.com/uditgoenka/autoresearch
  Phantom Connect           — https://docs.phantom.app/phantom-connect/introduction
  Squads Multisig           — https://docs.squads.so
  MoonPay Agents            — https://www.moonpay.com/developers/agents
  Drift Protocol            — https://drift.trade
  Pyth Network              — https://pyth.network
  Jupiter Aggregator        — https://jupiter.aggregate
  Kamino Finance            — https://kamino.finance
  Marginfi                  — https://marginfi.com

================================================================================
  PART 9: QUICK LINK INDEX
================================================================================

HACKATHON:
  Register:     https://arena.colosseum.org/register
  Rules:        https://colosseum.com/legal/Solana%20Frontier%20Hackathon%20Rules.pdf
  Resources:    https://colosseum.com/frontier/resources
  Copilot:      https://arena.colosseum.org/copilot

FRAMEWORKS:
  ATLAS:         https://github.com/chrisworsey55/atlas-gic
  karpathy:       https://github.com/karpathy/autoresearch
  uditgoenka:    https://github.com/uditgoenka/autoresearch

SPONSORED TOOLS:
  Phantom:       https://docs.phantom.app/phantom-connect/introduction
  CASH:          https://phantom.app/cash
  Squads:        https://docs.squads.so
  MoonPay:       https://www.moonpay.com/developers/agents
  Solana MCP:    https://github.com/solana-developers/solana-mcp
  Arcium:        https://docs.arcium.com

SOLANA DOCS:
  Core:          https://solana.com/developers
  Setup:         https://solana.com/developers/docs/setup
  Anchor:        https://www.anchor-lang.com/docs/introduction
  Transfer Fees: https://solana.com/docs/tokens/extensions/transfer-fees
  Program Ex:    https://github.com/solana-developers/program-examples

DEFI INTEGRATIONS:
  Jupiter:       https://jupiter.aggregate
  Drift:         https://drift.trade
  Kamino:        https://kamino.finance
  Marginfi:      https://marginfi.com
  Pyth:          https://pyth.network

RPC:
  Triton One:    https://triton.one (free private devnet/testnet)

RESEARCH:
  rtp-skills-research: https://github.com/tradewife/rtp-skills-research

================================================================================
  PART 10: CLDCDE SKILLS PER BUILD DAY
================================================================================

WEEK 1:  Day 1-2: swarm-orchestration, hive-mind-advanced
         Day 3-4: spec-lock
         Day 5:   red-team-tribunal

WEEK 2:  Day 1-2: sparc-methodology
         Day 3-4: compound-engineering
         Day 5:   verification-quality

WEEK 3:  Day 1-2: debt-sentinel
         Day 3-4: agentdb-memory-patterns, agentdb-advanced
         Day 5:   fpef-analyzer

WEEK 4:  Day 1-2: github-workflow-automation
         Day 3-4: performance-analysis
         Day 5:   stream-chain, flow-nexus-swarm

WEEK 5:  Day 1-2: prologue
         Day 3:   ae-proof-agent
         Day 4-5: red-team-tribunal (final sweep)

================================================================================
END OF BUILD PLAN v3.0
================================================================================
