================================================================================
  RTP — RESILIENT TOKEN PROTOCOL
  FULL-SCOPE BUILD PLAN v2.2
  Incorporating: swarm-planning-3.md + cldcde collection + hackathon rules
  + frontier resources page integration links + token adoption model
================================================================================

HACKATHON: Solana Frontier (Colosseum × Canteen)
DEADLINE:  May 11, 2026 (5 weeks from Apr 6)
PRIZES:    $300k total — $30k Grand Champion
REGISTER:  Individual by May 4 — https://arena.colosseum.org/register
RULES:     https://colosseum.com/legal/Solana%20Frontier%20Hackathon%20Rules.pdf
RESOURCES:  https://colosseum.com/frontier/resources
COPILOT:    https://arena.colosseum.org/copilot (pressure-test against 5400+ past submissions)

================================================================================
  PART 1: WHAT WE'RE BUILDING (THE FULL SCOPE)
================================================================================

RTP is a Solana-native, self-funding treasury governed by a modular swarm.
Any token project adopts RTP — their trading fees route to the swarm,
which autonomously researches, validates, and executes yield strategies
(30K configs/night, 9-fold walk-forward validation, fee-aware simulation)
— returning yield back to the project and its holders.

┌─────────────────────────────────────────────────────────┐
│              fee flow (token adoption → PDA)            │
│  Token project enables TransferFeeConfig on mint         │
│  Every trade → fee auto-routes → Treasury PDA            │
│  Swarm researches, validates, executes → yield           │
│  Yield flows back to project + token holders             │
└──────────────────────┬──────────────────────────────────┘
                       │
          ┌────────────┼────────────┐
          │            │            │
     < threshold   = threshold   > maintenance
     REINVEST      REDISTRIBUTE  AUTO-INVEST
     (yield/swarm)  70/20/10     (RTP ecosystem)
                                  │
                          > $1M → HUMANITY FUND
                          (USDC grants to Solana
                           public-goods projects)

SIX WINGS (Rust, independently testable, via Coordinator):
├── Trading Wing     — yield gen + Hyperliquid/Jupiter execution
├── Security Wing    — threat detection + defense
├── Evolve Wing      — self-modification + adaptation
├── Knowledge Wing   — realtime knowledge graph
├── Audit Wing       — soulcontract enforcement + safety
└── Future-proof Wing — quantum + existential monitoring

THREE LAYERS:
├── On-Chain (Solana/Anchor)  — Treasury PDA + CPI splits + phase evolution
├── Swarm Runtime (Rust)       — Coordinator + wings + message bus
└── Research Layer (Python)     — Yield brain (SHIPPING, black-boxed)

PHASED EVOLUTION (irreversible, on-chain):
├── Phase 1: Sustenance (< $50k)       — self-hydrate, reinvest
├── Phase 2: Ecosystem ($50k–$1M)      — auto-LP top RTP tokens
└── Phase 3: Humanity Fund (>$1M)      — USDC grants, quadratic funding

================================================================================
  PART 2: TECH STACK (CONFIRMED)
================================================================================

CORE FRAMEWORKS:
├── atlas-gic (#1)           — https://github.com/chrisworsey55/atlas-gic
│                              multi-agent Darwinian loop → Evolve Wing
├── karpathy/autoresearch    — https://github.com/karpathy/autoresearch
│                              Modify/Verify/Keep loop spec
├── uditgoenka/autoresearch  — https://github.com/uditgoenka/autoresearch
│                              Claude-native implementation
├── MetaClaw                 — https://github.com/aiming-lab/MetaClaw
│                              Knowledge Wing + human override UI
├── revfactory/harness       — https://github.com/revfactory/harness
│                              Coordinator architecture reference
└── autoagent                — https://github.com/kevinrgu/autoagent
                              lifecycle/scaffolding boilerplate

SPONSORED HACKATHON RESOURCES:
├── Phantom Connect + CASH   — https://docs.phantom.app/phantom-connect/introduction
│                              agentic wallet + stablecoin treasury flows
│                              Get Started: https://phantom.app/phantom-connect
│                              React Template: https://github.com/phantom-labs/phantom-connect-react
│                              JS Template: https://github.com/phantom-labs/phantom-connect-js
│                              CASH: https://phantom.app/cash
│                              Phantom MCP Server: (agentic use cases)
├── Squads Multisig          — https://docs.squads.so
│                              Security Wing + PDA upgrade authority
│                              Get Started: https://squads.so
│                              Altitude (financial ops): https://altitude.finance
│                              Altitude on X: https://x.com/AltitudeFi
├── Swig                     — https://docs.swig.fi
│                              programmable smart wallets (wing message bus)
│                              Overview: https://docs.swig.fi/overview
│                              TypeScript SDK: https://docs.swig.fi/typescript-sdk
│                              TypeScript SDK Tutorial: https://docs.swig.fi/typescript-sdk/tutorial
│                              Rust SDK: https://docs.swig.fi/rust-sdk
│                              Developer Portal: https://portal.swig.fi
├── MoonPay Agents           — https://www.moonpay.com/developers/agents
│                              agent money movement infra
│                              npm: npm install -g @moonpay/cli
│                              Skills repo: https://github.com/moonpay/agents-skills
├── Solana MCP               — https://github.com/solana-developers/solana-mcp
│                              AI dev assistant for Anchor
│                              AI-powered documentation search + Anchor guidance
├── Arcium                   — https://docs.arcium.com
│                              encrypted computation (optional stretch)
│                              Developer Docs: https://docs.arcium.com
│                              Arcis Rust Framework: https://docs.arcium.com/arcis/getting-started
│                              Purple Paper: https://docs.arcium.com/resources/purple-paper
│                              RFP: https://docs.arcium.com/resources/request-for-products
└── Reflect                  — https://reflect.finance/docs
                              credibly-neutral stablecoin strategies (reference)
                              SDK: https://github.com/reflectmoney/stable.ts

NOT USING:
├── World Coin — toxic sentiment, skip entirely
├── Privy — not yet available (coming soon)
└── Coinbase — not yet available (coming soon)

EXISTING SHIPPING CODE (black-boxed):
  Proven in fractal-swarm (tradewife/fractal-swarm.git), now feeds RTP.
  The Python yield brain runs locally (gitignored) and ships as compiled binary.
├── night_shift.py           — 30K configs/night, 9-fold WFA, Darwinian
├── paper_trader.py          — live Binance, ADX filter
├── future_blind_simulator   — 0.1% fees, 10bps slippage, ground truth
├── evaluator_calibration    — fast/full sim calibration
├── discrepancy_detector     — divergence detection
└── SOL config (+118.3% PnL, 78% consistency, 429 trades)

================================================================================
  PART 2B: SOLANA DEVELOPMENT RESOURCES (from hackathon page)
================================================================================

START HERE:
├── Introduction to Solana Development
│   https://solana.com/developers/docs/intro
├── Important Concepts
│   https://solana.com/developers/docs/core-concepts
├── Setup Your Environment
│   https://solana.com/developers/docs/setup
└── Hello World
    https://solana.com/developers/docs/hello-world

DEV STARTER PACK:
├── Solana Playground (browser IDE)
│   https://play.solana.com
├── create-solana-dapp (scaffold in minutes)
│   https://github.com/solana-developers/create-solana-dapp
└── npx create-solana-dapp@latest

ANCHOR:
├── Intro to Anchor
│   https://www.anchor-lang.com/docs/introduction
├── Build a CRUD dApp
│   https://solana.com/developers/crud
├── Solana Program Examples (Anchor, Rust, Python)
│   https://github.com/solana-developers/program-examples
└── Solana MCP (AI assistant for Anchor)
    https://github.com/solana-developers/solana-mcp

GUIDES + COURSES:
├── Solana Cookbook
│   https://solanacookbook.com
├── Solana Bootcamp (7-hour crash course)
│   https://www.solana.com/developers/courses
├── Solana Bytes (byte-sized video playlist)
│   https://www.solana.com/developers/videos
└── FreeCodeCamp Interactive Solana Course
    https://www.freecodecamp.org/learn

AGENT TOOLING:
├── Solana Agent Skills (pre-built skills for AI agents)
│   https://github.com/solana-developers/solana-agent-skills
└── Solana MCP (AI dev assistant)
    https://github.comsolana-developers/solana-mcp

TOKEN + PAYMENT:
├── SPL Token Extensions (TransferFeeConfig for RTP token adoption)
│   https://solana.com/docs/tokens/extensions/transfer-fees
├── Metaplex (NFTs)
│   https://docs.metaplex.com
├── Solana Pay (payments)
│   https://solanapay.com
└── Solana Actions & Blinks (shareable tx interfaces)
    https://solana.com/docs/advanced/blinks

GOVERNANCE / DAOs:
├── Realms Docs (DAO tooling)
│   https://docs.realms.today
└── Quadratic Funding (Cubik — for Phase 3 humanity fund)
    https://solanacompass.com/projects/cubik

RPC:
├── Triton One (recommended, free private devnet/testnet)
│   https://triton.one
└── Devnet & Testnet always free

================================================================================
  PART 3: JUDGING CRITERIA → RTP STRENGTH MAPPING
================================================================================

| Criterion        | RTP Delivers                                           |
|------------------|--------------------------------------------------------|
| Functionality    | Live demo: adopt→fees→swarm→yield→redistribute on devnet |
| Potential Impact | Any Solana token can adopt — unruggable yield standard  |
| Novelty          | 6-wing swarm + soulcontract + token adoption model      |
| UX               | Phantom Connect + CASH wallet flows                   |
| Open-source      | Full swarm arch + treasury program (MIT)               |
| Business Plan    | Adoption fees → self-funding swarm → yield to holders  |

BLACK-BOX / OPEN-SOURCE SPLIT:
  ✅ OPEN: swarm architecture, treasury program, soulcontract, wing interfaces
  ❌ BOXED: Python yield brain (binary), encrypted configs, loss function

================================================================================
  PART 4: CLDCDE SKILL MAPPING (REVISED FULL SCOPE)
================================================================================

Every cldcde skill mapped to its specific RTP use case, build phase, and
priority. Only skills with direct applicability are included.

─────────────────────────────────────────────────────────────────────────────────
  TIER 1: CRITICAL (used across multiple wings, needed Week 1)
─────────────────────────────────────────────────────────────────────────────────

  SKILL                     WING(S)         USE IN RTP
  ─────────────────────────────────────────────────────────────────────────────
  swarm-orchestration       Coordinator     Design Coordinator message bus,
                                             wing routing, fault tolerance,
                                             dynamic topology. Model each wing
                                             as a swarm agent.

  hive-mind-advanced        Coordinator     Queen-worker consensus topology.
                                             Coordinator=queen, wings=workers.
                                             Maps to Audit Wing approval flow.

  spec-lock                 Audit + All      soulcontract = spec. Spec-Lock
                                             ensures implementation never
                                             drifts from governance without
                                             detection. Critical invariant.

  red-team-tribunal         Audit           3-agent adversarial review
                                             (Skeptic + User Proxy + Optimizer).
                                             IS the Audit Wing's review pattern.
                                             Every wing proposal must pass.

  compound-engineering      Audit           Meta-orchestration: coordinates
                                             Debt-Sentinel + Red Team +
                                             Spec-Lock into unified workflow.

  verification-quality      Audit           Truth scoring (0.95 threshold) +
                                             automatic rollback. Maps to
                                             Audit Wing's safety.rs.

─────────────────────────────────────────────────────────────────────────────────
  TIER 2: HIGH (core wing functionality, needed Weeks 1-2)
─────────────────────────────────────────────────────────────────────────────────

  SKILL                     WING(S)         USE IN RTP
  ─────────────────────────────────────────────────────────────────────────────
  agentdb-memory-patterns   Knowledge       Institutional memory for the
                                             swarm. Session memory for
                                             trades, long-term for strategy
                                             history, pattern learning.

  agentdb-advanced          Knowledge       Distributed multi-database
                                             coordination. Market data +
                                             strategy results + security
                                             events + architectural decisions.

  reasoningbank-agentdb     Knowledge       Adaptive learning from trading
                                             outcomes. Trajectory tracking,
                                             verdict judgment, memory
                                             distillation. Knowledge Wing
                                             compounds knowledge across
                                             iterations.

  agentdb-learning          Evolve          9 RL algorithms (Decision Transformer,
                                             Q-Learning, Actor-Critic).
                                             Evolve Wing's self-improvement.

  sparc-methodology         Evolve          THE methodology for soulcontract
                                             amendments + Evolve Wing proposals.
                                             Specify → Pseudocode → Architect
                                             → Refine → Complete. Every change
                                             follows SPARC.

  ultra-planner              Evolve          Strategic planning for Evolve Wing
                                             architecture proposals.

  debt-sentinel             Security        Anti-pattern detection with hooks.
                                             Model for Runtime Defense —
                                             detect anomalous tx patterns.

  swarm-advanced            Coordinator     Distributed workflow patterns.
                                             Wings operating concurrently:
                                             Trading executing + Security
                                             scanning + Knowledge ingesting.

  stream-chain              Coordinator    Sequential pipeline: proposal →
                                             audit → approve → execute as
                                             typed chain.

  fpef-analyzer             Security+Evolve  Find-Prove-Evidence-Fix for
                                             incident response + degradation
                                             root cause analysis.

─────────────────────────────────────────────────────────────────────────────────
  TIER 3: MEDIUM (supporting infrastructure, Weeks 2-3)
─────────────────────────────────────────────────────────────────────────────────

  SKILL                     WING(S)         USE IN RTP
  ─────────────────────────────────────────────────────────────────────────────
  hooks-automation          Coordinator     Model message routing as hooks:
                                             every message triggers pre-check
                                             (Audit) and post-check (logging).

  performance-analysis      Evolve          Wing performance benchmarking.
                                             Self-assessment pattern for
                                             identifying what to evolve.

  github-workflow-automation Infra           Extend night shift CI to include
                                             Rust tests, Anchor builds, swarm
                                             integration tests.

  github-release-management  Evolve          Release orchestration with rollback.
                                             Maps to Evolve Wing rollback pattern.

  ae-ltd-skill-builder      Dev             Build custom RTP-specific skills
                                             for Claude Code development.

  flow-nexus-swarm          Coordinator     Event-driven workflow automation
                                             for wing proposal processing.

  mcp-universal-manager     Dev             Auto-discover and monitor MCP
                                             servers (Phantom MCP, Solana MCP,
                                             MoonPay skills).

─────────────────────────────────────────────────────────────────────────────────
  TIER 4: LOW (nice-to-have, Week 4-5 polish)
─────────────────────────────────────────────────────────────────────────────────

  SKILL                     WING(S)         USE IN RTP
  ─────────────────────────────────────────────────────────────────────────────
  prologue                  Dev             Navigate all tools during dev.

  ae-proof-agent            Dev             Competitive analysis for hackathon
                                             positioning (vs other submissions).

  agentic-jujutsu           Trading         Version control for AI agents.
                                             Strategy lifecycle tracking.

  skill-builder             Dev             Build additional custom skills.

  multi-platform-architect  Dev             Cross-platform if extending beyond
                                             Claude Code.

─────────────────────────────────────────────────────────────────────────────────
  NOT RELEVANT (ignore for this project)
─────────────────────────────────────────────────────────────────────────────────

  banana, d3mo-generator, remotion-best-practices, puppeteer-stealth,
  avant-garde-frontend-architect, blender-3d-studio, youtube-creator,
  youtube-creator-pro, notebooklm-pro, obs-studio-control, n8n-workflow,
  context7-docs, create-worktrees, mutation-tester, sota-template-suite,
  viral-automation-suite, visual-regression, fartnode-orchestrator-suite,
  emergent-capability-suite, google-labs-extension, remote-visual-debugger,
  bd-management, agent-zero-brain, opencode, all flow-nexus-* except swarm,
  all ae-ltd-* except skill-builder, all github-* except workflow+release,
  pair-programming, hyperliquid-risk-monitor (deleted — fabricated metrics)

================================================================================
  PART 5: 5-WEEK BUILD PLAN (HACKATHON TIMELINE)
================================================================================

WEEK 1: FOUNDATION + TREASURY
─────────────────────────────────────────
  Day 1-2:
  [ ] Register individually at https://arena.colosseum.org/register (by May 4)
  [ ] Set up dev environment:
      - Anchor: https://www.anchor-lang.com/docs/introduction
      - Solana CLI: https://solana.com/docs/installation
      - Rust: https://www.rust-lang.org/tools/install
      - Solana Playground: https://play.solana.com
  [ ] Scaffold rtp/ directory structure per README
  [ ] CLDCDE: Use swarm-orchestration + hive-mind-advanced to design
      Coordinator architecture (message bus, soulguard, lifecycle)

  Day 3-4:
  [ ] Phantom Connect integration:
      - Integration Guide: https://docs.phantom.app/phantom-connect/introduction
      - React Template: https://github.com/phantom-labs/phantom-connect-react
      - CASH stablecoin: https://phantom.app/cash
  [ ] Squads Multisig → secure treasury PDA upgrade authority:
      - Get Started: https://docs.squads.so
      - Altitude (treasury ops): https://altitude.finance
  [ ] Anchor IDL stub: withdraw_fees (TransferFeeConfig withdraw),
      check_redistribute, hydrate_swarm
      - withdraw_fees uses token::withdraw_withheld_tokens_from_mint CPI
      - https://solana.com/docs/tokens/extensions/transfer-fees
      - Anchor examples: https://github.com/solana-developers/program-examples
      - Solana MCP for AI-assisted dev: https://github.com/solana-developers/solana-mcp
  [ ] CLDCDE: Use spec-lock to define soulcontract spec + enforcement

  Day 5:
  [ ] Treasury PDA on devnet — deposit → threshold → redistribute tx
      - Free devnet RPC: https://triton.one (private devnet always free)
  [ ] CLDCDE: Use red-team-tribunal to adversarial-review treasury program
  [ ] Weekly checkpoint: Treasury CPI works on devnet

  DELIVERABLES: Treasury program on devnet, multisig secured, basic
                redistribution demoable

WEEK 2: EVOLVE WING + COORDINATOR
─────────────────────────────────────────
  Day 1-2:
  [ ] Fork ATLAS (https://github.com/chrisworsey55/atlas-gic) → replace
      Sharpe loss fn with treasury-native metric:
      (USDC yield / SOL reserves) × (1 - max drawdown) × wing consistency
  [ ] karpathy/autoresearch (https://github.com/karpathy/autoresearch) →
      spec for evolve/proposer.rs + rollback.rs
  [ ] CLDCDE: Use sparc-methodology for Evolve Wing proposal format

  Day 3-4:
  [ ] uditgoenka/autoresearch (https://github.com/uditgoenka/autoresearch) →
      wire Claude-native loop into Evolve Wing
  [ ] revfactory/harness (https://github.com/revfactory/harness) →
      Coordinator router + soulguard reference
  [ ] Swig integration for wing message bus:
      - TypeScript SDK: https://docs.swig.fi/typescript-sdk
      - Rust SDK: https://docs.swig.fi/rust-sdk
      - Developer Portal: https://portal.swig.fi
  [ ] CLDCDE: Use compound-engineering to orchestrate
      Debt-Sentinel + Red Team + Spec-Lock

  Day 5:
  [ ] Coordinator routes typed messages between wing stubs
  [ ] soulcontract enforced on every message
  [ ] CLDCDE: Use verification-quality for truth-scoring + rollback
  [ ] Weekly checkpoint: Coordinator + Evolve Wing prototype working

  DELIVERABLES: Coordinator + Evolve Wing skeleton, ATLAS-adapted loop

WEEK 3: KNOWLEDGE WING + SECURITY WING
─────────────────────────────────────────
  Day 1-2:
  [ ] MetaClaw (https://github.com/aiming-lab/MetaClaw) → Knowledge Wing
      memory + human override UI
  [ ] agentdb-memory-patterns → session + long-term memory
  [ ] CLDCDE: Use agentdb-advanced for distributed knowledge store design

  Day 3-4:
  [ ] Security Wing stub: basic vulnerability scanning
  [ ] debt-sentinel pattern for runtime defense hooks
  [ ] CLDCDE: Use fpef-analyzer for incident response methodology

  Day 5:
  [ ] Cross-wing queries working: any wing asks Knowledge Wing
  [ ] autoagent (https://github.com/kevinrgu/autoagent) → lifecycle
      boilerplate (spawn, health-check, retire)
  [ ] CLDCDE: Use hooks-automation for pre/post message hooks
  [ ] Weekly checkpoint: All 6 wing stubs respond to Coordinator

  DELIVERABLES: All 6 wings stubbed, Knowledge Wing has memory, human
                override works via MetaClaw

WEEK 4: FULL LOOP + BLACK-BOXING
─────────────────────────────────────────
  Day 1-2:
  [ ] Black-box Python yield brain: pyinstaller → night_shift.bin
  [ ] Rust FFI bridge: Trading Wing calls Python binary via
      std::process::Command, receives typed JSON proposal
  [ ] Encrypted configs (AES, build-time key)
  [ ] CLDCDE: Use github-workflow-automation for CI with Rust + Anchor

  Day 3-4:
  [ ] Full loop demo: Python proposes → Audit approves → Rust executes
  [ ] Trading Wing executor: Hyperliquid + Jupiter integration
  [ ] Self-hydration CPI: 10% yield → sustenance PDA
  [ ] MoonPay Agents integration for agent money movement:
      - npm install -g @moonpay/cli
      - Skills: https://github.com/moonpay/agents-skills
  [ ] CLDCDE: Use performance-analysis for wing benchmarking

  Day 5:
  [ ] Ecosystem auto-invest: excess SOL → Jupiter CPI → LP top RTP tokens
  [ ] Phase evolution logic (Sustenance → Ecosystem → Humanity)
      - DAO tooling reference: https://docs.realms.today
      - Quadratic funding reference: https://solanacompass.com/projects/cubik
  [ ] Weekly checkpoint: Full end-to-end loop demoable

  DELIVERABLES: Complete loop, black-boxed strategies, full demo flow

WEEK 5: POLISH + HACKATHON SUBMISSION
─────────────────────────────────────────
  Day 1-2:
  [ ] Demo flow rehearsed (3 minutes):
      1. Token adopts RTP — TransferFeeConfig enabled
      2. Trading fees auto-route to Treasury PDA
      3. Swarm researches, validates, executes yield strategy
      4. Reserves hit threshold → live redistribution tx
      5. Verify: project + holders receive yield, SOL untouched
  [ ] CLDCDE: Use prologue for ecosystem navigation during final dev

  Day 3:
  [ ] third-party-disclosure.md (ATLAS MIT + karpathy + sponsored)
  [ ] README polished for submission
  [ ] Video recording of demo
  [ ] CLDCDE: Use ae-proof-agent for competitive positioning
  [ ] Colosseum Copilot final check:
      https://arena.colosseum.org/copilot

  Day 4-5:
  [ ] Submit to Colosseum
  [ ] Buffer for fixes
  [ ] CLDCDE: Use red-team-tribunal for final adversarial review of
      entire codebase before submission

  DELIVERABLES: Polished demo, submission package, disclosure doc

================================================================================
  PART 6: CLDCDE SKILLS PER BUILD DAY (QUICK REFERENCE)
================================================================================

WEEK 1
  Day 1-2: swarm-orchestration, hive-mind-advanced
  Day 3-4: spec-lock
  Day 5:   red-team-tribunal

WEEK 2
  Day 1-2: sparc-methodology
  Day 3-4: compound-engineering
  Day 5:   verification-quality

WEEK 3
  Day 1-2: agentdb-memory-patterns, agentdb-advanced
  Day 3-4: fpef-analyzer, debt-sentinel
  Day 5:   hooks-automation

WEEK 4
  Day 1-2: github-workflow-automation
  Day 3-4: performance-analysis
  Day 5:   stream-chain, flow-nexus-swarm

WEEK 5
  Day 1-2: prologue
  Day 3:   ae-proof-agent
  Day 4-5: red-team-tribunal (final sweep)

================================================================================
  PART 7: DISCLOSURE + BLACK-BOX STRUCTURE
================================================================================

OPEN-SOURCE (judges see, MIT):
  rtp/
  ├── swarm/                          # ATLAS-inspired architecture
  │   ├── coordinator/                # Message bus + soulguard
  │   ├── wings/skeleton/             # Wing interfaces + lifecycle
  │   └── lib.rs
  ├── programs/rtp-treasury/          # Anchor program (full source)
  ├── soulcontract.md                 # Governance invariants
  ├── docs/demo-flow.md
  └── third-party-disclosure.md

BLACK-BOXED (proprietary binary/configs):
  ├── wings/trading/brain/
  │   ├── night_shift.bin             # PyInstaller binary
  │   ├── configs/encrypted/          # AES-encrypted strategy params
  │   └── validation_results.bin      # 429-trade ground truth
  ├── evolve/loss_function.bin        # Treasury-native scoring
  └── research_pipeline/              # Full-sim + self-correction

THIRD-PARTY DISCLOSURE:
  atlas-gic (MIT)          — https://github.com/chrisworsey55/atlas-gic
                            Evolve Wing autoresearch loop
  karpathy/autoresearch    — https://github.com/karpathy/autoresearch
                            Modify/Verify/Keep specification
  uditgoenka/autoresearch  — https://github.com/uditgoenka/autoresearch
                            Claude-native implementation
  MetaClaw (MIT)           — https://github.com/aiming-lab/MetaClaw
                            Knowledge Wing + human override
  revfactory/harness (MIT) — https://github.com/revfactory/harness
                            Coordinator architecture reference
  autoagent (MIT)          — https://github.com/kevinrgu/autoagent
                            Lifecycle scaffolding
  Phantom Connect           — https://docs.phantom.app/phantom-connect/introduction
                            Agentic wallet + CASH stablecoin
  Squads Multisig           — https://docs.squads.so
                            Treasury security + PDA authority
  Swig                      — https://docs.swig.fi
                            Programmable smart wallets
  MoonPay Agents            — https://www.moonpay.com/developers/agents
                            Agent money movement

================================================================================
  PART 8: KEY INVARIANTS (enforced on-chain)
================================================================================

  1. PDA owns treasury (no private key risk)
  2. SPL TransferFeeConfig immutable from mint (no fee revocation)
     https://solana.com/docs/tokens/extensions/transfer-fees
  3. All transfers via CPI (atomic, verifiable)
  4. Agent proposes, human approves irreversible actions
  5. No SOL liquidation (USDC-only yield flows)
  6. Phase transitions irreversible (Sustenance → Ecosystem → Humanity)
  7. soulcontract amendments require human signature + 24h monitoring
  8. Auto-rollback if performance degrades > 5% post-amendment
  9. Self-hydration: ops only funded if sustenance bucket > 90-day runway
 10. Strategies remain black-boxed (competitive moat)

================================================================================
  PART 9: REVENUE MODEL
================================================================================

  INFLOW:
  ├── Token adoption: TransferFeeConfig fee from every trade on adopting tokens
  │   ├── pump.fun (most common): 0.05% creator fee (SOL) per PumpSwap trade
  │   │   └── $100k daily vol → $50/day → 0.25 SOL → ~$50/day
  │   └── Any Solana token: custom fee % set at mint → routes to Treasury PDA
  ├── Hyperliquid: USDC yield from perp strategies
  │   └── 20-50% annual on treasury
  └── Ecosystem LP: yield from auto-invested positions

  OUTFLOW:
  ├── < threshold: 100% reinvest (yield/swarm)
  ├── = threshold: 70% holders / 20% dev / 10% ecosystem
  ├── 10% yield carve-out → sustenance PDA (ops funding)
  │   └── At $10k treasury: ~$100-200/mo ops cost (trivial vs yield)
  └── Ecosystem excess → auto-invest top RTP tokens (Jupiter CPI)

  BREAK-EVEN: ~$5k reserves covers 90-day ops runway

================================================================================
  PART 10: QUICK LINK INDEX
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
  MetaClaw:      https://github.com/aiming-lab/MetaClaw
  revfactory:    https://github.com/revfactory/harness
  autoagent:     https://github.com/kevinrgu/autoagent

SPONSORED TOOLS:
  Phantom:       https://docs.phantom.app/phantom-connect/introduction
  CASH:          https://phantom.app/cash
  Squads:        https://docs.squads.so
  Altitude:      https://altitude.finance
  Swig:          https://docs.swig.fi
  Swig TS SDK:   https://docs.swig.fi/typescript-sdk
  Swig Rust SDK: https://docs.swig.fi/rust-sdk
  Swig Portal:   https://portal.swig.fi
  MoonPay:       https://www.moonpay.com/developers/agents
  MoonPay CLI:   npm install -g @moonpay/cli
  Arcium:        https://docs.arcium.com
  Arcis (Rust):  https://docs.arcium.com/arcis/getting-started
  Solana MCP:    https://github.com/solana-developers/solana-mcp

SOLANA DOCS:
  Core:          https://solana.com/developers
  Setup:         https://solana.com/developers/docs/setup
  Anchor:        https://www.anchor-lang.com/docs/introduction
  Playground:    https://play.solana.com
  Cookbook:      https://solanacookbook.com
  Bootcamp:      https://www.solana.com/developers/courses
  Transfer Fees: https://solana.com/docs/tokens/extensions/transfer-fees
  Blinks:        https://solana.com/docs/advanced/blinks
  Program Ex:    https://github.com/solana-developers/program-examples
  Agent Skills:  https://github.com/solana-developers/solana-agent-skills
  CRUD dApp:      https://solana.com/developers/crud
  create-dapp:   https://github.com/solana-developers/create-solana-dapp
  Solana Pay:    https://solanapay.com
  Metaplex:      https://docs.metaplex.com
  Realms (DAOs): https://docs.realms.today
  Cubik (QF):    https://solanacompass.com/projects/cubik

RPC:
  Triton One:    https://triton.one (free private devnet/testnet)

================================================================================
END OF PLAN v2.1
================================================================================
