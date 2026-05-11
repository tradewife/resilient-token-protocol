# RTP Colosseum Video Specs v4 — Final Submission Direction

**Project:** Resilient Token Protocol  
**Pitch language:** **Supercharging Creator Fees**  
**Builder:** @trade_wife on X · @tradewife on GitHub · Solo builder, Sydney  
**Hackathon:** Solana Frontier / Colosseum submission  
**Outputs:** pitch video + technical overview video, each **under 3:00**  
**Production stack:** `/home/kt/hermes-apollo/skills/demo-video-stack`  
**Required engines:** HyperFrames + browser-harness + Impeccable design gate  

This spec replaces the earlier “just okay” cuts. The videos must feel like a serious solo founder built a real company-grade protocol, not a hackathon slideshow. They must work with or without voiceover and with or without a facecam square.

External constraints checked:

- Colosseum expects video submissions under 3 minutes and judges projects like startup pitches, not just engineering exercises.
- Colosseum’s own hackathon guidance emphasizes team background, product description, founder motivation, market opportunity, usage/traction, and a working demo.
- The live Colosseum Frontier page frames the competition as a startup proving ground for Solana founders, with submissions due during the active hackathon window.

---

## 0. Quality-Control Verdict

The previous specs were directionally right but still left too much room for a generic hackathon video. RTP deserves sharper treatment.

What the final Colosseum Copilot + VC audit confirmed:

1. **The category is the win condition.**  
   The project should not be framed as another AI agent or trading bot. It is a **fee-routing treasury protocol** for token projects. The swarm, research engine, and Flash Trade CPI are the proof system underneath that wedge.

2. **The closest overlaps validate the wedge, not erase it.**  
   Copilot surfaced adjacent projects:
   - `fungible` (Cypherpunk, Sep 2025): AMM launchpad where burning tokens grants perpetual trading fee rewards.
   - `zarnith` (Breakout, Apr 2025): modular fee-routing and revenue-splitting protocol.
   - `firebird` (Radar, Sep 2024): decentralized treasury management for Solana projects and DAOs.
   - `project-plutus` / `agent-arc` (Breakout, Apr 2025 winners): AI agent deployment and non-custodial AI trading.
   These are useful comparables, but none is the exact RTP wedge: **creator fees -> per-mint Treasury PDA -> autonomous CPI execution -> holder/project/ecosystem redistribution**. The videos must make that distinction obvious.

3. **Accelerator-quality projects reward clear wedges.**  
   Copilot’s accelerator-filtered results point to Kormos, Reflect, Hylo, Decal, Rakurai, Watt Protocol, and others: winners explain a narrow primitive clearly, then prove execution. RTP should pitch as infrastructure with a concrete first buyer, not as a sprawling “AI swarm.”

4. **Archive framing supports infrastructure timing.**  
   Copilot archive results emphasized that crypto products need primetime-ready infrastructure, token launches need PMF discipline, and Solana treasury infrastructure is becoming institutionally legible. RTP should frame itself as the missing operating layer between token fee revenue and program-enforced treasury yield.

5. **The live site already contains the product story.**  
   The website exposes the correct proof path: hero, vitals, live yield engine, mainnet proof cards, trust invariants, research pipeline, SDK CTA, docs, research, launch. The technical video must not replace this with invented slides. It must use the site deeply.

6. **The dashboard has dynamic loading states.**  
   Live browser capture must wait for hydrated client state. If the research page initially shows “Loading latest results...” or the trader shows “Connecting...”, the production model must wait, refresh, or capture from a local dev server with the same data files. It must not ship loading placeholders unless the scene explicitly uses them to say “live system connecting.”

7. **Some live site design choices conflict with Impeccable bans.**  
   The dashboard/docs currently include side-stripe visual treatments in the web app. Do not copy those into generated video graphics. Browser capture may show them because they are part of the live product, but all HyperFrames/overlay graphics must obey this spec’s stricter no-slop standard.

8. **The spec must force verification, not hope for craft.**  
   Every rendered scene needs a screenshot review. Every browser recording needs a post-capture frame inspection. Every slide needs a readability check at 1920x1080 and at 50% playback size. No tiny text. No faint labels. No unexplained blank space.

### Justice Test

The final videos have done the project justice only if a muted judge can say all of this after 45 seconds:

```text
RTP turns token creator fees into program-enforced yield treasuries.
It has a live website, real mainnet transactions, and an SDK adoption path.
The founder shipped serious infrastructure, not a wrapper.
```

If the judge instead says “AI trading bot” or “agent swarm demo,” the video failed.

### Judge-Ready One-Liners

The production model must keep these three lines in its working context while editing:

```text
3-5 words: Supercharging Creator Fees.
One sentence: Token projects route creator fees into RTP, and RTP turns them into program-enforced yield treasuries.
Investor wedge: launch platforms integrate once; every launched token can ship with a self-funding treasury.
```

The longer architecture exists to prove those lines. It must never compete with them.

---

## 1. The Strategic Frame

### The Judge’s Question

Every scene answers one of these:

1. **What is it?**  
   Creator fee yield infrastructure for Solana token projects.
2. **Why now?**  
   Solana launch platforms and token projects already generate creator fees, but those fees mostly sit idle.
3. **Why this team?**  
   A solo builder shipped a codebase beyond the engineering depth of most hackathon projects: Anchor program, Rust swarm, Python research engine, live trader, SDK, docs, dashboard, Railway services, mainnet proofs.
4. **Why should I believe it works?**  
   Live website, Solana Explorer, 325 unit + 5 integration tests, real mainnet Flash Trade proofs, running Railway services.
5. **Why does it matter?**  
   It turns “don’t rug” from a social promise into program-enforced, autonomous treasury infrastructure.

### Winning Through Line

**Creator fees are wasted capital. RTP turns them into self-funding treasuries.**

The project has evolved from “agentic trading” into a much sharper product:

> A launch platform integrates RTP once. Every token it launches gets a per-mint treasury PDA. Fees flow in. The swarm researches and executes validated yield strategies. Yield flows back 70/20/10. No RTP token. No human trading key. No shared pool.

This must be the spine of both videos. The technical depth is not the pitch; it is the proof behind the pitch.

### VC / Judge Pressure Test

The pitch should preempt the questions a serious judge will ask:

```text
Who is the first buyer?
Launch platforms and token creators that want better post-launch economics.

What exactly do they adopt?
An SDK / integration path that creates a per-mint Treasury PDA and routes fees into RTP.

Why is this not just another AI trading bot?
The product is fee-routing treasury infrastructure. Agents are the execution layer.

What is already proven?
Live website, docs, launch flow, SDK path, 325 unit + 5 integration tests, and mainnet Flash Trade CPI proofs.

What is not yet proven?
Real adopter capital at scale. The honest next milestone is first launch-platform or token-project integration.
```

Treat these as invisible guardrails. The videos do not need a Q&A slide, but no scene may create confusion that would make these answers harder.

### Honest Trade-Off Framing

Judges respect precise claims. The videos must avoid hype that a technical reviewer can puncture.

- Flash Trade is the selected execution venue because it enables CPI composability today. Do not imply it has Drift/Jupiter-scale liquidity.
- Backtest/research metrics validate the research engine and stress process. Do not present them as guaranteed live yield.
- Mainnet proof means CPI plumbing and PDA signing are real on Solana mainnet. It does not mean real adopter treasuries are already running at production scale.
- Squads, Arcium, MoonPay, Raydium, and other future integrations are roadmap unless the repo/site proves them live.
- RTP has no token. Do not let any visual imply an RTP token launch.

### Competitive Positioning

Use this mental map when writing captions or narration:

```text
Not a launchpad: RTP plugs into launch platforms.
Not a vault: RTP creates per-token treasuries from creator fees.
Not a copy-trading product: RTP executes program-bounded treasury strategy.
Not an AI-agent wrapper: RTP is Solana infrastructure with autonomous execution.
Not a generic treasury dashboard: RTP routes fees, signs CPI, and redistributes.
```

### Solo Builder Positioning

Do not apologize for solo. Make it part of the security and execution thesis:

- No coordination drag.
- No dead weight.
- Fewer humans in sensitive operational loops.
- Recent DeFi exploits often route through social engineering and compromised human process. RTP is built toward zero human-in-the-loop execution, bounded by on-chain invariants.
- A solo builder shipping this much is the founder-market fit signal.

Use this once in the pitch and once in the technical video, but do not overdo it.

---

## 2. Shared Video System

### Visual Register

**Botanical research station meets institutional surveillance.**  
The protocol is a living autonomous organism viewed through precise instruments.

The website already has the right direction: deep emerald void, coral accents, restrained interface density, flower image as organic intelligence. The videos should amplify that language without adding generic crypto effects.

### Typography

User requirement: **Geist**.

Use:

```css
--font-display: "Geist", "Geist Sans", system-ui, sans-serif;
--font-body: "Geist", "Geist Sans", system-ui, sans-serif;
--font-mono: "Geist Mono", "SF Mono", monospace;
```

Note: the live dashboard currently imports Bricolage Grotesque and Figtree. Do not fight the live site during browser capture. For HyperFrames title cards, overlays, labels, diagrams, captions, lower thirds, and code flows, use Geist only.

### Color Tokens

Match the website’s emerald/coral system:

```css
--void:          oklch(8% 0.025 160);
--surface-0:     oklch(11% 0.02 160);
--surface-1:     oklch(14% 0.018 160);
--surface-2:     oklch(18% 0.015 160);
--border:        oklch(25% 0.015 160);
--text-primary:  #e6f0e8;
--text-secondary:oklch(72% 0.03 160);
--text-tertiary: oklch(55% 0.025 160);
--text-muted:    oklch(42% 0.02 160);
--emerald:       oklch(55% 0.1 160);
--emerald-dim:   oklch(35% 0.06 160);
--coral:         oklch(75% 0.12 30);
--coral-dim:     oklch(40% 0.06 30);
```

### Composition Rules

- 1920x1080, 30fps, H.264, CRF 16, `yuv420p`, `faststart`.
- Minimum type size in rendered video:
  - Hero headlines: 64px+
  - Section titles: 38px+
  - Body/captions: 24px+
  - Mono/code: 22px+
  - Lower thirds: 20px+
- Never use faint labels below 60% opacity if they carry meaning.
- No important text within the bottom-right 360x260 safe area, so a square facecam can be overlaid.
- Use a persistent but quiet proof rail only where useful. It must not steal focus.
- All browser footage must feel human: deliberate cursor movement, dwell time, natural scroll rhythm.
- Every HyperFrames scene needs a clear hero frame before animation. Animate into layout; do not animate layout into existence.

### Banned

- Generic purple/cyan crypto palette.
- Gradient text.
- Side-stripe callouts in generated video graphics.
- Glassmorphism.
- Particle fields, random noise, bokeh/orbs.
- Hero metric grid as a substitute for narrative.
- Tiny, faint captions.
- Inconsistent placement: scenes must use a stable 160px left anchor unless explicitly centered for final identity.
- Empty “void” shots longer than 1.5s unless a major statement is landing.

### Programmatic Motion Language

Use motion to prove the architecture:

- **Capital flow:** fees move as SOL pulses through nodes.
- **PDA signing:** no key icon. Use a locked PDA node that emits a signature pulse only when `invoke_signed` appears.
- **Isolation:** multiple token mints split into separate treasury lanes, never pooling.
- **Research:** 30,000 candidate dots collapse into one promoted strategy through WFA gates.
- **Governance:** proposed action hits `soulguard`, passes or rejects against invariants.

Keep animations deterministic. Use seeded PRNG if needed; no `Math.random()`.

---

## 2A. Demo-Video-Stack Compliance

The production model must use the stack as an actual production pipeline, not as decoration.

### HyperFrames — Primary Engine

HyperFrames is the source of truth for generated scenes, diagrams, captions, proof rails, overlays, code-flow animations, and final compositing.

Mandatory:

- Read `media/colosseum/design.md` before writing any composition.
- Run HyperFrames prompt expansion mentally before each composition: intent, audience, rhythm, layout, motion, transition, verification.
- Build each scene’s **hero frame first** as static HTML/CSS. No GSAP until the frame is visually correct.
- Every composition must use valid `data-*` timing attributes.
- Every timed element must have `class="clip"`, `data-start`, `data-duration`, and `data-track-index`.
- Timelines must be paused and registered on `window.__timelines`.
- Use deterministic `fromTo()` where scene seeking could otherwise break `from()` animations.
- No iframes for captured website content. Use browser recordings or screenshots as assets.
- Use screenshots as layered panels only when needed; do not fake the live site.
- Run `npm run check` or the equivalent `npx hyperframes lint && npx hyperframes validate && npx hyperframes inspect` before render approval.

Motion sophistication:

- Each beat must declare a **world**, not only a layout.
- Each significant element needs a motion verb: draws, locks, counts, compresses, rejects, signs, splits, resolves.
- No scene may use the same `y: 30, opacity: 0` entrance for every element.
- No more than two independent tweens in a scene may use the same ease.
- Every scene needs build / breathe / resolve.
- Use hard cuts for proof moments and velocity-matched transitions for explanatory scenes. Do not crossfade everything by default.

### browser-harness — Live Product Capture

The website and Explorer recordings must be driven by real browser interaction.

Mandatory:

- First navigation uses `new_tab(url)`.
- Capture screenshot after every meaningful action and verify the state before continuing.
- Use coordinate clicks from screenshots for visible UI. Avoid brittle selector-only clicking.
- Stop on auth walls. Do not fake a wallet or private Railway dashboard.
- Use real Explorer pages for mainnet proof.
- Wait for hydration on dynamic pages. If the page is still loading after a reasonable wait, record a note and retry against local dev server with the same public data, but never hardcode contradictory overlays.

For browser footage, a good recording has:

- cursor movement that feels intentional,
- no speed-run scrolling,
- no accidental layout jumps,
- no visible “Loading...” states unless intentionally discussed,
- no cut before the proof is readable.

### Impeccable — Design Gate

Impeccable is not optional. The render must pass these gates:

- Anti-slop: no generic AI crypto look.
- Typography: all generated text readable at 50% scale.
- Layout: stable anchors, no arbitrary centering, no inconsistent indentation.
- Color: emerald/coral system only, no purple/cyan default.
- Contrast: every meaningful text element passes visual contrast.
- Composition: every frame has at least two focal points and three layers unless intentionally raw proof footage.

### architecture-diagram — Animated Systems, Not Tiny Boxes

Use architecture-diagram principles for technical scenes, but adapt the palette to RTP instead of using the skill’s default cyan/violet/slate palette.

Mandatory diagram content:

- Fee-routing adoption lane: token project / launchpad -> per-mint Treasury PDA.
- Research lane: Night Shift -> strategy promotion -> StrategyRecord Live.
- Execution lane: Trading Wing -> `open_flash_position` -> `invoke_signed` -> Flash Trade.
- Enforcement lane: soulguard + Anchor constraints.
- Redistribution lane: close -> SOL returns -> 70/20/10.

Diagram rules:

- Arrows behind nodes.
- Labels 24px+ in rendered video.
- Minimum 40px visual gap between lanes.
- No dense node map that requires pausing to understand.
- Animate build-up by stage: fee lane first, enforcement gate second, execution third, redistribution last.

### ASCII Video — Optional, Restrained, Earned

ASCII is allowed only as a restrained texture or end-crescendo accent after proof has been shown.

Rules:

- Never use ASCII behind small text.
- Never use dense ASCII as a substitute for architecture diagrams.
- If used, text legibility is primary: high contrast, large characters, shader stack max 2.
- Best use: 3-5 seconds in the final close as “the organism runs forever,” not as the opener.

### Remotion — Fallback for Data-Driven Assembly

HyperFrames remains primary. Remotion may be used only if a data-driven React composition is materially easier for:

- a reusable proof-card sequence,
- code-driven chart layouts,
- final programmatic stitching of many assets.

If Remotion is used, keep components small and inspectable. Preview before full render. Do not create a parallel design system.

---

## 3. Video 1 — Pitch Video

**Target duration:** 2:35-2:50  
**Maximum:** 3:00  
**Purpose:** Colosseum investor-style pitch.  
**Core feeling:** “This is a real founder with a category wedge and real execution.”  
**Structure:** Hook → Product → Live proof → Why this wins → Market → Founder → Close.

### Pitch Scorecard

The pitch video must satisfy Colosseum’s startup judging criteria explicitly:

| Criterion | Video Evidence |
|---|---|
| Founder + market fit | Solo builder from Sydney, prior ZKPUTER/zktrader hackathon placement, clear belief in agentic tokenomics and DeFi as better infrastructure |
| Insight | Creator fees already exist but idle; the missing layer is automated, program-enforced fee yield |
| Product + execution | Live dashboard, mainnet Explorer proofs, SDK call, docs, launch flow, 325+5 tests |
| Market size | Token projects and launch platforms, not generic DeFi users; wedge is fee-routing treasury infrastructure |
| Communication | “Supercharging Creator Fees” repeated and visually anchored |
| Viability | No RTP token, SDK path for launch platforms, future commercial path around integration, monitoring, keepers, and protocol usage |

### What The Pitch Must Not Do

- Do not lead with “AI swarm.” That is crowded and lowers the project into a saturated category.
- Do not spend 20 seconds listing all six wings. Show why the wings matter through proof.
- Do not overclaim production adoption. Say the adoption path is ready; show devnet launch flow and SDK.
- Do not hide domain-transfer risk. If the strategy metrics appear, frame them as research-engine validation and stress testing, not a guarantee of future yield.

### Pitch Video Beat Map

| Beat | Time | Engine | Purpose |
|---|---:|---|---|
| 1 | 0:00-0:12 | HyperFrames | Hook: wasted creator fees |
| 2 | 0:12-0:32 | HyperFrames | Product: Supercharging Creator Fees |
| 3 | 0:32-1:05 | browser-harness | Live website + mainnet proof |
| 4 | 1:05-1:32 | HyperFrames | Why this is defensible |
| 5 | 1:32-1:52 | HyperFrames | Market wedge |
| 6 | 1:52-2:20 | HyperFrames + brief repo flashes | Founder-market fit |
| 7 | 2:20-2:48 | HyperFrames | Final close + adoption |

### Scene 1 — Cold Hook: Creator Fees Are Sleeping

**Time:** 0:00-0:12  
**Engine:** HyperFrames kinetic type  
**Frame:** left-aligned at 160px, no cards, emerald void.

On-screen copy:

```text
Solana token projects already earn creator fees.

Most of that capital never becomes a treasury.

RTP puts it to work.
```

Motion:

- Line 1 fades up from 8px.
- Line 2 lands after a 0.7s pause, coral on “sits there”.
- Line 3 lands larger, emerald underline draw.

No logo yet. No founder. The first 10 seconds are the problem. Do not open with strategy PnL, agents, flowers, or architecture. The judge must understand the wasted-fee problem before seeing the machine.

### Scene 2 — Product: Supercharging Creator Fees

**Time:** 0:12-0:32  
**Engine:** HyperFrames architecture animation  
**Frame:** same left anchor.

Hero copy:

```text
SUPERCHARGING CREATOR FEES
Resilient Token Protocol
```

Animated capital flow:

```text
Creator Fees (SOL)
  -> per-mint Treasury PDA
  -> Flash Trade CPI
  -> SOL yield
  -> 70/20/10 redistribution
```

Support lines:

```text
No RTP token.
One SDK call to adopt.
Self-funding treasury infrastructure.
```

Optional microline, only if there is room at 24px+:

```text
Launch platforms integrate once. Every token can ship with a treasury.
```

Visual requirements:

- Use a flowing line, not card boxes.
- Each node can sit on a small surface, but the scene must not become a row of identical cards.
- The Treasury PDA node should lock, then pulse when the execution arrow fires.
- “per-mint” and “No RTP token” should be visible without narration.

### Scene 3 — Live Proof: Website + Explorer

**Time:** 0:32-1:05  
**Engine:** browser-harness capture with minimal HyperFrames lower-third  
**Purpose:** show it is live, not a concept.

Browser sequence:

1. Navigate to `https://www.resilientprotocol.xyz/`.
2. Dwell on hero:
   - “Every token gets a program-enforced treasury”
   - “No RTP token. Pure infrastructure.”
3. Scroll to vitals strip:
   - Cumulative PnL
   - Treasury SOL
   - Mainnet TXs = 4
   - Test coverage = 325+5
   - Calmar = 44.89
4. Scroll to “The yield engine is running. With real capital.”
5. Show current position / flat-watching state. Either is acceptable:
   - If position open: highlight `SOL/USDT · LONG · 9x`.
   - If flat: highlight the strategy rule: score > 0.25 with 3+ bullish timeframes.
6. Scroll to “Real mainnet transactions, not testnet.”
7. Click the **Open · CPI invoke_signed** transaction.
8. On Solana Explorer, show Success and program logs.
9. Inject only one callout:

```text
Treasury PDA signs here.
No human trading key exists.
```

Lower third:

```text
LIVE PRODUCT · resilientprotocol.xyz
```

Quality requirements:

- No overlaid giant arrows on the dashboard. Let the website carry itself.
- If Explorer logs are visually dense, zoom/crop a stabilized region.
- The callout must use Geist Mono at 24px+.
- If the trader API is still connecting, hold on the mainnet proof cards and vitals instead of letting “Connecting...” become the emotional center of the scene.

### Scene 4 — Why This Wins: The Depth Behind the Product

**Time:** 1:05-1:32  
**Engine:** HyperFrames, programmatic architecture montage  
**Purpose:** answer “what makes this more than a trading bot?”

This scene uses five distinct visual treatments, not identical boxes.

1. **Visible refusal**

```text
Bad actions are rejected before execution.
```

Visual: a proposed trade crosses `soulguard`, fails a coral `StrategyNotLive` / `BelowThreshold` / cap gate, and disappears. This is the moment constitutional governance becomes visible. Do not bury the security story inside a document scan.

2. **Constitutional governance**

```text
16 constitutional invariants
enforced in Rust and on-chain
```

Visual: `SOULCONTRACT.md` line scans into Anchor `require!` constraints.

3. **Per-token isolation**

```text
Every mint gets its own Treasury PDA.
No shared pool. No honeypot.
```

Visual: three token lanes split into three isolated PDAs. Loss/risk in lane B cannot cross into A or C.

4. **Research engine**

```text
30,000 configs/night
9-fold walk-forward validation
Darwinian evolution
```

Visual: candidate field collapses through gates into `SOL/USDT Survivor 2.69`. Label this as **research validation**, not promised live returns.

5. **On-chain execution**

```text
Flash Trade CPI
invoke_signed
no human keypair
```

Visual: `bridge.rs -> chain_client.rs -> lib.rs -> Flash Trade` as code pulse.

Bottom proof line:

```text
325 unit tests · 5 integration tests · 7 Railway services · mainnet proofs
```

This scene must feel like the invisible engineering layer becoming visible. It cannot be a list of claims. Use programmatic animation to make each constraint act on the system:

- a bad action hits `soulguard` and is rejected,
- a token treasury lane remains isolated when another lane flashes coral,
- the candidate field compresses into Survivor 2.69,
- `invoke_signed` activates only inside the Treasury PDA lane.

### Scene 5 — Market: The Wedge

**Time:** 1:32-1:52  
**Engine:** HyperFrames typographic narrative  
**Purpose:** communicate startup potential without a metric-grid cliché.

On-screen copy:

```text
Every cycle creates more Solana tokens and launch venues.

Launch platforms compete on distribution.
Communities compete on trust.

RTP gives both a new primitive:
fees that earn yield by default.
```

Final line:

```text
The layer between “fees exist” and “fees earn yield.”
```

Optional lower proof line:

```text
Adjacent products exist. The fee-routing treasury primitive is the wedge.
```

Visual:

- Do not use a “10,000+ / $50B / 0 competitors” grid.
- Instead, animate many small fee streams converging into separated PDA lanes.
- Coral only on “yield by default.”
- If showing comparables, show them as adjacent lanes outside the RTP path:
  - launchpad rewards,
  - fee splitting,
  - treasury dashboards,
  - AI trading terminals.
  Then resolve to RTP’s distinct lane: **creator fees -> Treasury PDA -> CPI yield -> redistribution**.

### Scene 6 — Founder: Solo Builder, Sydney

**Time:** 1:52-2:20  
**Engine:** HyperFrames with brief, tasteful repository/code flashes  
**Purpose:** founder-market fit and credibility.

On-screen structure:

```text
@trade_wife
Solo builder · Sydney
```

Then stagger:

```text
Built ZKPUTER, now zktrader
4th place · Zypherpunk Hackathon · first hackathon

Since then:
ZKVM tool-call experiments
OxAuteur cinematography intelligence
Senpi-Waifu Rust fork for HL trading

Arts degree, ten years ago.
Self-taught through YouTube, GitHub, Crypto Twitter.
```

Solo-builder punchline:

```text
RTP is designed for fewer humans in the loop.
In DeFi, every unnecessary human is another attack surface.
```

Visual:

- No avatar placeholder.
- No generic “team slide.”
- Use repo file map flashes: `rtp/`, `sdk/`, `dashboard/`, `research/`, `cli/`.
- Show “solo builder” as discipline, not loneliness.

### Scene 7 — Close: Adoption

**Time:** 2:20-2:48  
**Engine:** HyperFrames closing crescendo  
**Purpose:** make the product obvious and memorable.

Three statements, one at a time:

```text
Launch platforms integrate once.
Every token gets a treasury.
Creator fees start working.
```

Code:

```ts
const result = await registerWithRTP(connection, wallet, {
  authority: publicKey,
});
```

Final identity:

```text
RESILIENT TOKEN PROTOCOL
Supercharging Creator Fees

resilientprotocol.xyz
github.com/tradewife/resilient-token-protocol
@trade_wife
```

End on a 0.5s fade to void.

Do not end as a founder biography. End as a product adoption path.

---

## 4. Video 2 — Technical Overview

**Target duration:** 2:45-2:58  
**Maximum:** 3:00  
**Purpose:** convince technical judges the system is real, deep, and reviewable.  
**Core feeling:** “A human engineer is walking me through a working system.”  
**Directive:** the website is the demo. Use the live product heavily.

### Technical Thesis

The technical overview must prove that RTP is an end-to-end system, not a mocked frontend:

```text
Research produces strategies.
Rust coordinates and validates intent.
Anchor enforces constraints.
Treasury PDA signs CPI.
Flash Trade executes on Solana.
Yield returns and redistributes.
```

Every technical beat must map to one of those verbs. Avoid generic “architecture overview” language unless the viewer can see the corresponding file, website section, Explorer proof, or animated data path.

### Technical Proof Inventory

The technical overview must visibly include:

- `2bLg1Fu...` CPI open transaction on mainnet.
- `dFqkoP2...` CPI close transaction on mainnet.
- The dashboard’s mainnet proof cards.
- The docs sidebar with Architecture and Security sections.
- The launch page platform selector.
- The research page or dashboard research pipeline.
- A code-flow animation naming `bridge.rs`, `chain_client.rs`, `open_flash_position`, `invoke_signed`, and Flash Trade.
- The safety gates: frozen check, StrategyRecord Live, 20% position cap, max 3 open positions, remaining-account validation, 70/20/10 redistribution.
- A visible refusal path: an invalid or non-live strategy must be shown hitting `soulguard` or Anchor constraints and failing before it reaches CPI.
- A precise capital-scaling statement: mainnet CPI is proven with controlled micro positions; adopter treasury capital at scale is the next milestone, not a current claim.

### Technical Video Beat Map

| Beat | Time | Engine | Purpose |
|---|---:|---|---|
| 1 | 0:00-0:12 | browser-harness / Explorer | Lead with hardest proof |
| 2 | 0:12-0:27 | HyperFrames | Explain 3-layer architecture |
| 3 | 0:27-1:25 | browser-harness | Full dashboard walkthrough |
| 4 | 1:25-2:12 | browser-harness | Docs + research + launch pages |
| 5 | 2:12-2:42 | HyperFrames | Code-level CPI + invariant flow |
| 6 | 2:42-2:58 | HyperFrames | Technical close |

### Scene 1 — Mainnet Proof First

**Time:** 0:00-0:12  
**Engine:** browser-harness on Solana Explorer  
**Purpose:** prove reality before explaining anything.

Open:

```text
https://explorer.solana.com/tx/2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3
```

Show:

- Success.
- Flash Trade program invocation.
- Program logs / compute units if visible.

Overlay:

```text
SOLANA MAINNET
Flash Trade CPI · Treasury PDA invoke_signed
```

Secondary microcopy, if logs are readable:

```text
Mainnet plumbing proven. Capital scaling remains deliberately capped.
```

This is intentionally raw. No title card before it.

### Scene 2 — Architecture in 15 Seconds

**Time:** 0:12-0:27  
**Engine:** HyperFrames architecture diagram  
**Purpose:** orient the viewer before the website tour.

Animated stack:

```text
Python Research
30K configs/night · 9-fold WFA · Monte Carlo · strategy plugins

Rust Swarm Runtime
Coordinator · Trading · Security · Evolve · Knowledge · Audit · Futureproof

Solana Anchor Program
Treasury PDA · Flash Trade CPI · 70/20/10 · emergency freeze
```

Flow line:

```text
research validates -> swarm executes -> program enforces -> Explorer verifies
```

Visual requirements:

- Use three horizontal layers with enough contrast.
- Include animated arrows.
- No tiny labels. Each layer title 36px+, details 24px+.

### Scene 3 — Dashboard Walkthrough

**Time:** 0:27-1:25  
**Engine:** browser-harness  
**Purpose:** show the live product like a human would.

Sequence:

1. Open `https://www.resilientprotocol.xyz/`.
2. Hero:
   - “Every token gets a program-enforced treasury”
   - launch + docs buttons
   - flower image as product identity
3. Vitals:
   - Cumulative PnL
   - Treasury SOL
   - Mainnet TXs
   - Test coverage 325+5
   - Calmar 44.89
4. Mainnet proof section:
   - current position or flat-watching state
   - PnL chart
   - 4 proof cards
   - devnet redistribution link
   - source on GitHub link
5. Trust architecture:
   - PDA ownership
   - per-token isolation
   - emergency freeze
   - strategy lifecycle
   - CPI-only execution
   - irreversible phase evolution
6. Research engine:
   - 30,000 hypotheses
   - 9 folds
   - Darwinian evolution
   - overfitting detection
   - full-sim validation
   - validated card: Calmar 44.89, +554%, 12.3% DD, 100% consistency, 0 liquidations, 16,228 candidates
   - add or hold on a label that frames this as research validation, not a yield promise
7. Active strategy + swarm architecture:
   - Survivor 2.69 params
   - 6 wings
8. Integration CTA:
   - SDK code snippet
   - “One function call”

Interaction rhythm:

- Scroll 260-420px per motion.
- Dwell 1.0-1.5s on each proof region.
- Use cursor hover on proof cards and nav links.
- No overlays except a compact but readable top-right chapter label:

```text
TECH OVERVIEW · LIVE DASHBOARD
```

### Scene 4 — Docs, Research, Launch

**Time:** 1:25-2:12  
**Engine:** browser-harness  
**Purpose:** show this is documented, usable infrastructure.

#### Research page

Navigate to `/research`:

- Show “Night Shift Research.”
- Market state.
- Production baseline.
- Top candidate card.
- Strategy config JSON.
- Full report preview.

Chapter label:

```text
RESEARCH · NIGHT SHIFT OUTPUT
```

#### Docs page

Navigate to `/docs`:

- Sidebar visible.
- What is RTP?
- Why this is different.
- Architecture diagram.
- Squads vs Yield Aggregator vs RTP table.
- What’s been built table.
- Click or scroll to:
  - Getting Started — Platforms
  - SDK Reference
  - Treasury PDA
  - Fee Routing
  - Swarm Execution
  - Security Model

Chapter label:

```text
DOCS · INTEGRATION SURFACE
```

#### Launch page

Navigate to `/launch`:

- Platform selector: Pump.fun, Bags.fm, Raydium.
- Solana wallet connect (any wallet).
- Token form.
- Devnet treasury creation language.
- If wallet is not connected, do not fake a connected state. Show that the flow exists and is ready.

Chapter label:

```text
LAUNCH · TOKEN + RTP TREASURY
```

### Scene 5 — Code-Level CPI and Governance Flow

**Time:** 2:12-2:42  
**Engine:** HyperFrames + optional static code excerpts  
**Purpose:** show what the website cannot fully explain.

This is the most important technical animation.

Use real code excerpts as visual anchors, not wall-of-code screenshots. The animation should quote only short function/file identifiers and small snippets, then explain the flow with motion.

#### Flow A — Strategy Promotion

```text
Night Shift
  -> survivor config
  -> bridge.rs ExecutePermit
  -> Trading Wing
  -> soulguard.rs
  -> StrategyRecord must be Live
```

Visual:

- Candidate config crosses gates.
- Bad action is rejected by a red/coral `require!` gate.
- Live strategy receives an emerald permit.
- Show `StrategyNotLive` briefly as the rejected state.
- Show the actual security message as a product feature, not an error-screen embarrassment:

```text
System refused execution.
Only live, bounded strategies reach CPI.
```

#### Flow B — Execution

```text
chain_client.rs builds open_flash_position
Anchor validates:
  not frozen
  strategy Live
  runway floor
  max 20% position size
  max 3 open positions
  Flash Trade accounts valid

Treasury PDA signs via invoke_signed
Flash Trade opens position on Solana
```

Visual:

- Show a stylized code rail:

```text
ExecutePermit -> build_open_ix() -> open_flash_position() -> invoke_signed() -> Flash Trade
```

- Animate the PDA signature as program-derived, not key-derived.
- Show `fee-payer wallet = gas only` as a small side label.
- Show “remaining accounts validated” as a separate gate so the Flash Trade integration reads as secure, not just connected.
- If there is room, label Flash Trade honestly:

```text
Chosen for CPI composability.
Venue abstraction can expand later.
```

#### Flow C — Redistribution

```text
close_flash_position -> SOL returns -> check_redistribute
70% holders · 20% project dev · 10% ecosystem
```

Visual:

- SOL returns to the same treasury PDA, then splits.
- Keep the split large and readable.

### Scene 6 — Technical Close

**Time:** 2:42-2:58  
**Engine:** HyperFrames  
**Purpose:** summarize the proof.

On-screen:

```text
What judges can verify:

mainnet Flash Trade transactions
live dashboard
open-source repo
325 unit + 5 integration tests
SDK integration path
Solana-enforced treasury constraints
```

Final:

```text
RESILIENT TOKEN PROTOCOL
Supercharging Creator Fees
```

---

## 5. Browser-Harness Recording Specs

Use browser-harness exactly because the requested stack depends on it.

### Global Browser Settings

- Viewport: 1920x1080.
- Device scale factor: 1.
- Hide bookmarks bar.
- Use a clean Chrome profile.
- Network wait after navigation.
- Record both video and screenshots for hero frames.
- If an external auth wall appears, stop; do not fake authenticated views.

### Human Interaction Timing

```text
Initial page dwell: 1.2-1.8s
Scroll duration: 300-600ms
Scroll distance: 260-420px
Post-scroll dwell: 0.8-1.4s
Nav click dwell: 1.2s after load
Hover dwell: 0.4-0.7s
Explorer dwell: 2.0s on proof
```

### Pitch Browser Shot List

```text
p01-home-hero.png
p02-home-vitals.png
p03-live-engine.png
p04-mainnet-proof-cards.png
p05-explorer-success.png
p06-explorer-invoke-signed-callout.png
p07-sdk-snippet.png
```

### Technical Browser Shot List

```text
t01-explorer-success.png
t02-home-hero.png
t03-vitals.png
t04-live-engine.png
t05-proof-cards.png
t06-trust-invariants.png
t07-research-pipeline.png
t08-validated-strategy.png
t09-wings.png
t10-sdk-cta.png
t11-research-page.png
t12-research-config.png
t13-docs-overview.png
t14-docs-built-table.png
t15-docs-sdk-reference.png
t16-docs-treasury-pda.png
t17-docs-fee-routing.png
t18-docs-security.png
t19-launch-platforms.png
t20-launch-form.png
```

### Explorer Callout Injection

Only inject callouts on Solana Explorer, never across the RTP dashboard.

```js
const callout = document.createElement("div");
callout.textContent = "Treasury PDA signs via invoke_signed. No human trading key.";
callout.style.cssText = `
  position: fixed;
  left: 48px;
  bottom: 48px;
  z-index: 2147483647;
  font: 24px "Geist Mono", monospace;
  color: #e6f0e8;
  background: oklch(8% 0.025 160 / 0.94);
  border: 1px solid oklch(35% 0.06 160);
  border-radius: 6px;
  padding: 14px 18px;
`;
document.body.appendChild(callout);
```

---

## 6. HyperFrames Production Specs

### Required Compositions

```text
compositions/shared/design.css
compositions/pitch/01-hook.html
compositions/pitch/02-product-flow.html
compositions/pitch/04-depth.html
compositions/pitch/05-market.html
compositions/pitch/06-founder.html
compositions/pitch/07-close.html

compositions/tech/02-architecture.html
compositions/tech/05-cpi-flow.html
compositions/tech/06-close.html
```

### Required Components

```text
CapitalFlow
PerMintIsolation
ResearchFunnel
InvariantGate
PdaInvokeSigned
RedistributionSplit
FounderTimeline
ProofRail
```

### Motion Defaults

```js
const EASE_OUT = "power4.out";
const EASE_EXPO = "expo.out";
const ENTER = { opacity: 0, y: 8, scale: 0.985, duration: 0.5, ease: EASE_OUT };
```

Rules:

- All timelines `paused: true`.
- Register timelines on `window.__timelines`.
- No infinite loops.
- Crossfade between every segment.
- Every segment has a 6-frame pre-roll and post-roll for clean concat.

---

## 7. Overlay and Facecam Safety

Assume the user may overlay a small square talking-head video.

Reserved facecam-safe regions:

- Preferred: bottom-right 320x240 with 32px margin.
- Alternate: top-right 320x240.

Do not put essential text or proof badges in those regions. The proof rail, if used, should sit bottom-left or top-left.

All videos must still make sense if muted:

- Every scene has readable chapter labels.
- Every proof has visible labels.
- Technical claims appear as text, not only narration.
- Founder story appears as text.

---

## 8. Audio Guidance

The user will handle audio, but the video should be easy to narrate.

### Pitch Narration Skeleton

```text
Creator fees are one of the biggest unused assets in Solana tokens.
Resilient Token Protocol supercharges creator fees.
Any token project gets a program-enforced treasury. Fees flow in, RTP generates yield, yield flows back to holders.
Here is the live system.
The hard part is not the dashboard. The hard part is the trust model: per-token treasuries, PDA signing, on-chain constraints, and autonomous research.
I built this solo from Sydney because I believe DeFi should become the better system I thought crypto was when I first arrived.
RTP is pure infrastructure. No token. One function call. Supercharging Creator Fees.
```

### Technical Narration Skeleton

```text
Start with the proof: this is a real Solana mainnet transaction.
The architecture has three layers: Python researches, Rust coordinates and executes, Solana enforces.
The website is the demo: live trader state, proof transactions, strategy validation, invariants, SDK integration, docs, and launch flow.
The important security property is that the Treasury PDA signs through invoke_signed. No private key exists for trading.
The fee-payer only pays gas. Strategy actions are bounded by soulguard and the Anchor program.
Yield returns to the treasury and redistributes 70/20/10 on-chain.
```

Music:

- Sparse ambient electronic.
- No dramatic trailer hits.
- Pitch can be warmer and more energetic.
- Technical should be lower and steadier.
- Keep music at 8-12% under narration.

---

## 9. Quality Gate / Impeccable Audit

### Visual Audit

Before final encode, verify:

- Text never falls below readable size.
- No faint gray-on-dark critical labels.
- No slide has more than 65 characters per line unless it is code.
- No generic crypto palette.
- No unearned empty space.
- No inconsistent alignment.
- No randomly centered scenes except the final close.
- No dashboard overlays that obscure product evidence.
- No “architecture diagram as tiny boxes” problem.
- No long static website scroll without a visible proof target.

### Technical Audit

Verify:

- Browser captures are from the live site, not static mockups.
- Solana Explorer transaction links are valid and visible.
- Mainnet vs devnet labels are accurate.
- If trader is flat during capture, copy says “watching” not “open position.”
- If live dashboard data changes, do not hardcode contradictory overlay numbers.
- “325 unit + 5 integration tests” appears as test coverage, not “330 unit tests.”
- “No RTP token” appears in both videos.
- “Per-token isolation” appears in both videos.
- Technical video shows `/docs`, `/research`, and `/launch`, not only `/`.

### Anti-Slop Verdict

The final render should pass this test:

> Could a judge watch the first 30 seconds muted and understand that this is a real, high-engineering Solana infrastructure product?

If not, recut.

### VC Pitcher Audit

Before rendering, the implementer must score the video plan against this battlecard:

```text
Category clarity:
Does the video say fee-routing treasury protocol before it says AI agent?

Buyer clarity:
Can a judge name the buyer as launch platforms / token creators?

Proof clarity:
Can a judge name one live proof without narration?

Risk honesty:
Does the video avoid overstating Flash Trade liquidity, live capital scale, or guaranteed strategy returns?

Founder-market fit:
Does solo builder read as velocity + lower human attack surface, not as lack of team?

Adoption path:
Does the close tell the judge what happens next?
```

If any answer is no, revise the script and frames before rendering.

---

## 9A. Mandatory Vision Self-Review

The production model MUST self-review with vision. This is not optional.

### Required Review Artifacts

For each final candidate video, extract frames:

```bash
mkdir -p media/colosseum/final/qc/pitch media/colosseum/final/qc/tech

ffmpeg -i media/colosseum/final/rtp-pitch-supercharging-creator-fees.mp4 \
  -vf "fps=1/3" media/colosseum/final/qc/pitch/frame-%03d.png

ffmpeg -i media/colosseum/final/rtp-technical-overview.mp4 \
  -vf "fps=1/3" media/colosseum/final/qc/tech/frame-%03d.png
```

Also extract exact first-10-second frames at 1fps:

```bash
ffmpeg -ss 0 -t 10 -i media/colosseum/final/rtp-pitch-supercharging-creator-fees.mp4 \
  -vf "fps=1" media/colosseum/final/qc/pitch/first10-%02d.png

ffmpeg -ss 0 -t 10 -i media/colosseum/final/rtp-technical-overview.mp4 \
  -vf "fps=1" media/colosseum/final/qc/tech/first10-%02d.png
```

The coding/rendering model must inspect these frames visually and write a QC note before finalizing:

```text
media/colosseum/final/qc/QC-NOTES.md
```

### Vision Review Checklist

For every extracted frame, answer:

- Is the primary message readable in under 2 seconds?
- Is any text too small, faint, clipped, or under the facecam-safe area?
- Is there dead blank space that should contain architectural motion, proof context, or a stronger crop?
- Is placement stable from beat to beat?
- Does the scene look like RTP’s emerald/coral research-station identity, not generic crypto?
- Does the frame prove something, explain something, or emotionally land something?
- If it is browser footage, is the proof area actually visible?
- If it is an architecture slide, can a technical judge understand the data flow without narration?

### Browser Recording Review

For every browser recording:

- Scrub the raw capture before compositing.
- Reject recordings with accidental overscroll, page-loading placeholders, cursor jitter, hidden proof text, or unreadable Explorer logs.
- Confirm `/docs`, `/research`, `/launch`, and `/` all loaded in their intended hydrated state.
- Confirm Solana Explorer proof is visible, not just the URL bar.
- Confirm any injected callout does not cover the status or logs it is explaining.

### Slide / HyperFrames Review

For every generated scene:

- Run `npx hyperframes inspect`.
- Run `npx hyperframes validate`.
- Check the scene at 1920x1080 and at 960x540. If it fails at half size, text is too small.
- Verify timeline seeking: start, midpoint, and last frame must all show the correct visual state.
- Confirm no scene relies on audio to be understood.

### Required QC Decision

At the end of `QC-NOTES.md`, the model must write one of:

```text
APPROVED FOR SUBMISSION
```

or

```text
REJECTED — RECUT REQUIRED
```

If any of the following are true, the only valid decision is `REJECTED — RECUT REQUIRED`:

- first 10 seconds do not establish either the problem or mainnet proof,
- “Supercharging Creator Fees” is not memorable,
- text is tiny or faint,
- browser footage does not visibly prove live product behavior,
- the pitch reads as a generic AI agent project,
- the technical overview does not show website + docs + code flow,
- any scene feels like filler.

---

## 9B. Production Model Mandate

The coding/rendering model that implements these videos MUST follow this spec. Do not reinterpret it as loose inspiration.

Mandatory behavior:

1. Read this spec before editing any video composition.
2. Read `media/colosseum/design.md`.
3. Read the relevant local `CLAUDE.md` or `AGENTS.md` in each video project directory.
4. Use HyperFrames for generated compositions.
5. Use browser-harness or equivalent real browser capture for website/Explorer footage.
6. Use Impeccable principles as a hard design gate.
7. Use architecture-diagram principles for system-flow scenes.
8. Keep ASCII optional, restrained, and never in front of proof.
9. Verify with rendered frames, not just code inspection.
10. Produce `QC-NOTES.md` and recut until the result is approved.
11. Re-run the VC Pitcher Audit above after first render.
12. Re-run vision review after every recut, not only the final export.

The implementation model must treat every beat as load-bearing. No slop. No filler. No tiny text. No “good enough.” This project has real winning potential; the videos must communicate that level of seriousness.

This is mandatory:

```text
FOLLOW THIS SPEC.
SELF-REVIEW WITH VISION.
REJECT AND RECUT ANY FRAME THAT DOES NOT SERVE THE PITCH, PROOF, OR TECHNICAL DEMO.
```

---

## 10. Final File Targets

```text
media/colosseum/final/rtp-pitch-supercharging-creator-fees.mp4
media/colosseum/final/rtp-technical-overview.mp4

media/colosseum/final/rtp-pitch-supercharging-creator-fees.mov   optional mezzanine
media/colosseum/final/rtp-technical-overview.mov                 optional mezzanine
```

Submission labels:

```text
Presentation video:
RTP Pitch — Supercharging Creator Fees

Technical overview video:
RTP Technical Overview — PDA-Signed Autonomous Treasury Infrastructure
```

---

## 11. Non-Negotiable Final Edits

1. Pitch video must have the structure: **Problem → Solution → Live Proof → Why This Wins → Market → Founder → CTA**.
2. Technical video must start with mainnet proof, then use the website as the demo.
3. The first 10 seconds of each video must show a real hook or proof, not a decorative intro.
4. No tiny or faint text.
5. No inconsistent placement.
6. No generic AI motion graphics.
7. No reliance on facecam or voiceover for comprehension.
8. The viewer must remember the phrase: **Supercharging Creator Fees**.
