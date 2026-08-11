# RTP Colosseum Video Production Spec v3.1 (Impeccable-Audited)

**Builder:** @trade_wife · @tradewife (GitHub) · Solo builder, Sydney
**Hackathon:** SWARMs / Canteen x Colosseum — deadline May 11, 2026
**3-5 word pitch:** **Supercharging Creator Fees**
**Engine:** hermes-apollo demo-video-stack (HyperFrames + browser-harness + Impeccable)

> v3.1: Impeccable audit pass. Fixes: hero-metric pattern in market scene replaced with narrative flow; tech overview website walkthrough expanded to 95s with full page exploration including docs sidebar navigation; flower image added to title/close compositions; architecture layers made typographic not boxed; differentiation items given varied treatments; "Data Drift" preset replaced with project's own visual language. Both videos stand alone visually — no dependency on facecam or audio overlay. The user will record narration separately and overlay a small face-cam square.

---

## Shared Design System

### Color Palette (matches website globals.css)

```
Background:   oklch(8% 0.025 160)    -- void black, emerald-tinted
Surface-0:    oklch(11% 0.02 160)    -- card surfaces
Surface-1:    oklch(14% 0.018 160)   -- elevated surfaces
Surface-2:    oklch(18% 0.015 160)   -- hover states
Coral:        oklch(75% 0.12 30)     -- accent (10% usage, highlights only)
Coral-dim:    oklch(40% 0.06 30)     -- subtle coral borders
Emerald:      oklch(55% 0.1 160)     -- status/success
Emerald-dim:  oklch(35% 0.06 160)    -- borders, tags
Text-primary: #e6f0e8                -- primary text
Text-sec:     oklch(72% 0.03 160)    -- secondary
Text-tert:    oklch(55% 0.025 160)   -- tertiary
Text-muted:   oklch(42% 0.02 160)    -- captions, labels
Border:       oklch(25% 0.015 160)   -- card borders
```

### Typography

```
Display:   Geist Sans (500, 600 weight)
Body:      Geist Sans (400, 500 weight)
Mono:      Geist Mono (code, addresses, metrics)
Headlines: minimum 48px (rendered at 1920x1080)
Body:      minimum 20px
Type scale: fixed rem, not fluid (per Impeccable rule for product UI)
```

### Motion Rules

- `ease-out-quart` / `ease-out-expo` for all entrances. No bounce, no elastic.
- Stagger: 0.15-0.4s between sequential elements
- Entrance animations: `gsap.from()` only (scale 0.97, opacity 0, translateY 8px, 0.4s)
- No exit animations — transitions handle exits (crossfade 0.5s between scenes)
- Background depth: 2-3 decorative elements with slow ambient GSAP drift (seeded PRNG mulberry32)
- Deterministic rendering only — no `Math.random()`, `Date.now()`

### Visual Language

- **Decorative element:** The flower image (`bg-flower.jpg`) ONLY. Used in title and closing compositions as a full-bleed background at low opacity (15-20%). No particles, no canvas effects, no generative backgrounds.
- **Structure:** Typographic, not boxed. Information conveyed through type hierarchy (weight, size, color), not through containers. Surfaces used sparingly — only where the website itself uses them.
- **Spatial rhythm:** Generous. The void (background) is an active element. Content breathes. Asymmetric compositions. Left-aligned by default; centered only for the CTA crescendo.

### Banned (Impeccable absolute bans)

- Side-stripe borders (`border-left > 1px` on cards/callouts)
- Gradient text (`background-clip: text`)
- Particles, canvas noise, generative backgrounds
- Glassmorphism, glow borders
- Purple-blue neon, cyan-on-dark
- Hero metric card grids (big number + small label in a row/grid pattern)
- Crypto casino aesthetic
- Bounce/elastic easing
- Identical card grids (same structure repeated)
- Rounded rectangles with generic drop shadows

### Output Specs

| Property | Value |
|----------|-------|
| Resolution | 1920x1080 |
| Format | MP4 (H.264, CRF 16, preset slow) |
| FPS | 30 |
| Audio | AAC stereo, 48kHz |
| Pitch duration | <=3:00 |
| Tech overview duration | <=3:00 |

---

## VIDEO 1: PITCH VIDEO (<= 3 min)

**Purpose:** Investor pitch. Judges evaluate as "a pitch to potential investors and an application to Colosseum's accelerator."
**Structure:** Problem -> Solution -> Live Proof -> Differentiation -> Market -> Founder -> CTA

### Colosseum Judging Criteria Mapping

| Criterion | Scene | How |
|-----------|-------|-----|
| Founder + Market Fit | 6 | Solo builder arc, prior hackathon placement, self-taught |
| Insight | 2 | "Creator fees exist but earn nothing. One function call changes that." |
| Product + Execution | 3 | Live dashboard, real mainnet TXs, Solana Explorer proof |
| Market Size | 5 | 10K+ token projects, $50B+ monthly volume, zero competitors |
| Communication | All | Clear language, 3-word pitch on screen, no unexplained jargon |
| Viability | 2+5 | Self-funding model, no RTP token, B2B SDK |

### Beat Sheet

```
BEAT  TIME      CONTENT                                          ENGINE          ENERGY
----- --------- ------------------------------------------------ --------------- ------
0     0:00-0:12 HOOK — problem statement                         HyperFrames     High
                Word-by-word reveal (not char-by-char):
                "10,000+ Solana token projects have creator fees."
                "Zero products exist to make those fees earn yield."
                Beat. Then: "Until now." — emerald color shift.
                --void background. No decoration.

1     0:12-0:35 SOLUTION — "Supercharging Creator Fees"           HyperFrames     Medium-High
                Beat A: Title appears left-aligned:
                  "SUPERCHARGING CREATOR FEES"
                  "Resilient Token Protocol . Solana"
                  Geist 72px, --text-primary. Subtitle --text-tert 16px.
                
                Beat B: Capital flow — horizontal text nodes left-to-right:
                  "Creator Fees (SOL) -> Treasury PDA -> Flash Trade CPI
                   -> SOL Yield -> 70/20/10"
                  Each node: --surface-1 bg, 1px --border, arrows scaleX.
                
                Beat C: Three lines, staggered, left-aligned:
                  "No RTP token. Pure infrastructure."
                  "One function call to adopt."
                  "Self-funding. Forever."
                  --text-sec, 18px Geist.

2     0:35-1:10 LIVE DEMO — dashboard + Explorer                  browser-harness Low-Medium
                Sequence:
                1. Navigate to resilientprotocol.xyz — full page load
                2. Read hero: "Every token gets a program-enforced treasury" (2s)
                3. Scroll to vitals: PnL, Treasury SOL, Mainnet TXs, Coverage, Calmar
                4. Continue to "Proven on mainnet" — hover status pill
                5. Current Position card — Survivor 2.69 params
                6. Mainnet TX proof cards — all 4
                7. Click CPI open TX -> Solana Explorer opens
                8. On Explorer: show "Success", Program Logs, invoke_signed
                   Inject callout: "Treasury PDA signs here. No private key."
                9. Back -> scroll to SDK code: registerWithRTP()

                Lower-third throughout: "LIVE . resilientprotocol.xyz"

3     1:10-1:35 DIFFERENTIATION — why this wins                   HyperFrames     Medium
                Four points with VARIED visual treatment (not identical blocks):
                
                Point 1 (appears first, largest text — the primary differentiator):
                  "16 constitutional invariants."
                  "Enforced in Rust AND on-chain. Not a promise — a require! constraint."
                  Title: 28px Geist 600 --text-primary.
                  Body: 16px --text-tert.
                  No container. Left-aligned on void.
                
                Point 2 (indented, different scale — structural claim):
                  "Per-token isolation."
                  "Every mint gets its own Treasury PDA. No shared pool. No honeypot."
                  Title: 20px Geist 500 --text-sec.
                  Body: 14px --text-tert.
                  Indented --space-xl from left margin.
                
                Point 3 (code-style — research claim, Geist Mono for emphasis):
                  "30,000 configs/night · 9-fold WFA · Darwinian evolution"
                  Geist Mono, 16px, --text-sec. Single line, no title/body split.
                  Below in --text-muted 13px: "Not a backtest screenshot."
                
                Point 4 (coral accent on one word — execution claim):
                  "CPI-only execution."
                  "Treasury PDA signs via invoke_signed. No human keypair."
                  Title: 20px Geist 500 --text-primary.
                  "invoke_signed" in --coral to break the emerald monotone.
                  Body: 14px --text-tert.
                
                Below, generous gap:
                "325 Rust tests . 0 failures . 7 Railway services . All green"
                --text-muted, 13px, uppercase, letterspaced.

4     1:35-1:55 MARKET — the gap                                   HyperFrames     High
                NOT a metric grid. A narrative told in three beats.
                
                Beat 1 (left-aligned, single line, conversational):
                  "Over ten thousand Solana token projects have active
                   trading fees right now."
                  --text-primary, 24px Geist 400.
                
                Beat 2 (appears below, different rhythm — a fragment):
                  "$50 billion in monthly DEX volume."
                  --text-sec, 20px Geist 400. Indented --space-xl.
                
                Beat 3 (the gap — appears alone, pause before it):
                  "Zero products put those fees to work."
                  The word "Zero" in --coral. Rest in --text-primary.
                  24px Geist 500. Left-aligned.
                
                After a beat, the summary (narrow, centered):
                  "RTP is the yield infrastructure layer between
                   'fees exist' and 'fees earn yield.'"
                  --text-sec, 16px, max-width 50ch, centered.

5     1:55-2:25 FOUNDER — solo builder story                      HyperFrames     Medium
                Left-aligned profile. No card. No avatar placeholder.
                
                "@trade_wife
                Solo builder . Sydney, Australia
                
                Previous: ZKPUTER (now zktrader) — sovereign pair-programming
                          4th place, Zypherpunk Hackathon (NEAR tier) — first hackathon
                
                Also: ZKVM verified trade execution . OxAuteur cinematography
                      Senpi-Waifu (Rust fork for HL trading)
                
                Background: Arts degree (10+ years ago) . Self-taught via
                YouTube, GitHub, Crypto Twitter"
                
                Then the quote:
                "I believe agentic tokenomics can shape DeFi into what I
                thought it was when I first got into crypto — a better
                alternative than the fiat system for everyone, not just
                insiders and criminals."
                --text-sec, 16px italic Geist.

6     2:25-3:00 CTA — closing crescendo                            HyperFrames     Medium->Low
                Three statements, one at a time, centered, large:
                
                1. "Self-funding treasury."
                2. "No RTP token. Pure infrastructure."
                3. "Any token project. One function call."
                
                Each: 48px Geist, --text-primary, fade in opacity + translateY(8px).
                Previous line shifts to --text-tert.
                
                SDK line appears:
                  registerWithRTP(connection, wallet, { authority: publicKey });
                Geist Mono, 16px, --text-sec.
                
                Final identity:
                  "RESILIENT TOKEN PROTOCOL
                   Supercharging Creator Fees
                   resilientprotocol.xyz
                   github.com/tradewife/resilient-token-protocol
                   @trade_wife"
                
                Fade to --void.
```

### Segment Files

```
clips/pitch/
├── 00-hook.mp4           — 0:00-0:12   HyperFrames kinetic type
├── 01-solution.mp4       — 0:12-0:35   HyperFrames capital flow
├── 02-demo.mp4           — 0:35-1:10   browser-harness capture
├── 03-differentiation.mp4— 1:10-1:35   HyperFrames vertical list
├── 04-market.mp4         — 1:35-1:55   HyperFrames data points
├── 05-founder.mp4        — 1:55-2:25   HyperFrames profile + quote
└── 06-cta.mp4            — 2:25-3:00   HyperFrames closing crescendo
```

### browser-harness Demo Script (Pitch Video)

```
INTERACTION: heightened human-like
- Mouse: sigmoid curves, never linear
- Hover: 500ms dwell on interactive elements
- Scroll: 120px / 300ms — reading rhythm
- Section dwell: 1.5s after load
- Nav clicks: 600ms pause, natural arc
- NO emphasis zoom injection — the site's own design handles emphasis
- Callouts injected via page.evaluate() only on Solana Explorer

new_tab("https://www.resilientprotocol.xyz")
wait_for_load()
sleep(1.5)
capture_screenshot("p-hero.png")

# Scroll to vitals
scroll("down", 200)
sleep(0.8)
capture_screenshot("p-vitals.png")

# Scroll to Live Console
scroll("down", 500)
sleep(1.0)
capture_screenshot("p-console.png")

# Continue to mainnet proof cards
scroll("down", 400)
sleep(0.8)
capture_screenshot("p-proofs.png")

# Click the CPI open TX -> Explorer
click_at_xy(proof_card_x, proof_card_y)
sleep(2.0)
capture_screenshot("p-explorer.png")

# Inject callout on Explorer showing invoke_signed
page.evaluate('...callout injection...')
sleep(1.0)
capture_screenshot("p-explorer-callout.png")

# Back to dashboard
go_back()
sleep(1.5)

# Scroll to SDK code
scroll("down", 600)
sleep(0.8)
capture_screenshot("p-sdk.png")
```

### Audio Plan (Pitch)

- Background music: ambient electronic, 80-90 BPM, calm/emerald energy
- No narration by default (user will overlay facecam audio)
- Beat-sync optional: sync counter reveals to beat hits
- Music fades to 50% volume during demo section (0:35-1:10)
- Final encode merges: video + narration track + background music at 12% volume

---

## VIDEO 2: TECHNICAL OVERVIEW (<= 3 min)

**Purpose:** Architecture deep-dive. Proves execution quality. **The website IS the demo.**
**Structure:** Architecture context (15s) -> Full website exploration (115s) -> CPI code flow (20s) -> Close (10s)

> **CRITICAL DIRECTIVE:** The website must be fully explored and used. Every section, every page, every interactive element. A human showing their product would click through everything naturally — we do the same. The website walkthrough is 115 seconds (64% of the video). HyperFrames is used ONLY for the 15-second architecture opener, 20-second CPI code flow, and 10-second close. Everything else is the real, live site being used by a real browser.

### Beat Sheet

```
BEAT  TIME      CONTENT                                          ENGINE          ENERGY
----- --------- ------------------------------------------------ --------------- ------
0     0:00-0:15 ARCHITECTURE — three-layer stack                 HyperFrames     Medium
                TYPOGRAPHIC, not boxed. Three lines of text, no containers:
                
                Line 1: "On-chain: Anchor . PDA-owned . 19 instructions"
                Line 2: "Runtime: Rust . 6 wings . 325 tests"
                Line 3: "Research: Python . 30K configs/night . 9-fold WFA"
                
                Each line: --text-primary Geist 16px 500.
                No surface backgrounds. No borders. Just type on --void.
                Lines appear with staggered entrance (0.5s apart).
                
                After all three land:
                "Signing: Treasury PDA (invoke_signed). Capital stays on Solana."
                --text-muted, 12px. Fades in below.

1     0:15-2:10 FULL WEBSITE EXPLORATION                          browser-harness Low-Medium
                115 seconds. Every section. Every page. The product IS the demo.
                
                Part A — Dashboard Hero (0:15-0:30, 15s):
                  1. Navigate to resilientprotocol.xyz — watch full page load
                  2. Dwell on hero: read title, subtitle, flower image
                  3. Read vitals strip: PnL, Treasury SOL, Mainnet TXs (4),
                     Test coverage (325+5), Calmar (44.89)
                  4. Natural scroll rhythm, reading each element
                
                Part B — Live Console + Mainnet Proof (0:30-1:00, 30s):
                  5. Scroll to "The yield engine is running. Right now."
                  6. Read the status pill (connecting/position state)
                  7. Current Position card: read Survivor 2.69 params
                  8. Cumulative PnL chart area — show state
                  9. Scroll to On-Chain Proof grid
                  10. Read all 4 TX cards (hover each):
                      OPEN CPI invoke_signed . 99,214 CU
                      CLOSE SOL returned . settled mainnet
                      OPEN REST autonomous . score = 0.400
                      CLOSE REST autonomous . settled mainnet
                  11. Read extras links: Devnet redistribution TX,
                      Treasury program, GitHub source
                  12. Click the CPI OPEN TX -> Solana Explorer opens
                  13. On Explorer: locate "Success" status
                  14. Scroll to Program Instruction Logs
                  15. Find invoke_signed in the logs
                      Inject callout: "PDA signs the CPI. No human keypair."
                  16. Dwell 2s on the proof
                  17. Back button -> return to dashboard
                
                Part C — Trust Architecture (1:00-1:15, 15s):
                  18. Scroll to "Agents propose. The program disposes."
                  19. Read each invariant cell (hover briefly):
                      PDA Ownership, Per-Token Isolation, Emergency Freeze,
                      Strategy Lifecycle, CPI-Only Execution, Phase Irreversible
                  20. Read enforcement description at bottom
                
                Part D — Research Pipeline (1:15-1:35, 20s):
                  21. Scroll to "30,000 hypotheses tested every night"
                  22. Read each pipeline step (scroll through):
                      01 Grid Search (30,000)
                      02 Walk-Forward (9 folds)
                      03 Darwinian (5x50)
                      04 Overfitting Detection (3 checks)
                      05 Full-Sim Validation (0.1% fees)
                  23. Arrive at Validated Strategy card:
                      Read each metric: Calmar 44.89, +554%, 12.3% DD,
                      100% consistency, 0 liquidations, 16,228 candidates
                  24. Read Active Strategy panel (Survivor 2.69 params)
                  25. Read Swarm Architecture panel (6 wings list)
                
                Part E — Integration + Docs Page (1:35-2:00, 25s):
                  26. Scroll to "One function call"
                  27. Read the SDK code snippet
                  28. Click "Read the docs ->" in nav
                  29. Docs page loads — observe sidebar navigation
                  30. Read "What is RTP?" article
                  31. Scroll through architecture section (3-layer diagram)
                  32. Read comparison table (Squads vs Yield Agg vs RTP)
                  33. Read "What's Been Built" status table
                  34. Click sidebar: "Treasury PDA"
                  35. Read treasury PDA docs content
                  36. Click sidebar: "Fee Routing"
                  37. Read fee routing content (pump.fun, Bags.fm, Raydium)
                  38. Click sidebar: "Security Model"
                  39. Read security model content
                
                Part F — Launch Page (2:00-2:10, 10s):
                  40. Click "Launch" in nav
                  41. Launch page loads — show the token creation form
                  42. Read the form fields, beta toggle, wallet connect button
                  43. Navigate back to dashboard via brand logo click

2     2:10-2:50 CPI CODE FLOW — what the website can't show       HyperFrames     Medium-High
                Four code nodes in a horizontal flow. TYPOGRAPHIC, no boxes.
                
                "bridge.rs        ->  chain_client.rs  ->  lib.rs (Anchor)  ->  Flash Trade
                 ExecutePermit         build_open_ix(       invoke_signed(        Position
                 payload               treasury_pda,        &ix,                  opened
                                       seeds)               accounts,             on-chain
                                                            seeds)"
                
                Code: Geist Mono 14px, --text-sec. On --void. No containers.
                Arrows: --text-muted, drawn as text "->".
                
                Result line:
                "99,214 CU consumed . Confirmed mainnet . TX 2bLg1Fu..."
                --text-muted, 12px. Below the flow.

3     2:50-3:00 CLOSE — identity                                  HyperFrames     Medium->Low
                Clean identity card. Flower image at 15% opacity as background.
                
                "RESILIENT TOKEN PROTOCOL
                 Supercharging Creator Fees"
                
                --text-primary Geist 500, 36px name, 16px tagline.
                
                Below:
                "resilientprotocol.xyz
                 github.com/tradewife/resilient-token-protocol
                 @trade_wife"
                --text-tert, 14px.
                
                Fade to --void.
```

### Segment Files

```
clips/tech/
├── 00-architecture.mp4    — 0:00-0:15   HyperFrames typographic stack
├── 01-website.mp4         — 0:15-2:10   browser-harness FULL exploration (115s)
├── 02-cpi-flow.mp4        — 2:10-2:50   HyperFrames code flow
└── 03-close.mp4           — 2:50-3:00   HyperFrames identity card
```

### browser-harness Demo Script (Technical Video — FULL Exploration)

```
INTERACTION: heightened human-like
- Mouse: sigmoid curves, never linear
- Hover: 500ms dwell on interactive elements
- Scroll: 120px / 300ms — reading rhythm, NOT speed-run
- Section dwell: 2.0s after each section loads (slower than pitch — we're teaching)
- Nav clicks: 600ms pause, natural arc
- Sidebar clicks: 800ms pause (docs sidebar feels different than nav)
- Callouts ONLY on Solana Explorer — never on the dashboard itself

# --- Part A: Dashboard Hero ---
new_tab("https://www.resilientprotocol.xyz")
wait_for_load()
sleep(2.0)
capture_screenshot("t-hero.png")

scroll("down", 200)
sleep(1.0)
capture_screenshot("t-vitals.png")

# --- Part B: Live Console + Mainnet Proof ---
scroll("down", 500)
sleep(1.5)
capture_screenshot("t-console.png")

scroll("down", 200)
sleep(0.8)
capture_screenshot("t-position.png")

scroll("down", 300)
sleep(1.0)
capture_screenshot("t-proofs.png")

# Hover each TX card briefly
move_to(proof_card_1_x, proof_card_1_y)
sleep(0.5)
move_to(proof_card_2_x, proof_card_2_y)
sleep(0.5)
move_to(proof_card_3_x, proof_card_3_y)
sleep(0.5)
move_to(proof_card_4_x, proof_card_4_y)
sleep(0.5)
capture_screenshot("t-proofs-hover.png")

# Click into Explorer for CPI proof
click_at_xy(proof_card_1_x, proof_card_1_y)
sleep(2.5)
capture_screenshot("t-explorer.png")

scroll("down", 300)
sleep(1.0)
capture_screenshot("t-explorer-logs.png")

# Inject callout at invoke_signed
page.evaluate('...')
sleep(1.5)
capture_screenshot("t-explorer-callout.png")

go_back()
sleep(2.0)

# --- Part C: Trust Architecture ---
scroll("down", 600)
sleep(2.0)
capture_screenshot("t-trust.png")

# Hover each invariant cell
move_to(inv_cell_1_x, inv_cell_1_y)
sleep(0.4)
move_to(inv_cell_2_x, inv_cell_2_y)
sleep(0.4)
move_to(inv_cell_3_x, inv_cell_3_y)
sleep(0.4)
move_to(inv_cell_4_x, inv_cell_4_y)
sleep(0.4)
move_to(inv_cell_5_x, inv_cell_5_y)
sleep(0.4)
move_to(inv_cell_6_x, inv_cell_6_y)
sleep(0.4)
capture_screenshot("t-trust-hover.png")

# --- Part D: Research Pipeline ---
scroll("down", 700)
sleep(1.5)
capture_screenshot("t-pipeline.png")

scroll("down", 300)
sleep(1.0)
capture_screenshot("t-validated.png")

scroll("down", 200)
sleep(1.0)
capture_screenshot("t-strategy-wings.png")

# --- Part E: Integration + Docs Page ---
scroll("down", 400)
sleep(1.0)
capture_screenshot("t-integrate.png")

# Navigate to docs
click_at_xy(docs_nav_x, docs_nav_y)
wait_for_load()
sleep(2.0)
capture_screenshot("t-docs-landing.png")

# Scroll through "What is RTP?" article
scroll("down", 400)
sleep(1.5)
capture_screenshot("t-docs-what.png")

scroll("down", 400)
sleep(1.0)
capture_screenshot("t-docs-comparison.png")

scroll("down", 300)
sleep(1.0)
capture_screenshot("t-docs-built.png")

# Click sidebar: Treasury PDA
click_at_xy(sidebar_treasury_x, sidebar_treasury_y)
sleep(1.5)
capture_screenshot("t-docs-treasury.png")

scroll("down", 200)
sleep(0.8)
capture_screenshot("t-docs-treasury-detail.png")

# Click sidebar: Fee Routing
click_at_xy(sidebar_fee_x, sidebar_fee_y)
sleep(1.5)
capture_screenshot("t-docs-fees.png")

scroll("down", 200)
sleep(0.8)
capture_screenshot("t-docs-fees-detail.png")

# Click sidebar: Security Model
click_at_xy(sidebar_security_x, sidebar_security_y)
sleep(1.5)
capture_screenshot("t-docs-security.png")

scroll("down", 200)
sleep(0.8)
capture_screenshot("t-docs-security-detail.png")

# --- Part F: Launch Page ---
click_at_xy(launch_nav_x, launch_nav_y)
wait_for_load()
sleep(2.0)
capture_screenshot("t-launch.png")

scroll("down", 200)
sleep(0.8)
capture_screenshot("t-launch-form.png")

# Return to dashboard via brand click
click_at_xy(brand_x, brand_y)
sleep(1.5)
capture_screenshot("t-return.png")
```

### Audio Plan (Technical)

- Background music: ambient electronic, 70-80 BPM, calm/technical energy
- No narration by default (user will overlay)
- Lower energy than pitch video (deep-dive, not pitch)
- Final encode merges: video + narration track + background music at 12%

---

## Production Pipeline

### Implementation Sequence

#### Phase 1: Design Foundation (Impeccable)
1. Load dashboard `globals.css` color tokens -> define HyperFrames CSS variables
2. Set register: Brand (pitch video), Product (technical video)
3. Color strategy: Restrained (emerald-dominant, coral 10% accent)
4. Visual language: project's own — emerald void, editorial precision, flower as sole decoration. NOT a named HyperFrames preset.
5. Font: Geist Sans + Geist Mono (user-specified)

#### Phase 2: Shared Asset Build
1. HyperFrames composition templates: title card (flower bg), closing card (flower bg)
2. Capital flow diagram component (horizontal text nodes with text arrows)
3. Code flow diagram component (bridge.rs -> chain_client -> lib.rs -> Flash Trade)
4. Typographic stagger component (lines appearing with 0.5s delay)

#### Phase 3: Pitch Video Render
1. Render 00-hook (HyperFrames word-by-word reveal)
2. Render 01-solution (HyperFrames title + capital flow + three lines)
3. Run browser-harness capture of dashboard + Explorer (02-demo)
4. Render 03-differentiation (HyperFrames vertical list)
5. Render 04-market (HyperFrames data points, "0" in coral)
6. Render 05-founder (HyperFrames profile + quote)
7. Render 06-cta (HyperFrames closing crescendo + identity)
8. ffmpeg concat all clips with crossfade transitions

#### Phase 4: Technical Video Render
1. Render 00-architecture (HyperFrames typographic 3-line stack)
2. Run browser-harness full website exploration (01-website) — 115s, ALL pages
3. Render 02-cpi-flow (HyperFrames typographic code flow)
4. Render 03-close (HyperFrames identity card with flower background)
5. ffmpeg concat all clips with crossfade transitions

#### Phase 5: Quality Gate (Impeccable)
1. `npx hyperframes validate` on every composition — fix WCAG AA failures
2. AI slop test: scan for banned patterns
3. Category-reflex check: is palette guessable from "crypto"? (emerald/coral should pass)
4. Animation choreography review — verify rhythm, no dead frames
5. Final encode: H.264, CRF 16, preset slow, faststart, yuv420p

### Render Commands

```bash
# Concatenate clips with crossfade
ffmpeg -f concat -safe 0 -i clips/pitch/manifest.txt \
  -c:v libx264 -preset slow -crf 16 \
  -pix_fmt yuv420p -movflags +faststart pitch_raw.mp4

# Add narration + background music
ffmpeg -i pitch_raw.mp4 -i narration.mp3 -i bg.mp3 \
  -filter_complex "[1:a]volume=1.0[vo];[2:a]volume=0.12[bg];[vo][bg]amix=inputs=2" \
  -c:v copy -c:a aac -shortest pitch_final.mp4

# Same for technical overview
ffmpeg -f concat -safe 0 -i clips/tech/manifest.txt \
  -c:v libx264 -preset slow -crf 16 \
  -pix_fmt yuv420p -movflags +faststart tech_raw.mp4

ffmpeg -i tech_raw.mp4 -i narration.mp3 -i bg.mp3 \
  -filter_complex "[1:a]volume=1.0[vo];[2:a]volume=0.12[bg];[vo][bg]amix=inputs=2" \
  -c:v copy -c:a aac -shortest tech_final.mp4
```

### Callout Injection (Solana Explorer)

```javascript
// Injected via page.evaluate() when on Solana Explorer
const callout = document.createElement('div');
callout.textContent = 'Treasury PDA signs here. No private key.';
callout.style.cssText = `
  font: 12px 'Geist Mono', monospace;
  color: oklch(55% 0.1 160);
  background: oklch(8% 0.025 160 / 0.9);
  border: 1px solid oklch(25% 0.015 160);
  padding: 4px 8px;
  border-radius: 4px;
  position: fixed;
  bottom: 20px;
  left: 20px;
  z-index: 10000;
`;
document.body.appendChild(callout);
```

---

## Impeccable Audit Checklist (v3.1)

| Check | Status | Notes |
|-------|--------|-------|
| No side-stripe borders (`border-left > 1px`) | Pass | None used |
| No gradient text | Pass | All text solid color |
| No particles / canvas noise / generative backgrounds | Pass | Flower image only (as on website) |
| No glassmorphism / glow borders | Pass | None |
| No hero metric card grids | **Fixed** | Market scene was big-number grid. Now narrative prose beats |
| No identical card grids | **Fixed** | Differentiation had 4 identical blocks. Now varied treatment per point |
| No bounce/elastic easing | Pass | ease-out-quart/expo only |
| No purple-blue / cyan-on-dark | Pass | Emerald OKLCH + coral only |
| Colors match production CSS (OKLCH) | Pass | All values from globals.css |
| Font: Geist (user request) | Pass | Geist Sans + Geist Mono |
| Type scale: fixed rem, not fluid | Pass | Per Impeccable product UI rule |
| Neutrals tinted toward brand hue (160) | Pass | Matches globals.css |
| Technical overview is website-first | **Fixed** | Expanded from 85s to 115s (64% of video). Full docs sidebar exploration added |
| No decoration competing with content | Pass | Flower at 15-20% opacity only in title/close |
| Deterministic rendering (no Math.random) | Pass | Seeded PRNG mulberry32 |
| Layout before animation (gsap.from only) | Pass | All entrances from hero frame |
| All transitions mandatory, no jump cuts | Pass | Crossfade 0.5s between all scenes |
| Background depth (2-3 ambient elements) | Pass | Slow GSAP drift on void |
| Architecture layers typographic not boxed | **Fixed** | Removed --surface-0 bg + borders. Text on void only |
| CPI flow typographic not boxed | **Fixed** | Removed containers. Code as text on void |
| Docs sidebar fully navigated | **Fixed** | Treasury PDA, Fee Routing, Security Model all clicked and read |
| Launch page explored | **Fixed** | Token creation form shown, beta toggle visible |
| Terminal simulation removed | **Removed** | Replaced with more website time. Tests/infra visible on dashboard |
| Category-reflex: palette guessable from "crypto"? | Pass | Emerald+coral is NOT the standard purple/cyan crypto palette |

---

## Recording Order

> Record technical overview first (show what you built, straightforward). Then pitch video — confidence from the technical walkthrough carries into delivery.

---

## Key Production Notes

1. **Deterministic rendering only** — seeded PRNG (mulberry32), no Math.random/Date.now
2. **Layout before animation** — position elements at hero frame first, then gsap.from()
3. **No exit animations** — transitions handle exits. Final scene only may fade out
4. **Transitions are mandatory** — crossfade between all scenes, no jump cuts
5. **Tinted neutrals** — never pure #000 or #fff, always toward emerald hue 160
6. **Background depth** — 2-3 ambient elements per scene (slow GSAP drift)
7. **Geist font** — user-specified. Geist Sans for display/body, Geist Mono for code
8. **Coral at 10%** — accents only (highlights, active states, the "0" gap moment)
9. **Min 48px headlines** — optimized for screen visibility and judge clarity
10. **Both videos standalone** — must nail the pitch and demo without facecam or audio
11. **browser-harness first nav** — always `new_tab()`, never `goto_url()`
12. **Screenshot device pixels** — divide by devicePixelRatio before clicking
13. **Synchronous timeline construction** — never async/await in GSAP timeline setup
14. **HyperFrames repeat** — always calculate exact repeat count, never `repeat: -1`
15. **ffmpeg pipe** — never stderr=subprocess.PIPE with long-running ffmpeg
