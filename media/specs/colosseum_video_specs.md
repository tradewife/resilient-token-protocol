# RTP Colosseum Video Specs — Pitch + Technical Overview (v2 · Audited)

**Builder:** @trade_wife · @tradewife (GitHub) · Solo builder, Sydney
**Hackathon:** SWARMs / Canteen × Colosseum — deadline May 11, 2026
**3–5 word pitch:** **Supercharging Creator Fees**

---

## Design Foundation

> [!IMPORTANT]
> Both videos work as standalone visual experiences. Audio narration recorded by the builder and overlaid. Optional small face-cam square in one corner. All text and proof must be readable without narration.

### Visual Language (from `.impeccable.md` + `globals.css`)

- **Theme:** Dark. Deep emerald void (`oklch(8% 0.025 160)`). Every neutral tints toward hue 160.
- **Decorative interest:** The flower image only. No particles, no canvas effects, no generative backgrounds. The UI is precise and quiet.
- **Colors (OKLCH, matching production CSS):**
  - Emerald: `oklch(55% 0.1 160)` — life, yield, live status
  - Coral: `oklch(75% 0.12 30)` — peak moments, warnings, accents (10% usage max)
  - Text primary: `#e6f0e8` · Secondary: `oklch(72% 0.03 160)` · Tertiary: `oklch(55% 0.025 160)`
  - Surfaces: `oklch(11% 0.02 160)` / `oklch(14% 0.018 160)` / `oklch(18% 0.015 160)`
  - Border: `oklch(25% 0.015 160)`
- **Typography:**
  - Display: `Geist`, system-ui, sans-serif
  - Body: `Geist`, system-ui, sans-serif
  - Mono: `Geist Mono`, `SF Mono`, monospace
  - Fixed `rem` scale (no fluid type in product UI per Impeccable). Min 48px headlines in compositions, 20px body.
- **Motion:** `ease-out-quart` / `expo` only. No bounce, no elastic. Stagger 0.15–0.4s.
- **Banned (Impeccable absolute bans + .impeccable.md anti-refs):**
  - Side-stripe borders (`border-left > 1px` on cards/callouts)
  - Gradient text (`background-clip: text`)
  - Particles, canvas noise, generative backgrounds
  - Glassmorphism, glow borders
  - Purple-blue neon, cyan-on-dark
  - Hero metric card grids (big number + small label repeated in grid)
  - Crypto casino aesthetic

### Output Specs

| Property | Value |
|----------|-------|
| Resolution | 1920×1080 |
| Format | MP4 (H.264, CRF 16) |
| FPS | 24 |
| Audio | AAC stereo, 48kHz |
| Max duration | 180s each |

---

# VIDEO 1: PITCH VIDEO (≤3 min)

**Purpose:** Investor pitch. Judges evaluate as "a pitch to potential investors and an application to Colosseum's accelerator."

**Structure:** Problem → Solution → Live Proof → Differentiation → Market → Founder → CTA

### Colosseum Judging Criteria Mapping

| Criterion | Scene | How |
|-----------|-------|-----|
| Founder + Market Fit | 6 | Solo builder arc, 5 projects in 6 months, prior hackathon placement |
| Insight | 2 | "Creator fees exist but earn nothing. One function call changes that." |
| Product + Execution | 3 | Live dashboard, real mainnet TXs, Solana Explorer proof |
| Market Size | 5 | 10K+ token projects, $50B+ monthly volume, zero competitors |
| Communication | All | Clear language, 3-word pitch on screen, no unexplained jargon |
| Viability | 2+5 | Self-funding model, no RTP token, B2B SDK |

---

### SCENE 1 — Hook (0:00–0:12) · 12s · HyperFrames

**On-screen text (large, center, staggered word-by-word reveal):**

```
10,000+ Solana token projects have creator fees.
Zero products exist to make those fees earn yield.
```

**Visual:** `--void` background. No decoration. Text appears word-by-word (not character-by-character). After "Zero products" lands, a beat. Then:

```
Until now.
```

Emerald color shift on "now" (not glow — just color change to `--emerald`). Clean cut to Scene 2.

**Narration guide:**
> "There are over ten thousand token projects on Solana with active trading fees. Those fees sit in wallets, earning nothing. No product exists to put them to work. Until now."

---

### SCENE 2 — Solution (0:12–0:35) · 23s · HyperFrames

**Beat 1 (0:12–0:18):** Title appears, left-aligned:

```
SUPERCHARGING CREATOR FEES
Resilient Token Protocol · Solana
```

Geist, 72px, `--text-primary`. Subtitle in `--text-tertiary`, 16px.

**Beat 2 (0:18–0:26):** Capital flow — a single horizontal line of text nodes connected by `→`, appearing left-to-right:

```
Creator Fees (SOL)  →  Treasury PDA  →  Flash Trade CPI  →  SOL Yield  →  70/20/10
```

Each node: `--surface-1` background, 1px `--border`, `--text-secondary` text. Arrows drawn via `scaleX` transform. Simple, structural, editorial.

**Beat 3 (0:26–0:35):** Three lines, staggered, left-aligned:
- `No RTP token. Pure infrastructure.`
- `One function call to adopt.`
- `Self-funding. Forever.`

`--text-secondary`, 18px Geist. No decoration.

**Narration guide:**
> "Resilient Token Protocol. Supercharging creator fees. Token projects route their trading fees to RTP. An autonomous swarm generates yield via on-chain perpetuals on Flash Trade. Yield flows back — seventy, twenty, ten — enforced on-chain. No RTP token. Pure infrastructure. One function call to adopt."

---

### SCENE 3 — Live Proof: Dashboard (0:35–1:10) · 35s · browser-harness

**The product demo. Captured live on the actual website.**

**Sequence:**
1. Navigate to `https://www.resilientprotocol.xyz` — full page load, natural dwell
2. Read the hero: "Every token gets a program-enforced treasury" — 2s dwell
3. Scroll to vitals strip — pause on Cumulative PnL, Treasury SOL, Mainnet TXs, Test coverage, Calmar
4. Continue to §1 "Proven on mainnet" — hover on the live status pill
5. Scroll to Validated Strategy card — dwell on each metric: `Calmar 44.89 · +554% · 100% consistency · 0 liquidations`
6. Scroll to mainnet TX proof cards — click one → Solana Explorer opens → show "Success" and `invoke_signed` in program logs
7. Return to dashboard, scroll to SDK code: `registerWithRTP()`

**Callout injection (via `page.evaluate`, part of the recording):**
When on Solana Explorer showing invoke_signed:
```
"Treasury PDA signs here. No private key."
```
Style: `font: 12px Geist Mono; color: oklch(55% 0.1 160); background: oklch(8% 0.025 160 / 0.9); border: 1px solid oklch(25% 0.015 160); padding: 4px 8px; border-radius: 4px;`

**Lower-third throughout:**
```
● LIVE · resilientprotocol.xyz
```

**Narration guide:**
> "This is the live dashboard. The yield engine is running right now on Railway — no human in the loop. Calmar ratio forty-four point eight nine. Five hundred fifty-four percent compounded return. One hundred percent consistency across nine walk-forward folds. Zero liquidations. And these are real mainnet transactions. You can see invoke_signed right here in the program logs. The Treasury PDA signs. No private key exists."

---

### SCENE 4 — Differentiation (1:10–1:35) · 25s · HyperFrames

**Four differentiation points. NOT a card grid — a vertical list with generous spacing:**

```
Constitutional Governance
  soulcontract.md enforced in Rust AND on-chain. 16 invariants.
  Not a promise — a require! constraint.

Per-Token Isolation
  Every token gets its own Treasury PDA.
  No shared pool. No honeypot.

Proven Research Engine
  30K configs/night. 9-fold WFA. Darwinian evolution.
  Not a backtest screenshot.

CPI-Only Execution
  Treasury PDA signs via invoke_signed into Flash Trade.
  No human keypair. Fully auditable on-chain.
```

Title in `--text-primary`, 18px, Geist weight 500. Description in `--text-tertiary`, 14px. Each block separated by `--space-xl`. Left-aligned. No borders, no cards, no decoration.

Below, after a `--space-2xl` gap:
```
325 Rust tests · 0 failures · 7 Railway services · All green
```
`--text-muted`, 13px, uppercase, letterspaced.

**Narration guide:**
> "What makes this different. Constitutional governance — sixteen invariants enforced in Rust and on-chain. Per-token isolation — no shared pool, no honeypot. A proven research engine — thirty thousand configs tested every night. And CPI-only execution — the Treasury PDA signs via invoke_signed. Three hundred twenty-five tests. Zero failures."

---

### SCENE 5 — Market (1:35–1:55) · 20s · HyperFrames

**Three data points, vertical, left-aligned, with generous type contrast:**

```
10,000+          Solana token projects with active trading fees
$50B+            Monthly Solana DEX volume
0                Existing products for creator fee yield
```

Large numbers in `--text-primary`, 56px Geist weight 400. Labels in `--text-tertiary`, 16px. Numbers use `font-variant-numeric: tabular-nums`. The "0" appears last, colored `--coral` momentarily — the gap.

Below:
```
RTP is the yield infrastructure layer between
"fees exist" and "fees earn yield."
```
`--text-secondary`, 16px, max-width 50ch.

**Narration guide:**
> "Over ten thousand Solana token projects with active trading fees. Fifty billion dollars in monthly DEX volume. And zero products in this category. RTP is the missing layer."

---

### SCENE 6 — Founder (1:55–2:25) · 30s · HyperFrames

**Left-aligned profile. No card. No avatar placeholder.**

```
@trade_wife
Solo builder · Sydney, Australia

Previous: ZKPUTER (now zktrader) — sovereign pair-programming harness
          4th place, Zypherpunk Hackathon (NEAR tier) — first hackathon ever

Also built: ZKVM verified trade execution · OxAuteur cinematography intelligence
            Senpi-Waifu (Rust fork for HL trading)

Background: Arts degree (10+ years ago) · Self-taught via YouTube,
            GitHub, Crypto Twitter · Building full-time in crypto
```

Name in `--text-primary`, 20px Geist. Details in `--text-secondary`, 14px.

After profile, the quote appears:

```
"I believe agentic tokenomics can shape DeFi into what I thought
it was when I first got into crypto — a better alternative than
the fiat system for everyone, not just insiders and criminals."
```

`--text-secondary`, 16px italic Geist. No decorative bar, no quotation marks icon. Just the text, offset with `--space-xl` top margin.

**Narration guide:**
> "I'm trade_wife. Solo builder from Sydney. ZKPUTER placed fourth in the Zypherpunk Hackathon — my first hackathon ever. Five projects in six months. Arts degree ten years ago. Everything I know about code, I learned from the internet. I believe agentic tokenomics can shape DeFi into a better system for everyone."

---

### SCENE 7 — CTA (2:25–3:00) · 35s · HyperFrames

**Three statements, one at a time, centered, large:**

1. `Self-funding treasury.` (2:25)
2. `No RTP token. Pure infrastructure.` (2:32)
3. `Any token project. One function call.` (2:39)

Each: 48px Geist, `--text-primary`, fade in with `opacity` + `translateY(8px)`. Previous line stays visible but shifts to `--text-tertiary`.

**(2:44)** The SDK line:
```typescript
registerWithRTP(connection, wallet, { authority: publicKey });
```
`Geist Mono`, 16px, `--text-secondary`.

**(2:50)** Final identity:
```
RESILIENT TOKEN PROTOCOL
Supercharging Creator Fees

resilientprotocol.xyz
github.com/tradewife/resilient-token-protocol
@trade_wife
```
`--text-primary` for name, `--text-tertiary` for links.

**(2:57)** Fade to `--void`.

**Narration guide:**
> "Self-funding treasury. No token. Pure infrastructure. Any token project integrates with one function call. The swarm runs. Improves. Evolves. Funded by its own yield. Forever. Resilient Token Protocol. Supercharging creator fees."

---

# VIDEO 2: TECHNICAL OVERVIEW (≤3 min)

**Purpose:** Architecture deep-dive. Proves execution quality. The website IS the demo — explore it fully and naturally like a human would.

**Structure:** Website walkthrough (the bulk of the video) + brief architecture context + terminal/infrastructure proof

> [!IMPORTANT]
> This video is primarily a browser-harness recording of the live website. HyperFrames compositions are used only for the brief architecture context (Scene 1) and closing identity (Scene 5). Everything else is the real, live site.

---

### SCENE 1 — Architecture Context (0:00–0:25) · 25s · HyperFrames

**Three-layer stack, built bottom-up with staggered entrance (0.5s apart):**

```
ON-CHAIN (Solana / Anchor)
Treasury PDA · Flash Trade CPI · 19 instructions · Phase evolution

SWARM RUNTIME (Rust · 325 tests)
Coordinator → 6 Wings: Trading · Security · Evolve · Knowledge · Audit · Futureproof

RESEARCH LAYER (Python)
Night Shift · 30K configs · 9-fold WFA · Darwinian evolution
```

Each layer: full-width, `--surface-0` background, 1px `--border`, `--space-md` padding. Layer title in `--text-primary` Geist 16px weight 500. Details in `--text-tertiary` 13px.

After stack builds, a single line below:
```
Signing: Treasury PDA (invoke_signed) · Capital never leaves Solana
```
`--text-muted`, 12px uppercase letterspaced.

**Narration guide:**
> "Three layers. A Solana Anchor program with nineteen instructions and PDA-owned treasury. A Rust swarm with six coordinated wings. And a Python research layer testing thirty thousand configurations per night. Let me walk you through the live product."

---

### SCENE 2 — Full Website Walkthrough (0:25–1:50) · 85s · browser-harness

**This is the core of the technical video. Navigate the full site naturally.**

**Part A — Dashboard (0:25–0:55) · 30s:**
1. Navigate to `https://www.resilientprotocol.xyz`
2. Observe hero: flower image, title, vitals strip — dwell naturally
3. Scroll to §1 "Proven on mainnet" — read the status pill (live/watching/position)
4. Examine Current Position card — show the Survivor 2.69 parameters
5. Examine Cumulative PnL chart — hover over data points if trades exist
6. Scroll through Recent Tape if trades are showing
7. Arrive at On-Chain Proof section — examine all 4 mainnet TX cards
8. Click the CPI open TX → Solana Explorer opens
9. On Explorer: show "Success", scroll to Program Logs, find `invoke_signed`
   - Inject callout: `"PDA signs the CPI. No human keypair."`
10. Back button → return to dashboard

**Part B — Trust Model (0:55–1:10) · 15s:**
11. Scroll to §2 "Trustless by design" — read the 6 invariant cells
12. Read the enforcement description: soulguard.rs + on-chain require!

**Part C — Research Pipeline (1:10–1:25) · 15s:**
13. Scroll to §3 "Self-improving research engine" — read through the 5 pipeline steps
14. Arrive at Validated Strategy card — dwell on each metric
15. Examine Active Strategy panel (Survivor 2.69 params) and Swarm Architecture panel (6 wings)

**Part D — Integration + Docs (1:25–1:50) · 25s:**
16. Scroll to §4 "Integrate" — read the SDK code snippet
17. Click "Read the docs →" in the nav
18. On docs page: browse Architecture section, scroll through the trust model documentation
19. Navigate to Launch page — show the token creation form (devnet)
20. Return to dashboard via nav

**Interaction style throughout:**
```
Mouse: sigmoid curves, never linear snaps
Hover dwell: 500ms on interactive elements
Scroll: 120px per 300ms — reading rhythm
Section dwell: 1.5s after each section loads
Tab/nav clicks: 600ms pause, natural cursor arc
No emphasis zoom injection — the site's own design handles emphasis
```

**Narration guide:**
> "Here's the live product at resilientprotocol.xyz. [Narrate what's on screen as you browse — the status, the strategy parameters, the trade history, the mainnet proof, the trust model, the research pipeline, the SDK integration, the docs, the launch page.]"

---

### SCENE 3 — CPI Code Flow (1:50–2:15) · 25s · HyperFrames

**Four code nodes in a horizontal flow (the one thing the website can't show):**

```
bridge.rs              chain_client.rs         lib.rs (Anchor)          Flash Trade
ExecutePermit    →     build_open_ix(          invoke_signed(           Position
  payload               treasury_pda,            &ix,                  opened
                         seeds)                   accounts,             on-chain
                                                  seeds)
```

Each node: `--surface-1`, 1px `--border`, code in `Geist Mono` 14px. Arrows drawn with `scaleX`.

Result line:
```
99,214 CU consumed · Confirmed mainnet · TX 2bLg1Fu...
```
`--text-muted`, 12px.

**Narration guide:**
> "The CPI execution flow. The Trading Wing receives a validated strategy via bridge.rs. The chain client builds the instruction with Treasury PDA seeds. The on-chain handler calls invoke_signed into Flash Trade. Ninety-nine thousand compute units. Confirmed mainnet."

---

### SCENE 4 — Tests + Infrastructure (2:15–2:45) · 30s · HyperFrames (animated terminal)

**Terminal simulation with human-like keystroke rhythm:**

```bash
$ cargo test --lib 2>&1 | tail -3
  test result: ok. 325 passed; 0 failed; 0 ignored

$ rtp status services
  rtp-trader        ● ONLINE    Flash Trade SOL/USDT
  rtp-dashboard     ● ONLINE    resilientprotocol.xyz
  rtp-devnet-loop   ● SUCCESS   LLM-driven evolution (6h)
  rtp-night-shift   ● SUCCESS   30K configs (daily)
  rtp-fee-crank     ● SUCCESS   Fee sweep (1h)
  rtp-promote       ● SUCCESS   Strategy promotion (daily)
  rtp-swarm-ci      ● SUCCESS   Build + test + clippy
```

Terminal: `--void` background, `Geist Mono` 14px, `--text-secondary`. Status dots: `--emerald`. No border, no window chrome decoration.

Below terminal:
```
ZERO HUMAN INTERVENTION · SELF-FUNDED GAS · 24/7 AUTONOMOUS
```
`--text-muted`, 11px uppercase letterspaced.

**Narration guide:**
> "Three hundred twenty-five Rust tests. Zero failures. Seven Railway services — all green, all autonomous. The trader runs twenty-four-seven. The night shift runs daily. The fee crank sweeps hourly. Zero human intervention."

---

### SCENE 5 — Close (2:45–3:00) · 15s · HyperFrames

**Final recap, left-aligned, staggered line-by-line:**

```
3 layers      Python research · Rust swarm · Solana on-chain
6 wings       Trading · Security · Evolve · Knowledge · Audit · Futureproof
16 invariants Enforced in Rust AND on-chain
325 tests     0 failures
7 services    All green, all autonomous
1 function    registerWithRTP()
```

Numbers in `--text-primary` Geist 20px. Labels in `--text-tertiary` 14px.

**Identity:**
```
RESILIENT TOKEN PROTOCOL
resilientprotocol.xyz · github.com/tradewife/resilient-token-protocol
```

Fade to `--void`.

**Narration guide:**
> "Three layers. Six wings. Sixteen invariants. Three hundred twenty-five tests. Seven autonomous services. One function call. Resilient Token Protocol."

---

# Production Pipeline

### Asset Checklist

| Asset | Video | Scene | Tool |
|-------|-------|-------|------|
| Hook text composition | Pitch | 1 | HyperFrames |
| Solution flow composition | Pitch | 2 | HyperFrames |
| Dashboard + Explorer recording | Pitch | 3 | browser-harness |
| Differentiation list composition | Pitch | 4 | HyperFrames |
| Market data composition | Pitch | 5 | HyperFrames |
| Founder profile composition | Pitch | 6 | HyperFrames |
| CTA crescendo composition | Pitch | 7 | HyperFrames |
| Architecture stack composition | Tech | 1 | HyperFrames |
| Full website walkthrough recording | Tech | 2 | browser-harness |
| CPI code flow composition | Tech | 3 | HyperFrames |
| Terminal simulation composition | Tech | 4 | HyperFrames |
| Close recap composition | Tech | 5 | HyperFrames |

### Render Steps

```
1. Builder records all narration audio (separate tracks per scene)
2. browser-harness captures (dashboard walkthrough, Explorer, docs, launch page)
   → .webm → .mp4 via ffmpeg
3. Build HyperFrames HTML compositions
   → npx hyperframes lint → fix
4. npx hyperframes render each scene → .mp4 clips
5. ffmpeg concat:
   ffmpeg -f concat -safe 0 -i manifest.txt \
     -c:v libx264 -preset slow -crf 16 \
     -pix_fmt yuv420p -movflags +faststart output.mp4
6. Add narration + background music:
   ffmpeg -i output.mp4 -i narration.mp3 -i bg.mp3 \
     -filter_complex "[1:a]volume=1.0[vo];[2:a]volume=0.12[bg];[vo][bg]amix=inputs=2" \
     -c:v copy -c:a aac -shortest final.mp4
7. Review against Impeccable checklist → re-render if needed
```

### Browser-Harness Rules

```
INTERACTION: heightened human-like
- Mouse: sigmoid curves, never linear
- Hover: 500ms dwell on interactive elements
- Scroll: 120px / 300ms — reading rhythm
- Section dwell: 1.5s after load
- Nav clicks: 600ms pause, natural arc
- NO emphasis zoom injection — let the site's own design work
- Callouts injected via page.evaluate() only on Solana Explorer

CALLOUT STYLE:
  font: 12px Geist Mono;
  color: oklch(55% 0.1 160);
  background: oklch(8% 0.025 160 / 0.9);
  border: 1px solid oklch(25% 0.015 160);
  padding: 4px 8px;
  border-radius: 4px;
```

---

## Impeccable Audit Checklist

| Check | Status |
|-------|--------|
| No side-stripe borders (`border-left > 1px`) | ✓ Removed from Scene 4 differentiation |
| No gradient text | ✓ All text solid color |
| No particles / canvas noise / generative backgrounds | ✓ Void background only, flower image where needed |
| No glassmorphism / glow borders | ✓ None |
| No hero metric card grids | ✓ Market data is vertical list, not grid |
| No bounce/elastic easing | ✓ ease-out-quart/expo only |
| No purple-blue / cyan-on-dark | ✓ Emerald OKLCH + coral only |
| Colors match production CSS (OKLCH) | ✓ All values from globals.css |
| Font: Geist (user request) | ✓ |
| Type scale: fixed rem, not fluid | ✓ Per Impeccable rule for product UI |
| Neutrals tinted toward brand hue (160) | ✓ Matches globals.css |
| Technical overview is website-first | ✓ 85s of 180s is browser-harness |
| No decoration competing with content | ✓ Per .impeccable.md principle 2 |

> [!TIP]
> **Recording order:** Record technical overview first (show what you built, straightforward). Then pitch video — confidence from the technical walkthrough carries into delivery.
