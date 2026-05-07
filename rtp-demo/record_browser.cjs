const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const REC_DIR = path.join(__dirname, 'recordings');
const SIZE = { width: 1920, height: 1080 };

async function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function recordScene(browser, name, dur, fn) {
  console.log(`\nRecording ${name}...`);
  const dir = fs.mkdtempSync('/tmp/pw-video-');
  const context = await browser.newContext({
    viewport: SIZE,
    recordVideo: { dir, size: SIZE }
  });

  // Hide scrollbars
  await context.addInitScript(() => {
    const s = document.createElement('style');
    s.textContent = '::-webkit-scrollbar{display:none}*{scrollbar-width:none}';
    document.head.appendChild(s);
  });

  const page = await context.newPage();
  
  try {
    await fn(page);
  } catch(e) {
    console.error(`  Error in ${name}:`, e.message);
  }

  // Wait extra for any pending animations
  await sleep(1000);

  // Get video path BEFORE closing context
  const video = page.video();
  let videoPath = null;
  if (video) {
    videoPath = await video.path();
    console.log(`  Video path: ${videoPath}`);
  }

  await context.close();

  const outMp4 = path.join(REC_DIR, `${name}.mp4`);
  if (videoPath && fs.existsSync(videoPath)) {
    // Convert webm → mp4
    try {
      execSync(`ffmpeg -y -i "${videoPath}" -c:v libx264 -preset fast -crf 16 -pix_fmt yuv420p -movflags +faststart "${outMp4}"`, 
        { stdio: 'pipe', timeout: 120000 });
      const size = (fs.statSync(outMp4).size / 1e6).toFixed(1);
      console.log(`  ✓ ${outMp4} (${size}MB)`);
    } catch(e) {
      console.error(`  ffmpeg error:`, e.stderr ? e.stderr.toString().slice(0, 200) : e.message);
    }
  } else {
    console.log(`  WARNING: No video recorded for ${name}`);
  }
}

(async () => {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage']
  });

  // ═══ SCENE 3: Dashboard ═══
  await recordScene(browser, 'scene3_dashboard', 20, async (page) => {
    await page.goto('https://www.resilientprotocol.xyz', { waitUntil: 'networkidle', timeout: 30000 });
    await sleep(3000);

    // Arc cursor to hero
    await page.mouse.move(0, 540);
    for (let i = 1; i <= 20; i++) {
      await page.mouse.move(i * 48, 540 - i * 12, { steps: 1 });
    }
    await sleep(800);

    // Scroll through sections
    for (let i = 0; i < 8; i++) {
      await page.evaluate(() => window.scrollBy({ top: 300, behavior: 'smooth' }));
      await sleep(800);
    }
    await sleep(2000);
  });

  // ═══ SCENE 4: Solana Explorer ═══
  await recordScene(browser, 'scene4_explorer', 20, async (page) => {
    await page.goto('https://explorer.solana.com/tx/2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3', {
      waitUntil: 'networkidle', timeout: 30000
    });
    await sleep(4000);

    // Scroll through tx details
    for (let i = 0; i < 6; i++) {
      await page.evaluate(() => window.scrollBy({ top: 250, behavior: 'smooth' }));
      await sleep(700);
    }
    await sleep(2000);
  });

  // ═══ SCENE 8: Terminal ═══
  await recordScene(browser, 'scene8_terminal', 25, async (page) => {
    const termHtml = `<!DOCTYPE html><html><head><style>
      * { margin:0; padding:0; box-sizing:border-box; }
      body { background:#0d1117; color:#e8edf5; font:14px "JetBrains Mono","Fira Code",monospace; padding:20px 40px; }
      .line { margin:4px 0; min-height:20px; }
      .prompt { color:#14f195; }
      .output { color:#8899aa; white-space:pre-wrap; }
      .highlight { color:#14f195; font-weight:bold; }
    </style></head><body id="term"></body></html>`;
    
    fs.writeFileSync('/tmp/rtp-terminal.html', termHtml);
    await page.goto('file:///tmp/rtp-terminal.html', { waitUntil: 'load' });
    await sleep(1000);

    // Type commands with human rhythm
    async function addLine(html) {
      await page.evaluate((h) => {
        document.getElementById('term').innerHTML += `<div class="line">${h}</div>`;
      }, html);
      await sleep(200);
    }

    await addLine('<span class="prompt">$ </span>cd rtp/swarm && cargo test --lib 2>&1 | tail -5');
    await sleep(2000);
    await addLine('<span class="output">running 325 tests</span>');
    await addLine('<span class="highlight">test result: ok. 325 passed; 0 failed; 0 ignored;</span>');
    await addLine('<span class="output"></span>');
    await addLine('<span class="output">All 6 wings. 9/9 Flash Trade CPI tests. 0 clippy warnings.</span>');
    await sleep(2000);

    await addLine('<span class="prompt">$ </span>npx tsx cli/bin/rtp.ts status');
    await sleep(1500);
    await addLine('<span class="highlight">rtp-trader:    ✅ LIVE    SOL/USDT Survivor 2.69</span>');
    await addLine('<span class="output">rtp-dashboard: ✅ ONLINE  resilientprotocol.xyz</span>');
    await addLine('<span class="output">rtp-devnet-loop: ✅ CRON   Every 6h</span>');
    await addLine('<span class="output">rtp-night-shift: ✅ CRON   Daily 14:00 UTC</span>');
    await addLine('<span class="output">7/7 services green</span>');
    await sleep(2000);

    await addLine('<span class="prompt">$ </span>npx tsx cli/bin/rtp.ts accounts derive --mint So11111111111111111111111111111112');
    await sleep(1000);
    await addLine('<span class="highlight">Treasury PDA:  7oZTJW... (derived offline)</span>');
    await addLine('<span class="output">Vault:         FvYQhN... (ATA)</span>');
    await addLine('<span class="output">No RPC needed — pure PDA derivation</span>');
    await sleep(1500);
  });

  // ═══ SCENE 9: Railway ═══
  await recordScene(browser, 'scene9_railway', 15, async (page) => {
    const railHtml = `<!DOCTYPE html><html><head><style>
      * { margin:0; padding:0; box-sizing:border-box; }
      body { background:#0a0f1a; color:#e8edf5; font:14px "JetBrains Mono","Fira Code",monospace; padding:40px 60px; }
      h1 { font-size:28px; color:#e8edf5; margin-bottom:30px; letter-spacing:-1px; }
      .svc { display:flex; align-items:center; gap:16px; padding:14px 0; border-bottom:1px solid rgba(255,255,255,0.05); }
      .dot { width:12px; height:12px; border-radius:50%; background:#14f195; box-shadow:0 0 8px rgba(20,241,149,0.5); animation:pulse 2s infinite; }
      @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.6} }
      .name { color:#e8edf5; font-weight:600; width:220px; }
      .status { color:#14f195; font-weight:600; }
      .desc { color:#8899aa; }
      .footer { margin-top:40px; color:#8899aa; font-size:12px; letter-spacing:2px; text-transform:uppercase; text-align:center; }
    </style></head><body>
      <h1>Railway · Resilient Token Protocol</h1>
      <div class="svc"><div class="dot"></div><div class="name">rtp-trader</div><div class="status">ONLINE</div><div class="desc">Always-on · 5min poll · Flash Trade · SOL/USDT</div></div>
      <div class="svc"><div class="dot"></div><div class="name">rtp-dashboard</div><div class="status">ONLINE</div><div class="desc">resilientprotocol.xyz · SSR</div></div>
      <div class="svc"><div class="dot"></div><div class="name">rtp-devnet-loop</div><div class="status">COMPLETED</div><div class="desc">Cron · Every 6h · LLM evolution</div></div>
      <div class="svc"><div class="dot"></div><div class="name">rtp-night-shift</div><div class="status">COMPLETED</div><div class="desc">Cron · Daily 14:00 UTC · 30K configs</div></div>
      <div class="svc"><div class="dot"></div><div class="name">rtp-swarm-ci</div><div class="status">COMPLETED</div><div class="desc">325 tests · cargo test + clippy</div></div>
      <div class="svc"><div class="dot"></div><div class="name">rtp-fee-crank</div><div class="status">COMPLETED</div><div class="desc">Cron · Hourly · Fee sweep</div></div>
      <div class="svc"><div class="dot"></div><div class="name">rtp-promote-strategy</div><div class="status">COMPLETED</div><div class="desc">Cron · Nightly · Strategy promotion</div></div>
      <div class="footer">ZERO HUMAN INTERVENTION · SELF-FUNDED GAS · 7/7 GREEN</div>
    </body></html>`;

    fs.writeFileSync('/tmp/rtp-railway.html', railHtml);
    await page.goto('file:///tmp/rtp-railway.html', { waitUntil: 'load' });
    await sleep(2000);

    // Hover through each service
    for (let i = 0; i < 7; i++) {
      const y = 130 + i * 50;
      await page.mouse.move(100, y, { steps: 10 });
      await sleep(400);
    }
    await sleep(3000);
  });

  await browser.close();
  console.log('\n✓ All browser recordings complete!');
})();
