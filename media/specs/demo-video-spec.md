# RTP Colosseum Demo Video — AI-Driven Production Spec

## Philosophy

This video needs to do two things: prove it works and show how deep the rabbit hole goes. A solo developer built a three-layer autonomous system with 325 Rust tests, on-chain Flash Trade CPI execution, a 30K-config-per-night research engine, 6-wing swarm governance, and 16 constitutional invariants — all running live right now on Railway. Most of that is invisible from the dashboard. The demo must make the invisible visible.

Browser recording is the backbone. Gstack is the right tool — it provides persistent Chromium with sub-200ms commands, snapshot-based element selection, connect mode for real visible Chrome, and the screenshot/prettyscreenshot commands for clean captures. Browser-harness is interesting (raw CDP, self-healing) but it's designed for LLM task completion, not demo recording — it doesn't have gstack's 50+ commands, cookie import, or annotation/overlay features.

The core trick: use gstack's prettyscreenshot to capture "hero frames" at every significant moment, then use HyperFrames to compose those screenshots with animated overlays (callouts, zoom highlights, data annotations) that point at the interesting parts. Pure browser recording is boring — watch someone scroll a page. Browser recording with animated annotations explaining what's happening? That's a demo.

## Tool Stack

| Tool | Role | Why this one |
|------|------|-------------|
| **gstack** | Browser driving + screenshot capture | Persistent Chromium, 50+ commands, `prettyscreenshot`, `connect` mode for real Chrome, `snapshot -i` for element refs, `style` for cleanup |
| **HyperFrames** | Animated scenes (intro, metrics, architecture, overlays, outro) | HTML-to-video with GSAP animation, scene transitions, TTS pipeline |
| **open-design** | Design system + visual identity | Clone needed. 71 brand-grade design systems including Dark Premium. Provides palette, typography, motion defaults |
| **Kokoro-82M** | TTS voiceover narration | Via HyperFrames TTS pipeline. Natural-sounding English |
| **FFmpeg** | Final concatenation + encoding | System-installed. Concatenates animated + recorded segments |

Not used: browser-harness (designed for LLM task completion, not recording — no screenshot annotation, no cleanup, no cleanup/undo, no diff). Playwright raw (gstack wraps it with persistent daemon and command layer).

## Pre-Production Setup

1. `cd ~/tabs && git clone https://github.com/nexu-io/open-design` — design systems
2. Create `~/tabs/rtp-demo-video/` with subdirs: `compositions/`, `screenshots/`, `audio/`, `recordings/`, `output/`
3. Write `DESIGN.md` using open-design's Dark Premium palette:
   - `#0a0f1a` (deep navy — bg)
   - `#e8edf5` (cool white — fg)
   - `#14f195` (Solana green — accent, for positive/LIVE/success)
   - `#9945ff` (Solana purple — secondary, for architecture layers)
   - `#f43f5e` (rose — danger/stops/rejections)
4. Verify gstack: `~/.claude/skills/gstack/bin/browse status`

## Video Structure (3 min, 10 scenes)

### Scene 1: Title Card (HyperFrames, 10s)

**Narration**: "Resilient Token Protocol. Autonomous treasury infrastructure for every token on Solana."

Dark canvas (#0a0f1a). Ambient glow (#14f195, 3%, breathing). Title slides up: "RESILIENT TOKEN PROTOCOL" in 120px bold. Subtitle fades in. Three pills stagger: "Flash Trade CPI" / "6-Wing Swarm" / "30K Configs/Night". Ghost text "RTP" 300px 4% opacity. Crossfade.

### Scene 2: Dashboard Hero — The One-Liner (gstack recording, ~15s)

**Narration**: "Token projects route trading fees to RTP. The swarm generates yield via on-chain perpetuals. Yield flows back to holders. No human keypair exists."

gstack drives Chrome to resilientprotocol.xyz. prettyscreenshot captures hero section. HyperFrames overlay: animated callout arrow pointing at "Every token gets a program-enforced treasury" with highlight sweep. Dwell 3s. Show it's a real live site (not a mockup).

### Scene 3: Live Autonomous Trader (gstack recording, ~25s)

**Narration**: "The trader runs 24/7 on Railway. No human in the loop. Right now it's watching SOL with an open position, waiting for the trailing stop to trigger."

This is the most important scene. Deep showcase:

1. gstack: `goto https://www.resilientprotocol.xyz` → scroll to "Live Trading" section
2. prettyscreenshot captures the LIVE green dot, SOL price, signal score, RSI, bullish/bearish
3. HyperFrames overlay: zoom highlight on LIVE status, animated data labels explaining each field
4. gstack: scroll to "Validated Strategy" section — Calmar 44.89, +554%, 9/9 folds, 429 trades
5. prettyscreenshot captures the metrics
6. HyperFrames overlay: animated counter reveals on each metric, "Out-of-sample — not a backtest" callout

**Why this matters**: Judges need to see that the system is live and autonomous, not a demo script. The real-time SOL price and open position prove it's actively trading.

### Scene 4: Mainnet Proof — Solana Explorer (gstack recording, ~20s)

**Narration**: "Real mainnet transactions. Not testnet. Not simulation. The Treasury PDA signs via invoke_signed — no private key exists for trading."

Confirmed mainnet transactions (from live trader):
- Open TX `2bLg1Fu...` — 99,214 CU consumed, Flash Trade CPI open (CPI proof)
- Close TX `dFqkoP2...` — SOL returned to treasury (CPI proof)
- Open TX `MQNU7AbR...` — score=0.400, 3 bullish TFs (live trader)
- Open TX `55BrK7Fi...` — post-redeploy position (live trader)
- Close TX `56PLUQA...` — SOL returned (live trader)

1. gstack: `goto https://explorer.solana.com/tx/YtGKq46w...?cluster=mainnet-beta` (latest live trader open TX)
2. prettyscreenshot captures transaction detail
3. HyperFrames overlay: animated highlight on Flash Trade program invocation, "invoke_signed" callout with arrow
4. Show the program log — real CU consumption, real signatures
5. Second tab: close TX `56PLUQA...` showing SOL returned to treasury

**Why this matters**: This is the hardest thing to fake. Real mainnet TX with real program logs. Judges can verify by pasting the signature.

### Scene 5: The invoke_signed Deep Dive (HyperFrames composition, ~20s)

**Narration**: "The Treasury PDA owns all assets. No private key. The program IS the only authority. Here's the exact CPI flow: Treasury PDA seeds passed to invoke_signed, Flash Trade opens the position on-chain, SOL yield returns to the vault."

Animated code walkthrough (HyperFrames composition):
- Show the Rust code path: bridge.rs → chain_client.rs instruction builder → lib.rs handler → invoke_signed with PDA seeds
- Animated code highlighting: each function name glows as narration mentions it
- Arrow flow: "Trading Wing → Treasury PDA → Flash Trade CPI → Position Opened"
- Final frame: "99,214 CU consumed — confirmed on mainnet"

**Why this matters**: This is the technical depth that separates RTP from "we built a trading bot." The invoke_signed architecture is unique — no human key, program-only authority. Judges who read code need to see this.

### Scene 6: Research Engine — Night Shift (gstack + HyperFrames, ~20s)

**Narration**: "Every night, the research engine evaluates thirty thousand strategy configurations. Nine-fold walk-forward validation. Only strategies surviving all nine independent time windows get promoted to Live."

1. gstack: `goto https://www.resilientprotocol.xyz/docs` → scroll to research section
2. prettyscreenshot captures night shift stats
3. HyperFrames data-viz: animated pipeline diagram:
   - Grid of 30,720 configs → Darwinian filter → 9-fold WFA → full-sim validation → Survivor 2.69
4. Animated counter: "30,720 → 15,280 → 9 folds → 1 survivor"
5. Show the Survivor 2.69 config: signal_threshold=0.25, tp_atr=5.0, sl_atr=2.7, trailing_stop_atr=0.14, min_alignment=3, leverage=9x

**Why this matters**: The research engine is the brain. 30K configs/night with 9-fold WFA is quantitative rigor most hackathon projects don't have. Show the pipeline, not just the result.

### Scene 7: Three-Layer Architecture (HyperFrames, ~15s)

**Narration**: "Three layers. Python research brain up top. Rust swarm runtime in the middle. Solana on-chain treasury at the bottom. The LLM sits in the Evolve Wing, proposing mutations within soulcontract bounds."

Animated stack diagram:
- Layer 1 (bottom): "Solana On-Chain" — Treasury PDA, Flash Trade CPI, Phase Evolution (green accent)
- Layer 2 (middle): "Rust Swarm" — 6 Wings, Coordinator, Soulguard, Evolve Wing with LLM (purple accent)
- Layer 3 (top): "Python Research" — Night Shift, WFA, Darwinian (white accent)
- Animated data flow arrows: research validates → swarm executes → on-chain settles
- Callout: "Evolve Wing LLM proposes, Soulguard disposes" with constraint check animation

### Scene 8: Railway Infrastructure (gstack recording, ~15s)

**Narration**: "Seven services, all green, all autonomous. The trader runs as an always-on service polling every five minutes."

1. gstack: navigate to Railway dashboard (authenticated via CLI), or compose terminal output from `railway status`
2. prettyscreenshot captures service list
3. HyperFrames overlay: service diagram with green dots:
   - rtp-trader (always-on) → rtp-dashboard (SSR)
   - rtp-night-shift (daily cron) → rtp-devnet-loop (6h cron)
   - rtp-swarm-ci (validation) → rtp-fee-crank (hourly) → rtp-promote-strategy (daily cron)
4. Callout: "Zero human intervention. Self-funded gas."

### Scene 9: Terminal — Tests + CLI (gstack recording, ~25s)

**Narration**: "Three hundred and twenty-five Rust tests. Zero failures. The CLI deploys a treasury with one command. Let's see it."

This is the "show me the code" scene:

1. Terminal: `cd rtp/swarm && cargo test --lib 2>&1 | tail -5`
   - Output: `test result: ok. 325 passed; 0 failed; 0 ignored`
2. Terminal: `cd ../.. && npx tsx cli/bin/rtp.ts status`
   - Shows protocol health, treasury state, strategy status
3. Terminal: `npx tsx cli/bin/rtp.ts accounts derive --mint So11...1112`
   - Shows PDA derivation — treasury, strategy, vaults

Each command runs with a natural pause. gstack captures the terminal via browser or we record directly. HyperFrames adds animated highlights on key output.

**Why this matters**: 325 tests is engineering rigor. The CLI shows operational maturity. Judges who build infra respect this.

### Scene 10: Outro (HyperFrames, ~15s)

**Narration**: "Self-funding treasury. No RTP token. Pure infrastructure. Any token project integrates with one function call."

- "Self-funding treasury." fades in, holds 2s
- Crossfade: "No RTP token. Pure infrastructure."
- Crossfade: code snippet `registerWithRTP()` with syntax highlighting
- Final frame: "Resilient Token Protocol" + `github.com/tradewife/resilient-token-protocol` + `resilientprotocol.xyz`
- Fade to #0a0f1a

## Production Pipeline

```
+---------------------------------+
|1. Setup: open-design + DESIGN.md|
+---------------------------------+
                v
+---------------------------------+
|    2. Write narration script    |
+---------------------------------+
                v
+---------------------------------+
| 3. Generate TTS audio per scene |
+---------------------------------+
                v
+---------------------------------+
|   4. Record browser via gstack  |
+---------------------------------+
                v
+---------------------------------+
|   5. Capture prettyscreenshots  |
+---------------------------------+
                v
+---------------------------------+
|6. Build HyperFrames compositions|
+---------------------------------+
                v
+---------------------------------+
|    7. Render animated scenes    |
+---------------------------------+
                v
+---------------------------------+
|     8. Merge audio per scene    |
+---------------------------------+
                v
+---------------------------------+
|   9. FFmpeg concat final video  |
+---------------------------------+
```

## Step-by-step Execution

| Step | Action | Output |
|------|--------|--------|
| 1 | Clone open-design, create project dir, write DESIGN.md | Project structure + visual identity |
| 2 | Write full narration for all 10 scenes | `audio/script.md` |
| ~~3~~ | ~~Generate TTS audio via Kokoro-82M for each scene~~ | ~~Deferred — no audio for initial cut~~ |
| 4 | gstack: record Scenes 2, 3, 4, 8, 9 (browser walkthroughs) | `recordings/scene-N.webm` → convert to `.mp4` |
| 5 | gstack: `prettyscreenshot` at each hero moment in those scenes | `screenshots/hero-N.png` |
| 6 | Build HyperFrames compositions for Scenes 1, 5, 6, 7, 10 | `compositions/*.html` |
| 7 | Integrate screenshots into compositions as overlays | Compositions reference screenshots |
| 8 | `hyperframes render --workers 1` all compositions | `output/scene-N.mp4` |
| ~~9~~ | ~~Merge audio into each MP4~~ | ~~Deferred — initial cut is silent~~ |
| 10 | Concatenate all 10 scenes: `ffmpeg -f concat -safe 0 -i manifest.txt -c:v libx264 -preset slow -crf 16 -pix_fmt yuv420p -movflags +faststart rtp-demo-final.mp4` | Final 3-min video |

## Key Technical Notes

- gstack `prettyscreenshot` automatically cleans up ads, cookie banners, and visual noise before capture. Use `cleanup --all` if needed.
- gstack `connect` mode drives a visible Chrome window — if we want to show the actual browser interaction happening in real-time (screen-share style), use this. For cleaner captures, headless + prettyscreenshot is better.
- HyperFrames + browser screenshots: The HyperFrames SKILL.md documents the exact pipeline for combining animated scenes with Playwright recordings (the "split-render + concat" approach). Follow it exactly.
- No `repeat: -1` in HyperFrames — all animations must be finite repeats calculated from duration.
- Crossfade transitions between every scene — no jump cuts.
- Terminal recording: Use gstack to navigate to a web-based terminal (like Railway's terminal) or compose terminal output as styled HTML in HyperFrames with typewriter animation. Direct terminal recording is the hardest part — the HyperFrames animated terminal fallback is reliable.

## What Makes This Demo Different

1. It's a real live system, not a mockup — the trader is running right now
2. Mainnet transactions with real SOL — not testnet
3. 325 tests — engineering depth, not hackathon speed
4. invoke_signed — no human key, program IS the authority
5. 30K configs/night with 9-fold WFA — quantitative rigor
6. Seven autonomous services on Railway — no human ops
7. The video itself is AI-produced — dogfooding the agent thesis

## Output Specs

- **Duration**: ~180 seconds (3 minutes)
- **Resolution**: 1920x1080
- **Format**: MP4 (H.264, AAC)
- **File size**: ~80-120MB
- **Narration**: TTS voiceover throughout all scenes
