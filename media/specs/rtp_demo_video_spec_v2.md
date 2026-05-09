# RTP Demo Video — Unified Production Spec v2
## Hermes + HyperFrames + ascii-video + Heightened Browser-Use

**Project:** Resilient Token Protocol (RTP)
**URL:** https://www.resilientprotocol.xyz
**Repo:** https://github.com/tradewife/resilient-token-protocol
**Target Duration:** 3 minutes (180s)
**Output:** 1920×1080 MP4, H.264, 24fps
**Deadline:** May 11 2026 — Colosseum / Canteen SWARM submission

---

## Philosophy

This is not a screen recording. It is an **audiovisual argument** for a live, running,
autonomous system. Every frame earns its place. The browser interaction is not a user
demo — it is an entity navigating its own infrastructure. The ASCII visual layer signals
machine intelligence. The HyperFrames composition layer gives it editorial structure.

**Three-layer output architecture:**
1. **Hermes ascii-video** — generative ASCII motion backgrounds (the "organism" layer)
2. **HyperFrames** — composition engine, GSAP animations, annotations, TTS narration,
   and final render (the "editorial" layer)
3. **Heightened browser-use** — Hermes drives a real visible Chrome session with
   human-like but intensified behaviour: deliberate mouse paths, purposeful pauses,
   zoom-on-hover emphasis, and annotated spotlight overlays injected via JS (the
   "proof" layer)

The demo never fakes. The trader is live on Railway. The mainnet TXs are real.
The 325 tests actually pass. The browser session shows the real site, the real Railway
dashboard, the real Solana Explorer. Everything can be verified by a judge in 10 seconds.

---

## Toolchain Setup

### 1. Install HyperFrames skills into Hermes
```bash
npx skills add heygen-com/hyperframes
# Registers: /hyperframes /hyperframes-cli /hyperframes-media /gsap /tailwind
```

### 2. Initialise the project
```bash
npx hyperframes init rtp-demo --tailwind
cd rtp-demo
# Project dirs: compositions/ screenshots/ assets/ audio/ recordings/ output/
```

### 3. Verify ascii-video skill is active in Hermes
```
/ascii-video
```

### 4. Install catalog blocks
```bash
npx hyperframes add flash-through-white
npx hyperframes add data-chart
npx hyperframes add instagram-follow   # repurposed as "LIVE" lower-third
```

### 5. Design tokens (inject as CSS vars into all compositions)
```css
:root {
  --bg:       #0a0f1a;   /* deep navy */
  --fg:       #e8edf5;   /* cool white */
  --green:    #14f195;   /* Solana green — live / success / yield */
  --purple:   #9945ff;   /* Solana purple — architecture / swarm */
  --rose:     #f43f5e;   /* danger / stop / rejection */
  --amber:    #f59e0b;   /* research / night shift */
  --mono:     "JetBrains Mono", "Fira Code", monospace;
}
```

---

## Heightened Browser-Use — Interaction Spec

This is the core upgrade from v1. "Heightened human-like" means:

**The cursor behaves like someone who understands what they are looking at.**

### Principles
- **Purposeful entry** — cursor enters from the left edge mid-screen, not from a corner.
  It arcs in a gentle sigmoid path toward its target. Never snaps.
- **Deliberate hover** — on every important element, pause 400–800ms before clicking.
  The hover triggers real browser tooltip/hover states. This reads as intentional.
- **Emphasis zoom** — after reaching a key metric (Calmar 44.89, +554%, 325 tests),
  inject a CSS transform zoom (scale 1.0→1.08, 300ms ease-out) on the element via JS.
  This is a spotlight, not a click.
- **Scroll rhythm** — scroll at 120px/300ms. Not a flick. Not a crawl. The rhythm of
  someone reading. Pause 1.5s after each major section appears.
- **Tab sequencing** — when showing multiple browser tabs (dashboard → Railway → Explorer),
  pause 600ms on the tab bar before clicking the next tab. The viewer's eye follows.
- **Annotation injection** — Hermes injects a custom overlay div into the live page DOM
  via JS (via `evaluate` / CDP) to add callout arrows, highlight rings, and data labels.
  These are part of the recorded session, not post-production overlays.

### Hermes Browser-Use Prompt Template (use for every browser scene)
```
Use gstack (persistent Chromium) in connect mode (visible Chrome).

INTERACTION STYLE: heightened human-like
- Mouse paths: sigmoid curves, never linear snaps
- Hover dwell: 500ms on key elements before action
- Scroll speed: 120px per 300ms — reading rhythm, not flick
- After each section loads: 1.5s dwell before next action
- On key metrics: inject emphasis zoom via JS (scale 1.08, 300ms ease-out)
- Annotation injection: use page.evaluate() to inject .rtp-callout divs with
  position:fixed overlays, green border rings, and label text
- Tab switches: pause 600ms on tab bar, arc cursor to target tab

CALLOUT DIV SPEC:
  style: border: 2px solid #14f195; border-radius: 6px; padding: 4px 8px;
         font: 12px JetBrains Mono; color: #14f195; background: rgba(0,0,0,0.7);
         position: fixed; z-index: 99999; pointer-events: none;

Record the full session as a .webm. Do not cut. Output: recordings/scene-N.webm
Convert to MP4: ffmpeg -i recordings/scene-N.webm -c:v libx264 -crf 16 recordings/scene-N.mp4
```

---

## Scene Breakdown

### SCENE 1 — Cold Open (0:00–0:08) | 8 seconds | HyperFrames + ascii-video

**Narration:** None. Pure visual.
**Goal:** First 3 seconds feel like alien intelligence activating. No title yet.

#### ascii-video background — `ascii_cold_open.mp4`
```
Mode: generative
Duration: 9s (1s trim buffer)
Grid layer 1 (sm, 10px): domain-warp fBM, katakana palette, 100% opacity
Grid layer 2 (md, 16px): vortex field, alchemical symbols, screen blend 60%
Grid layer 3 (xl, 24px): rings (concentric, bass-driven), block elements, overlay blend 30%
Color strategy: angle-mapped rainbow → hard cut to monochrome teal at t=6s
Shaders: glitch art mood (heavy chromatic aberration + glitch bands + color wobble)
         then snap to clean modern (bloom + vignette) at t=6s — matches title reveal
FeedbackBuffer: zoom in, decay 0.92, hue shift +3°/frame
Directional arc: glitch intensity 1.0 → 0.0 over t=0 to t=6 (organism "settling")
Profile: production (1080p 24fps)
Output: assets/ascii_cold_open.mp4
```

#### HyperFrames composition
```html
<div id="stage" data-composition-id="cold-open" data-start="0"
     data-width="1920" data-height="1080">
  <video id="bg-cold" data-start="0" data-duration="8" data-track-index="0"
         src="../assets/ascii_cold_open.mp4" muted playsinline></video>

  <!-- flash-through-white shader transition INTO scene 2 at t=8 -->
  <div id="flash-out" class="clip shader-flash"
       data-start="7.7" data-duration="0.3" data-track-index="5"></div>
</div>
```

---

### SCENE 2 — Title Card (0:08–0:18) | 10 seconds | HyperFrames

**Narration (Kokoro TTS):** "Resilient Token Protocol. Autonomous treasury infrastructure
for every token on Solana."

#### ascii-video background — `ascii_title_bg.mp4`
```
Mode: generative
Duration: 11s
Value field: fBM noise (slow, 2 octaves) — almost static, just breathing
Palette: density ramp ` .:-=+#@█`
Color: monochrome teal, 25% opacity (background breathes, never competes)
Shaders: clean modern (bloom 10% + vignette)
No FeedbackBuffer
Profile: production
Output: assets/ascii_title_bg.mp4
```

#### HyperFrames composition
```html
<video id="bg-title" data-start="0" data-duration="10" data-track-index="0"
       src="../assets/ascii_title_bg.mp4" muted playsinline></video>

<div id="title-main" class="clip" data-start="0.4" data-duration="9.6" data-track-index="2"
     style="font: 900 96px/1 var(--mono); color: var(--fg); text-align:center;
            position:absolute; top:38%; width:100%; letter-spacing:-2px;">
  RESILIENT TOKEN PROTOCOL
</div>

<div id="subtitle" class="clip" data-start="1.2" data-duration="8.8" data-track-index="2"
     style="font: 400 22px var(--mono); color:#8899aa; text-align:center;
            position:absolute; top:53%; width:100%; letter-spacing:4px; text-transform:uppercase;">
  Autonomous Treasury Infrastructure · Solana
</div>

<!-- three stat pills stagger in at t=2.5 -->
<div id="pill-1" class="clip stat-pill" data-start="2.5" data-duration="7" data-track-index="3"
     style="left:28%">Flash Trade CPI</div>
<div id="pill-2" class="clip stat-pill" data-start="2.9" data-duration="6.6" data-track-index="3"
     style="left:44.5%">6-Wing Swarm</div>
<div id="pill-3" class="clip stat-pill" data-start="3.3" data-duration="6.2" data-track-index="3"
     style="left:58%">30K Configs/Night</div>

<!-- ghost RTP watermark -->
<div id="ghost-rtp" data-start="0" data-duration="10" data-track-index="1"
     style="font: 900 380px var(--mono); color:rgba(20,241,149,0.04);
            position:absolute; top:15%; left:50%; transform:translateX(-50%)">
  RTP
</div>
```

#### GSAP
```javascript
gsap.from("#title-main", { opacity:0, y:40, duration:0.7, ease:"expo.out" })
gsap.from("#subtitle",   { opacity:0, y:20, duration:0.5, ease:"power3.out", delay:1.2 })
gsap.from(".stat-pill",  { opacity:0, scale:0.85, duration:0.35, stagger:0.4,
                           ease:"back.out(1.6)", delay:2.5 })
gsap.to("#stage",        { opacity:0, duration:0.3, delay:9.7 })  // crossfade out
```

#### TTS audio
```
/hyperframes-media
Text: "Resilient Token Protocol. Autonomous treasury infrastructure for every token on Solana."
Voice: Kokoro — af_sarah, speed 0.95, neutral-warm
Output: audio/vo_s2_title.wav
```

---

### SCENE 3 — Live Dashboard (0:18–0:38) | 20 seconds | Browser-use

**Narration:** "Token projects route trading fees to RTP. The swarm generates yield via
on-chain perpetuals. Yield flows back to holders. The trader is live right now — no human
keypair exists."

#### Hermes browser-use prompt
```
gstack — connect mode (real visible Chrome, 1920×1080)

Navigate to https://www.resilientprotocol.xyz

SEQUENCE:
1. Entry: arc cursor from left edge to hero section (sigmoid path, 1.2s travel)
2. Dwell on hero headline 800ms
3. Inject callout ring around "Every token gets a program-enforced treasury":
   page.evaluate: inject .rtp-callout at bounding box of that element (green ring + label)
4. Scroll down at reading rhythm (120px/300ms) to "Live Trading" section
   Pause 1.5s when LIVE green dot comes into viewport
5. Inject emphasis zoom on LIVE dot: scale(1.0)→scale(1.12) 300ms ease-out, hold 1s, back
6. Inject callout labels on:
   - SOL price field: "Real-time mainnet price"
   - Signal score: "Multi-timeframe confluence score"
   - Calmar ratio: "44.89 — risk-adjusted return"
7. Scroll to "+554%" metric. Inject emphasis zoom.
   Pause 2s — this is the hero number.
8. Inject callout: "+554% compounded · 9x leverage · 9/9 folds profitable · 0 liquidations"
9. Scroll to "No Human Keypair" section (or equivalent). Dwell 1.5s.
10. prettyscreenshot: capture full viewport with injected overlays
    Output: screenshots/s3_dashboard_hero.png

Record full session: recordings/scene3_dashboard.webm → .mp4
```

#### HyperFrames overlay (applied over screen recording)
```html
<video id="browser-s3" data-start="0" data-duration="20" data-track-index="0"
       src="../recordings/scene3_dashboard.mp4" muted playsinline></video>

<!-- lower third LIVE indicator (HyperFrames catalog: instagram-follow repurposed) -->
<div id="lower-live" class="clip lower-third" data-start="8" data-duration="12"
     data-track-index="2"
     style="bottom:60px; left:40px; font: 600 14px var(--mono); color:var(--green);
            background:rgba(0,0,0,0.75); padding:6px 14px; border-radius:4px;
            border-left: 3px solid var(--green);">
  ● LIVE · Railway · SOL/USDT Survivor 2.69
</div>
```

---

### SCENE 4 — Mainnet Transaction Proof (0:38–0:58) | 20 seconds | Browser-use

**Narration:** "Real mainnet transactions. Not testnet. Not simulation. The Treasury PDA
signs via invoke_signed — no private key exists for trading."

**The hardest thing to fake. Show it full-screen, uncut.**

#### Hermes browser-use prompt
```
gstack — connect mode

TAB 1 — Open position TX:
1. goto https://explorer.solana.com/tx/2bLg1FuJkGd5hGBwEe35hwMvHtYv6dMJHxQjRMFa8PkHSwSCSQfeNbdgGwnFx8eGbMbVZhjjkMZiT3vDjvmQRQM
2. Wait for full load (2s)
3. Arc cursor slowly to transaction header — dwell 600ms
4. Inject emphasis zoom on "Success" status badge
5. Scroll to Program Logs section — reading rhythm 120px/300ms
6. Pause 2s on "invoke_signed" line in logs — inject callout:
   "Treasury PDA signs here. No private key."
7. Inject emphasis zoom on compute unit consumption: "99,214 CU consumed"
   Inject callout: "On-chain Flash Trade CPI execution"
8. prettyscreenshot: capture with overlays → screenshots/s4_open_tx.png

TAB 2 — Close position TX (arc to new tab, pause 600ms on tab bar):
1. goto https://explorer.solana.com/tx/56PLUQAYQhYhpXmG8mN3s8WFXVSatdEPT2jHJmhRSR4SxKKXQxjyRBJ6Z5NqDVbGHCw3VEy3VTbKF2v8zGd4aXh
2. Wait for load (2s)
3. Inject callout on SOL returned to treasury vault
4. prettyscreenshot → screenshots/s4_close_tx.png

Record both tabs: recordings/scene4_explorer.mp4
```

#### HyperFrames overlay
```html
<video id="browser-s4" data-start="0" data-duration="20" data-track-index="0"
       src="../recordings/scene4_explorer.mp4" muted playsinline></video>

<div id="proof-label" class="clip" data-start="1" data-duration="18" data-track-index="2"
     style="top:20px; right:30px; font:500 13px var(--mono); color:var(--amber);
            background:rgba(0,0,0,0.8); padding:5px 12px; border-radius:4px;
            border: 1px solid var(--amber);">
  Solana Mainnet · Verifiable
</div>
```

---

### SCENE 5 — invoke_signed Architecture (0:58–1:18) | 20 seconds | HyperFrames

**Narration:** "The Treasury PDA owns all assets. No private key. The Anchor program is
the only authority. Here's the exact CPI flow: Treasury PDA seeds pass to invoke_signed,
Flash Trade opens the position on-chain, SOL yield returns to the vault."

**Animated code walkthrough — pure HyperFrames composition.**

#### ascii-video background — `ascii_code_bg.mp4`
```
Mode: generative
Duration: 21s
Value fields: reaction-diffusion (coral preset) + box-drawing palette (circuit-like)
Color: monochrome teal 15% opacity — barely visible, structural
Shaders: clean modern — vignette only
No FeedbackBuffer
Profile: production
Output: assets/ascii_code_bg.mp4
```

#### HyperFrames composition
```html
<video id="bg-code" data-start="0" data-duration="20" data-track-index="0"
       src="../assets/ascii_code_bg.mp4" muted playsinline></video>

<!-- Code flow diagram — revealed with GSAP stagger -->
<div id="code-title" class="clip" data-start="0.5" data-duration="19"
     data-track-index="2"
     style="top:60px; left:50%; transform:translateX(-50%);
            font:700 18px var(--mono); color:var(--green); text-align:center;
            letter-spacing:3px; text-transform:uppercase;">
  CPI Flow · Treasury PDA → Flash Trade
</div>

<!-- Three code block nodes animate in -->
<div id="node-bridge" class="clip code-node" data-start="1" data-duration="18.5"
     data-track-index="3" style="top:160px; left:120px; width:340px">
  <div class="node-label" style="color:var(--purple)">bridge.rs</div>
  <pre>execute_cpi(config, seeds)</pre>
</div>
<div id="node-chain" class="clip code-node" data-start="1.8" data-duration="17.7"
     data-track-index="3" style="top:160px; left:780px; width:360px">
  <div class="node-label" style="color:var(--purple)">chain_client.rs</div>
  <pre>build_open_instruction(
  treasury_pda, seeds)</pre>
</div>
<div id="node-lib" class="clip code-node" data-start="2.6" data-duration="16.9"
     data-track-index="3" style="top:160px; left:1440px; width:340px">
  <div class="node-label" style="color:var(--green)">lib.rs handler</div>
  <pre>invoke_signed(
  &ix, accounts, seeds)</pre>
</div>

<!-- Animated arrow SVG flows between nodes -->
<svg id="flow-arrows" class="clip" data-start="3.2" data-duration="16.5"
     data-track-index="4" viewBox="0 0 1920 200"
     style="position:absolute; top:180px; left:0; width:100%">
  <path id="arrow1" d="M460,60 C620,60 680,60 780,60"
        stroke="#9945ff" stroke-width="2" fill="none" marker-end="url(#arrowhead)"/>
  <path id="arrow2" d="M1140,60 C1300,60 1360,60 1440,60"
        stroke="#14f195" stroke-width="2" fill="none" marker-end="url(#arrowhead)"/>
</svg>

<!-- Result banner -->
<div id="result-banner" class="clip" data-start="12" data-duration="8"
     data-track-index="4"
     style="bottom:120px; left:50%; transform:translateX(-50%);
            font:700 16px var(--mono); color:var(--green);
            background:rgba(20,241,149,0.1); border:1px solid var(--green);
            padding:10px 28px; border-radius:6px; text-align:center;">
  99,214 CU consumed · Confirmed mainnet · invoke_signed · No human key
</div>
```

#### GSAP
```javascript
gsap.from(".code-node",     { opacity:0, y:30, duration:0.5, stagger:0.8, ease:"power3.out" })
gsap.from("#arrow1",        { drawSVG:"0%", duration:0.6, delay:3.2 })
gsap.from("#arrow2",        { drawSVG:"0%", duration:0.6, delay:4.2 })
gsap.from("#result-banner", { opacity:0, scale:0.9, duration:0.5, ease:"back.out(1.4)", delay:12 })
// Glow pulse on invoke_signed text
gsap.to("#node-lib pre",    { textShadow:"0 0 12px #14f195", duration:0.4,
                              repeat:-1, yoyo:true, delay:8 })
```

---

### SCENE 6 — Research Engine Night Shift (1:18–1:38) | 20 seconds | HyperFrames + data-chart

**Narration:** "Every night, the research engine evaluates thirty thousand strategy
configurations. Nine-fold walk-forward validation. Only survivors across all nine
independent time windows get deployed."

#### ascii-video background — `ascii_research_bg.mp4`
```
Mode: generative
Duration: 21s
Value fields: strange-attractors (Clifford), temporal noise morph
Palette: math symbols (` ·∘∙•°±×÷≈≠≡∞∫∑Ω`)
Color: monochrome amber (research/night shift tone)
Shaders: cinematic mood (bloom + vignette + grain)
Profile: production
Output: assets/ascii_research_bg.mp4
```

#### HyperFrames composition
```html
<video id="bg-research" data-start="0" data-duration="20" data-track-index="0"
       src="../assets/ascii_research_bg.mp4" muted playsinline></video>

<!-- Animated pipeline counter -->
<div id="pipeline-title" class="clip" data-start="0.5" data-duration="19"
     data-track-index="2"
     style="top:60px; left:50%; transform:translateX(-50%);
            font:700 18px var(--mono); color:var(--amber); letter-spacing:3px">
  NIGHT SHIFT · DARWINIAN RESEARCH ENGINE
</div>

<!-- Counter blocks stagger in with animated number reveals -->
<div id="count-1" class="clip counter-block" data-start="1" data-duration="18.5"
     data-track-index="3" style="left:15%">
  <span class="count-num" id="n1">0</span>
  <span class="count-label">parameter configs</span>
</div>
<div id="count-2" class="clip counter-block" data-start="2" data-duration="17.5"
     data-track-index="3" style="left:38%">
  <span class="count-num" id="n2">0</span>
  <span class="count-label">WFA folds</span>
</div>
<div id="count-3" class="clip counter-block" data-start="3" data-duration="16.5"
     data-track-index="3" style="left:60%">
  <span class="count-num" id="n3">0</span>
  <span class="count-label">Calmar ratio</span>
</div>
<div id="count-4" class="clip counter-block" data-start="4" data-duration="15.5"
     data-track-index="3" style="left:79%">
  <span class="count-num" id="n4">0</span><span style="font-size:0.6em">%</span>
  <span class="count-label">consistency</span>
</div>

<!-- Animated bar chart race (hyperframes add data-chart) -->
<div id="chart-area" class="clip" data-start="8" data-duration="12"
     data-track-index="4" style="bottom:100px; left:10%; width:80%; height:250px">
  <!-- data-chart block with SOL/BNB/ETH/BTC PnL bars animated in -->
</div>
```

#### GSAP counter animations
```javascript
// Animated number rollups
gsap.to("#n1", { innerHTML: "30,720", duration:2.5, delay:1.2,
                 snap: { innerHTML: 1 }, ease:"power2.out" })
gsap.to("#n2", { innerHTML: "9", duration:1, delay:2.2, ease:"power2.out" })
gsap.to("#n3", { innerHTML: "44.89", duration:1.5, delay:3.2, ease:"power2.out" })
gsap.to("#n4", { innerHTML: "100", duration:1.5, delay:4.2, ease:"power2.out" })
// Counter glow on complete
gsap.to("#n4", { color:"#14f195", textShadow:"0 0 16px #14f195",
                 duration:0.4, delay:5.7 })
```

---

### SCENE 7 — Three-Layer Architecture (1:38–1:53) | 15 seconds | HyperFrames

**Narration:** "Three layers. Python research brain. Rust swarm runtime with six wings.
Solana on-chain treasury. The Evolve Wing proposes mutations within soulcontract bounds.
The Audit Wing enforces them."

#### ascii-video background — `ascii_arch_bg.mp4`
```
Mode: generative
Duration: 16s
Value fields: voronoi cells (slowly multiplying) + reaction-diffusion (worms preset)
Palette: box-drawing + braille (structural, technical)
Color: teal/purple split — teal bottom third (Solana), purple middle (Rust), white top (Python)
Shaders: clean modern — bloom only
Profile: production
Output: assets/ascii_arch_bg.mp4
```

#### HyperFrames composition
Three animated layer blocks slide up from off-screen, staggered 0.6s each:
```html
<div id="layer-solana" class="clip arch-layer" data-start="1" data-duration="14"
     data-track-index="2" style="bottom:80px; border-color:var(--green)">
  <div class="layer-title" style="color:var(--green)">SOLANA ON-CHAIN</div>
  <div class="layer-items">Treasury PDA · Flash Trade CPI · Phase Evolution · invoke_signed</div>
</div>
<div id="layer-rust" class="clip arch-layer" data-start="1.6" data-duration="13.4"
     data-track-index="2" style="bottom:240px; border-color:var(--purple)">
  <div class="layer-title" style="color:var(--purple)">RUST SWARM · 6 WINGS</div>
  <div class="layer-items">Trading · Security · Evolve · Knowledge · Audit · Future-proof</div>
</div>
<div id="layer-python" class="clip arch-layer" data-start="2.2" data-duration="12.8"
     data-track-index="2" style="bottom:400px; border-color:var(--fg)">
  <div class="layer-title" style="color:var(--fg)">PYTHON RESEARCH</div>
  <div class="layer-items">Night Shift · 30K Configs · WFA · Darwinian · Full Sim</div>
</div>

<!-- Animated data flow arrows between layers -->
<!-- Callout: soulguard enforces on every message -->
<div id="soul-callout" class="clip" data-start="7" data-duration="8"
     data-track-index="3"
     style="right:100px; top:50%; font:500 13px var(--mono);
            color:var(--amber); border:1px solid var(--amber);
            padding:8px 16px; background:rgba(0,0,0,0.8)">
  soulguard.rs enforces on every message ↓
</div>
```

---

### SCENE 8 — 325 Tests + CLI (1:53–2:18) | 25 seconds | Browser-use (terminal)

**Narration:** "Three hundred and twenty-five Rust tests. Zero failures. The CLI deploys
a treasury with one command."

#### Hermes browser-use prompt
```
Navigate to a web-based terminal showing the project root
OR use gstack to drive a local terminal via a browser terminal (ttyd / wetty)
OR compose as a styled HTML HyperFrames terminal simulation if live terminal unavailable.

PREFERRED — live terminal via browser:
1. Open ttyd or wetty in Chrome via gstack
2. Arc cursor slowly into terminal area
3. Type commands with human-like keystroke rhythm:
   - 90ms average inter-key delay
   - occasional 200ms pause mid-word (natural hesitation)
   - 500ms pause before hitting Enter

COMMAND SEQUENCE:
  Cmd 1: cd rtp/swarm && cargo test --lib 2>&1 | tail -5
          [pause 3s for output]
          [inject emphasis zoom on "325 passed; 0 failed"]
          [inject callout: "All 6 wings. 9/9 Flash Trade CPI tests. 0 clippy warnings."]

  Cmd 2: cd ../.. && npx tsx cli/bin/rtp.ts status --all
          [pause 2.5s for output]
          [inject callout on LIVE status lines]

  Cmd 3: npx tsx cli/bin/rtp.ts accounts derive --mint So11111111111111111111111111111112
          [pause 1.5s for PDA output]
          [inject callout: "Treasury PDA derived offline — no RPC needed"]

FALLBACK — HyperFrames animated terminal:
  If live terminal unavailable, use typewriter-animation HTML with
  monospace styling and terminal colour scheme. Same commands, same timing.
  This is the reliable fallback. The Nous Research video used this approach.

Record: recordings/scene8_terminal.mp4
```

#### HyperFrames overlay
```html
<video id="browser-s8" data-start="0" data-duration="25" data-track-index="0"
       src="../recordings/scene8_terminal.mp4" muted playsinline></video>

<div id="test-badge" class="clip" data-start="5" data-duration="20"
     data-track-index="2"
     style="top:20px; right:30px; font:700 13px var(--mono); color:var(--green);
            background:rgba(20,241,149,0.12); border:1px solid var(--green);
            padding:6px 14px; border-radius:4px;">
  325 tests · 0 failures · 0 clippy warnings
</div>
```

---

### SCENE 9 — Railway Infrastructure (2:18–2:33) | 15 seconds | Browser-use

**Narration:** "Seven services, all green, all autonomous. The trader polls every five
minutes. No human intervention. Self-funded gas."

#### Hermes browser-use prompt
```
gstack — connect mode

Navigate to Railway dashboard (authenticated session, or use railway CLI output
piped through a web terminal)

If direct Railway dashboard accessible:
1. goto https://railway.app/project/[your-project-id]
2. Reading-rhythm scroll through service list
3. Inject emphasis zoom on each green status dot (hover sequence: 400ms each)
4. Inject callout labels:
   - rtp-trader: "Always-on · 5min poll · Flash Trade · SOL/USDT"
   - rtp-night-shift: "Nightly cron · 30K configs"
   - rtp-fee-crank: "Hourly fee sweep"
5. Pause 2s on the full service grid

FALLBACK — compose terminal output as styled HTML:
  Output of `railway status` or equivalent, styled with service status indicators.
  Green dots, service names, uptime. HyperFrames typewriter animation.

prettyscreenshot → screenshots/s9_railway.png
Record: recordings/scene9_railway.mp4
```

#### HyperFrames overlay
```html
<video id="browser-s9" data-start="0" data-duration="15" data-track-index="0"
       src="../recordings/scene9_railway.mp4" muted playsinline></video>

<div id="autonomy-label" class="clip" data-start="3" data-duration="12"
     data-track-index="2"
     style="bottom:50px; left:50%; transform:translateX(-50%);
            font:500 12px var(--mono); color:#8899aa; letter-spacing:2px">
  ZERO HUMAN INTERVENTION · SELF-FUNDED GAS
</div>
```

---

### SCENE 10 — Crescendo Outro (2:33–3:00) | 27 seconds | HyperFrames + ascii-video

**Narration:** "Self-funding treasury. No RTP token. Pure infrastructure. Any token
project integrates with one function call. The swarm runs. Improves. Defends. Evolves.
Forever."

**This is the 7-layer ascii crescendo. Earned by everything before it.**

#### ascii-video background — `ascii_crescendo.mp4`
```
Mode: generative — 7-layer crescendo finale
Duration: 28s

Layer 1 (bg, sm grid, 10px, 100%): fBM noise, teal monochrome, slow drift
Layer 2 (md grid, 16px, 30%, screen): reaction-diffusion coral preset, teal
Layer 3 (sm grid, 10px, 20%, overlay): cellular automata Game of Life, analog fade trails
Layer 4 (lg grid, 20px, 25%, screen): dual counter-rotating spirals, teal+purple split hue
Layer 5 (md grid, 16px, 20%, overlay): Voronoi cells — start sparse, multiply to max density at t=20s
Layer 6 (sm grid, 10px, 15%, screen): boid flocking (200 agents), data character set
Layer 7 (xxl grid, 40px, 45%, normal): "RTP" as stencil text mask, domain-warp reveals over t=8-18s
         Characters: project-specific palette ` .·~=≈∞⚡☿✦★⊕◊◆▲▼●■`

Color strategy: triadic teal + purple + white, OKLCH perceptually uniform interpolation
Shader chain: bloom (0.3→1.0 over duration) + chromatic aberration (beat-reactive at t=20)
              + kaleidoscope (activate at t=22, 4-fold) + mirror quad (activate at t=25)
FeedbackBuffer: rotate CW 0.3°/frame, decay 0.96, hue shift +1°/frame
Directional param arc: bloom intensity 0.2→1.0, Voronoi cell count 5→80, all directed arcs

Particle burst at t=22: explosion system, 300 particles, energy character set, radial burst
Profile: production (1080p 24fps)
Output: assets/ascii_crescendo.mp4
```

#### HyperFrames composition
```html
<video id="bg-crescendo" data-start="0" data-duration="27" data-track-index="0"
       src="../assets/ascii_crescendo.mp4" muted playsinline></video>

<!-- Fade-in mission statements, one at a time -->
<div id="stmt-1" class="clip mission-stmt" data-start="1" data-duration="4"
     data-track-index="2">Self-funding treasury.</div>
<div id="stmt-2" class="clip mission-stmt" data-start="5.5" data-duration="4"
     data-track-index="2">No RTP token. Pure infrastructure.</div>
<div id="stmt-3" class="clip mission-stmt" data-start="10.5" data-duration="5"
     data-track-index="2">Any token project. One function call.</div>

<!-- Code snippet: registerWithRTP() -->
<div id="code-cta" class="clip" data-start="15" data-duration="7" data-track-index="3"
     style="font:600 22px var(--mono); color:var(--green); text-align:center;
            position:absolute; top:42%; width:100%;">
  registerWithRTP(connection, wallet, { authority: publicKey });
</div>

<!-- Final identity block -->
<div id="final-title" class="clip" data-start="22.5" data-duration="4.5"
     data-track-index="4"
     style="font:900 64px var(--mono); color:var(--fg); text-align:center;
            position:absolute; top:34%; width:100%; letter-spacing:-1px;">
  RESILIENT TOKEN PROTOCOL
</div>
<div id="final-url" class="clip" data-start="23.5" data-duration="3.5"
     data-track-index="4"
     style="font:400 18px var(--mono); color:var(--green); text-align:center;
            position:absolute; top:52%; width:100%; letter-spacing:2px;">
  resilientprotocol.xyz · github.com/tradewife/resilient-token-protocol
</div>

<!-- Fade to black -->
<div id="fade-out" class="clip" data-start="26.5" data-duration="0.5"
     data-track-index="5"
     style="background:var(--bg); width:100%; height:100%;
            position:absolute; top:0; left:0;"></div>
```

#### GSAP
```javascript
const missionStmts = [".mission-stmt"]
gsap.from(missionStmts, { opacity:0, y:25, duration:0.6,
                          stagger: { amount: 4.5 }, ease:"power3.out" })
gsap.to(missionStmts,   { opacity:0, duration:0.4, stagger:{ amount:4.5 }, delay:3 })
gsap.from("#code-cta",  { opacity:0, scale:0.95, duration:0.5, ease:"back.out(1.3)", delay:15 })
gsap.from("#final-title",{ opacity:0, y:30, duration:0.8, ease:"expo.out", delay:22.5 })
gsap.from("#final-url", { opacity:0, duration:0.5, delay:23.5 })
gsap.to("#fade-out",    { opacity:1, duration:0.5, delay:26.5 })
```

---

## Audio Layer (Full Composition)

```html
<!-- Background music — 15% vol, ducked to 7% under voiceover -->
<audio id="bg-music" data-start="0" data-duration="180" data-track-index="10"
       data-volume="0.15" src="../audio/bg_music.wav"></audio>

<!-- Per-scene TTS voiceover clips -->
<audio id="vo-s2"  data-start="8"    data-duration="9"  data-track-index="11" data-volume="1.0" src="../audio/vo_s2_title.wav"></audio>
<audio id="vo-s3"  data-start="18"   data-duration="18" data-track-index="11" data-volume="1.0" src="../audio/vo_s3_dashboard.wav"></audio>
<audio id="vo-s4"  data-start="38"   data-duration="18" data-track-index="11" data-volume="1.0" src="../audio/vo_s4_mainnet.wav"></audio>
<audio id="vo-s5"  data-start="58"   data-duration="18" data-track-index="11" data-volume="1.0" src="../audio/vo_s5_invoke.wav"></audio>
<audio id="vo-s6"  data-start="78"   data-duration="18" data-track-index="11" data-volume="1.0" src="../audio/vo_s6_research.wav"></audio>
<audio id="vo-s7"  data-start="98"   data-duration="13" data-track-index="11" data-volume="1.0" src="../audio/vo_s7_arch.wav"></audio>
<audio id="vo-s8"  data-start="113"  data-duration="22" data-track-index="11" data-volume="1.0" src="../audio/vo_s8_tests.wav"></audio>
<audio id="vo-s9"  data-start="138"  data-duration="13" data-track-index="11" data-volume="1.0" src="../audio/vo_s9_railway.wav"></audio>
<audio id="vo-s10" data-start="153"  data-duration="24" data-track-index="11" data-volume="1.0" src="../audio/vo_s10_outro.wav"></audio>
```

#### TTS generation (all scenes in one Hermes call)
```
/hyperframes-media

Generate TTS voiceover files using Kokoro local synthesis.
Voice: af_sarah, speed 0.92, neutral-warm professional tone.
Slight emphasis on numbers and protocol-specific terms.

vo_s2_title.wav (9s):
"Resilient Token Protocol. Autonomous treasury infrastructure for every token on Solana."

vo_s3_dashboard.wav (18s):
"Token projects route trading fees to RTP. The swarm generates yield via on-chain
perpetuals. Yield flows back to holders. The trader is live right now — no human keypair exists."

vo_s4_mainnet.wav (18s):
"Real mainnet transactions. Not testnet. Not simulation. The Treasury PDA signs via
invoke_signed — no private key exists for trading."

vo_s5_invoke.wav (18s):
"The Treasury PDA owns all assets. No private key. The Anchor program is the only authority.
Treasury PDA seeds pass to invoke_signed, Flash Trade opens the position on-chain,
SOL yield returns to the vault."

vo_s6_research.wav (18s):
"Every night, the research engine evaluates thirty thousand strategy configurations.
Nine-fold walk-forward validation. Only survivors across all nine independent time windows get deployed."

vo_s7_arch.wav (13s):
"Three layers. Python research brain. Rust swarm runtime with six wings. Solana on-chain
treasury. The soulguard enforces the constitutional layer on every message."

vo_s8_tests.wav (22s):
"Three hundred and twenty-five Rust tests. Zero failures. The CLI deploys a treasury with
one command. Let's see it."

vo_s9_railway.wav (13s):
"Seven services, all green, all autonomous. The trader polls every five minutes.
No human intervention. Self-funded gas."

vo_s10_outro.wav (24s):
"Self-funding treasury. No RTP token. Pure infrastructure. Any token project integrates
with one function call. The swarm runs. Improves. Defends. Evolves. Forever."

Output all files to: audio/
```

---

## Asset Checklist

| Asset | Scene | Tool | Status |
|-------|-------|------|--------|
| `ascii_cold_open.mp4` | 1 | Hermes ascii-video | [ ] |
| `ascii_title_bg.mp4` | 2 | Hermes ascii-video | [ ] |
| `ascii_code_bg.mp4` | 5 | Hermes ascii-video | [ ] |
| `ascii_research_bg.mp4` | 6 | Hermes ascii-video | [ ] |
| `ascii_arch_bg.mp4` | 7 | Hermes ascii-video | [ ] |
| `ascii_crescendo.mp4` | 10 | Hermes ascii-video | [ ] |
| `scene3_dashboard.mp4` | 3 | Hermes browser-use (gstack) | [ ] |
| `scene4_explorer.mp4` | 4 | Hermes browser-use (gstack) | [ ] |
| `scene8_terminal.mp4` | 8 | Hermes browser-use / HF terminal | [ ] |
| `scene9_railway.mp4` | 9 | Hermes browser-use (gstack) | [ ] |
| `vo_s2_title.wav` | 2 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s3_dashboard.wav` | 3 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s4_mainnet.wav` | 4 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s5_invoke.wav` | 5 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s6_research.wav` | 6 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s7_arch.wav` | 7 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s8_tests.wav` | 8 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s9_railway.wav` | 9 | HyperFrames-media (Kokoro) | [ ] |
| `vo_s10_outro.wav` | 10 | HyperFrames-media (Kokoro) | [ ] |
| `bg_music.wav` | All | Royalty-free (your choice) | [ ] |
| `demo_final.mp4` | — | HyperFrames render + FFmpeg | [ ] |

---

## Render Pipeline

```
Step 1:  /hyperframes-media → generate all 9 TTS wav files
         (run first — no dependencies, longest tail)

Step 2:  /ascii-video → generate all 6 ascii background mp4s
         (spawn 6 parallel subagents — they are independent)

Step 3:  Hermes browser-use → record scenes 3, 4, 8, 9
         (scenes 3+4 can run in parallel; 8+9 can run in parallel)

Step 4:  Build all HyperFrames HTML compositions (10 files)
         Each composition references its assets from ../assets/ and ../recordings/

Step 5:  npx hyperframes lint compositions/*.html
         Fix any attribute errors before committing to render

Step 6:  npx hyperframes render compositions/scene-N.html --output output/scene-N.mp4
         Render all 10 scenes (can parallelise with --workers flag)

Step 7:  FFmpeg concat:
         ffmpeg -f concat -safe 0 -i manifest.txt \
                -c:v libx264 -preset slow -crf 16 \
                -pix_fmt yuv420p -movflags +faststart \
                output/rtp-demo-final.mp4
```

### Master Hermes prompt to kick off full pipeline
```
/hyperframes /hyperframes-cli /hyperframes-media /ascii-video /gsap

PROJECT: Resilient Token Protocol
URL: https://www.resilientprotocol.xyz
REPO: https://github.com/tradewife/resilient-token-protocol

TASK: Produce the full 3-minute demo video following the attached spec.

EXECUTION:
Spawn parallel subagents for:
  A: /hyperframes-media — all 9 TTS voiceover files
  B: /ascii-video — all 6 ascii background videos (6 sub-subagents inside B)
  C: browser-use — record scenes 3 and 4 (gstack, connect mode, heightened human-like)
  D: browser-use — record scenes 8 and 9

Report back when all assets are complete. Then:
  E: Build all 10 HyperFrames compositions
  F: npx hyperframes lint → fix → render all scenes
  G: FFmpeg concat → rtp-demo-final.mp4

Output: output/rtp-demo-final.mp4
```

---

## Output Specs
- **Duration:** ~180 seconds (3 minutes)
- **Resolution:** 1920×1080
- **Format:** MP4 (H.264, CRF 16)
- **Audio:** AAC stereo, 48kHz
- **Expected file size:** 80–120MB

---

## What Makes This Demo Different from v1

| Issue in v1 | Fix in v2 |
|-------------|-----------|
| Browser-use felt mechanical | Heightened human-like: sigmoid cursor paths, 500ms hover dwells, emphasis zoom injection, reading-rhythm scroll |
| No ASCII visual layer | 6 ascii-video clips provide the organism/intelligence aesthetic throughout |
| Annotations were post-production overlays | Callout divs injected into live DOM via page.evaluate() — part of the recorded session |
| gstack role was unclear | Explicit: connect mode, prettyscreenshot for hero frames, webm→mp4 pipeline |
| Scenes were loosely connected | Hard visual rhythm: glitch open → breathing title → browser proof → code diagram → data → architecture → tests → infrastructure → crescendo CTA |
| No parallel asset generation | Master prompt spawns subagents A/B/C/D simultaneously |
