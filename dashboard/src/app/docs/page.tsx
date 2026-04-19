"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import Topbar from "../Topbar";

// --- Doc content definitions ---

interface DocSection {
  slug: string;
  title: string;
  content: React.ReactNode;
}

interface DocGroup {
  label: string;
  items: DocSection[];
}

// --- Content components ---

function CodeBlock({ children, language = "typescript" }: { children: string; language?: string }) {
  return (
    <div className="docs-code-block">
      <div className="docs-code-header">
        <span>{language}</span>
      </div>
      <pre><code>{children}</code></pre>
    </div>
  );
}

function Callout({ type = "info", title, children }: { type?: "info" | "warning" | "tip"; title?: string; children: React.ReactNode }) {
  const colors = {
    info: { bg: "rgba(0,150,136,0.08)", border: "var(--emerald)" },
    warning: { bg: "rgba(255,107,107,0.08)", border: "var(--coral)" },
    tip: { bg: "rgba(0,210,140,0.06)", border: "#00d18c" },
  };
  const c = colors[type];
  return (
    <div className="docs-callout" style={{ background: c.bg, borderLeft: `3px solid ${c.border}` }}>
      {title && <div className="docs-callout-title" style={{ color: c.border }}>{title}</div>}
      <div className="docs-callout-body">{children}</div>
    </div>
  );
}

function Table({ headers, rows }: { headers: string[]; rows: string[][] }) {
  return (
    <div className="docs-table-wrap">
      <table className="docs-table">
        <thead>
          <tr>{headers.map((h) => <th key={h}>{h}</th>)}</tr>
        </thead>
        <tbody>
          {rows.map((row, i) => <tr key={i}>{row.map((cell, j) => <td key={j} dangerouslySetInnerHTML={{ __html: cell }} />)}</tr>)}
        </tbody>
      </table>
    </div>
  );
}

// --- Doc content ---

const DOC_GROUPS: DocGroup[] = [
  {
    label: "Overview",
    items: [
      {
        slug: "overview",
        title: "What is RTP?",
        content: (
          <>
            <p>RTP (Resilient Token Protocol) gives every token a program-enforced treasury vault on Solana. Trading fees accumulate in the vault, an autonomous swarm generates yield on Hyperliquid, and yield flows back to holders, developers, and ecosystem — 70/20/10 split, enforced on-chain. Forever.</p>
            <p>There is no RTP token. RTP is infrastructure.</p>

            <h3>Why This Is Different</h3>
            <ul>
              <li><strong>Constitutional governance</strong> — soulcontract enforced in Rust AND on-chain (Anchor program). No one — not even the team — can override the rules. This is not a promise — it&apos;s a <code>require!</code> constraint.</li>
              <li><strong>Self-funding economics</strong> — treasury generates its own yield via Hyperliquid perps, with irreversible phase evolution (Sustenance → Ecosystem → Humanity). No VC dependency.</li>
              <li><strong>Proven research engine</strong> — 30,000 strategy configs tested per night, 9-fold walk-forward validation, Darwinian evolution. Not a backtest screenshot — out-of-sample results across 9 independent time windows.</li>
              <li><strong>Real execution</strong> — EIP-712 signed orders from Rust, fills on Hyperliquid testnet, USDC yield deposited to Solana treasury PDA. BUY→fill→SELL→fill→PnL round-trip verified.</li>
              <li><strong>307 Rust tests, 0 failures</strong> — 6-wing swarm architecture with Security, Audit, Evolve, Knowledge, and Futureproof wings. Not a wrapper around an API — a real multi-agent system.</li>
            </ul>

            <h3>How It Works</h3>
            <ol>
              <li><strong>Fees arrive</strong> — trading fees from the token flow to the per-mint treasury vault PDA</li>
              <li><strong>Swarm trades</strong> — the autonomous swarm executes validated strategies on Hyperliquid (USDC-margined, no SOL liquidation risk)</li>
              <li><strong>Yield returns</strong> — generated yield flows back to the treasury PDA</li>
              <li><strong>Redistribution</strong> — 70% to holders, 20% to project dev, 10% to ecosystem (enforced on-chain)</li>
            </ol>

            <h3>What&apos;s Been Built</h3>
            <Table
              headers={["Component", "Status", "Detail"]}
              rows={[
                ["Anchor treasury program", "✅ Deployed (devnet)", "8/8 on-chain steps completed including redistribution"],
                ["Rust swarm runtime", "✅ 307 tests passing", "6 wings: Trading, Security, Evolve, Knowledge, Audit, Futureproof"],
                ["Hyperliquid execution", "✅ Round-trip verified", "BUY→fill→SELL→fill→PnL from Rust, EIP-712 signed"],
                ["Treasury yield deposit", "✅ On-chain confirmed", "USDC yield → SOL → treasury PDA via CPI transfer"],
                ["Autonomous daemon", "✅ 7 cycles completed", "6h cron, LLM-driven strategy evolution, auditable trail"],
                ["SDK", "✅ Shipped", "<code>@resilient-protocol/sdk</code> — one function call to register any token"],
                ["Dashboard", "✅ Live", "<a href='https://resilientprotocol.xyz' style='color: var(--coral)'>resilientprotocol.xyz</a> — live treasury state, wallet connect"],
              ]}
            />

            <Callout type="tip" title="Token Creator or Platform?">
              <p>If your launchpad supports RTP, enabling the treasury takes one click. See <a href="#getting-started-creators" style={{ color: "var(--coral)" }}>Getting Started for Token Creators</a>. If you&apos;re a platform integrating RTP as a feature, see <a href="#getting-started-platforms" style={{ color: "var(--coral)" }}>Getting Started for Platforms</a>.</p>
            </Callout>
          </>
        ),
      },
      {
        slug: "getting-started-creators",
        title: "Getting Started — Token Creators",
        content: (
          <>
            <p>Two ways to get an RTP treasury for your token: launch from our site, or register an existing token.</p>

            <h3>Path A: Launch From RTP</h3>
            <p>Use the <a href="/launch" style={{ color: "var(--coral)" }}>RTP launch page</a>. Pick your platform, fill in token details, sign with Phantom. The token goes live on-chain and the RTP treasury is initialized automatically.</p>
            <ol>
              <li><strong>Pick a platform</strong> — Pump.fun, Bags.fm, or Raydium LaunchLab</li>
              <li><strong>Fill in token details</strong> — name, symbol, image, description</li>
              <li><strong>Sign with Phantom</strong> — one transaction creates the token on-chain</li>
              <li><strong>RTP treasury auto-initializes</strong> — treasury PDA, vault PDA, and adopter record created</li>
            </ol>

            <h3>Path B: Register An Existing Token</h3>
            <p>Already have a token on Pump.fun, Bags.fm, or Raydium? Register it with RTP programmatically:</p>
            <CodeBlock>{`import { registerWithRTP } from "@resilient-protocol/sdk";
import { Connection, PublicKey } from "@solana/web3.js";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const result = await registerWithRTP(connection, wallet, {
  mint: new PublicKey("YourExistingMintAddress"),
  platform: "pumpfun",  // or "bags" or "raydium"
  name: "My Token",
  symbol: "MTK",
});

// result.treasuryPDA — your token's treasury
// result.vaultPDA — where fees accumulate`}</CodeBlock>

            <h3>After Registration: What Happens</h3>
            <ol>
              <li><strong>Fees accumulate</strong> — trading fees flow into your treasury vault PDA</li>
              <li><strong>Swarm activates</strong> — autonomous strategies execute on Hyperliquid (USDC-margined, no SOL liquidation risk)</li>
              <li><strong>Yield returns</strong> — generated yield flows back to the treasury PDA</li>
              <li><strong>Redistribution</strong> — 70% to holders, 20% to project dev, 10% to ecosystem (enforced on-chain)</li>
            </ol>

            <h3>Fee Routing Per Platform</h3>
            <p>Trading fees must reach the RTP treasury vault. The mechanism differs per platform:</p>
            <Table
              headers={["Platform", "Can route to RTP?", "Changeable?", "How"]}
              rows={[
                ["Pump.fun", "Yes", "Once only", "One-time fee redirect to treasury PDA. RTP keeper claims from deployer wallet."],
                ["Bags.fm", "Yes", "Anytime", "Set treasury PDA as fee claimer. Update claimers anytime via admin API."],
                ["Raydium", "Yes", "Limited", "Creator fees go to pool_creator wallet. Forward to RTP manually or via platform redirect."],
              ]}
            />

            <Callout type="info" title="Pump.fun One-Time Redirect">
              <p>Pump.fun allows only <strong>one</strong> post-launch fee redirect per token. If you redirect to the RTP treasury PDA, that&apos;s your one shot — make it count. After redirecting, the RTP keeper handles claiming and forwarding automatically.</p>
            </Callout>

            <Callout type="tip" title="No RTP Token Required">
              <p>RTP is infrastructure, not a token. You keep your own token. RTP wraps it with treasury functionality.</p>
            </Callout>
          </>
        ),
      },
      {
        slug: "getting-started-platforms",
        title: "Getting Started — Platforms",
        content: (
          <>
            <p>This guide is for launchpads and token deployment platforms integrating RTP as a feature they offer to token creators.</p>

            <h3>Prerequisites</h3>
            <ul>
              <li>A Solana launchpad or token deployment platform</li>
              <li>Node.js 18+</li>
              <li><code>@solana/web3.js</code> v1.98+</li>
              <li>A Phantom wallet (for signing transactions)</li>
            </ul>

            <h3>Installation</h3>
            <CodeBlock>{`npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor`}</CodeBlock>

            <h3>Three-Step Integration</h3>

            <h4>Step 1: Register the token with RTP</h4>
            <p>After your platform creates a token mint, call <code>registerWithRTP</code>:</p>
            <CodeBlock>{`import { registerWithRTP } from "@resilient-protocol/sdk";

const result = await registerWithRTP(connection, wallet, {
  mint: new PublicKey(mintAddress),
  platform: "pumpfun",
  name: "My Token",
  symbol: "MTK",
  holdersWallet: publicKey,
  projectDevWallet: publicKey,
  ecosystemWallet: publicKey,
});`}</CodeBlock>
            <p>This creates the treasury PDA, vault PDA, and adopter record on-chain.</p>

            <h4>Step 2: Configure fee routing</h4>
            <Table
              headers={["Platform", "Fee Routing Method", "Guide"]}
              rows={[
                ["Pump.fun", "Creator fees → deployer wallet → RTP keeper claims", "<a href='#pump-fun'>Pump.fun</a>"],
                ["Bags.fm", "Multi-claimer fee share with treasury PDA", "<a href='#bags-fm'>Bags.fm</a>"],
                ["Raydium", "<code>updatePlatformCpCreator</code> → treasury PDA", "<a href='#raydium'>Raydium</a>"],
              ]}
            />

            <h4>Step 3: Display treasury state (optional)</h4>
            <CodeBlock>{`import { fetchTreasuryState } from "@resilient-protocol/sdk";

const state = await fetchTreasuryState(connection, mintAddress);
// state.phase, state.vaultBalance, state.totalFeesWithdrawn`}</CodeBlock>

            <Callout type="info" title="Enterprise API (Planned)">
              <p>If you prefer not to run chain operations yourself, the Enterprise API will offer the same functionality via a simple REST API. See <a href="#enterprise-api" style={{ color: "var(--coral)" }}>Enterprise API</a> for the planned surface.</p>
            </Callout>
          </>
        ),
      },
    ],
  },
  {
    label: "Integration",
    items: [
      {
        slug: "pump-fun",
        title: "Pump.fun",
        content: (
          <>
            <p>Pump.fun is the highest-volume memecoin launchpad on Solana. Creator fees (0.05–0.95% dynamic) go to the token deployer&apos;s wallet. RTP integrates via a keeper that periodically claims these fees and forwards them to the treasury PDA.</p>

            <h3>How It Works</h3>
            <ol>
              <li>Trader buys/sells → Pump.fun collects fees</li>
              <li>Creator fee portion → deployer wallet</li>
              <li>RTP keeper claims from deployer wallet</li>
              <li>Keeper forwards to treasury vault PDA</li>
              <li><code>withdraw_fees</code> confirms deposit on-chain</li>
            </ol>

            <h3>Launch via PumpPortal API</h3>
            <CodeBlock>{`const response = await fetch("https://pumpportal.fun/api/trade-local", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    publicKey: wallet.publicKey.toBase58(),
    action: "create",
    tokenMetadata: {
      name: "My Token",
      symbol: "MTK",
      uri: "https://ipfs.io/ipfs/...",
    },
    mint: mintKeypair.publicKey.toBase58(),
    denominatedInSol: "true",
    amount: 0.1,
    slippage: 10,
    priorityFee: 0.00005,
    pool: "pump",
  }),
});`}</CodeBlock>

            <h3>Register with RTP</h3>
            <CodeBlock>{`const result = await registerWithRTP(connection, wallet, {
  mint: new PublicKey(mintAddress),
  platform: "pumpfun",
  name: "My Token",
  symbol: "MTK",
});`}</CodeBlock>

            <h3>Fee Structure</h3>
            <Table
              headers={["Market Cap", "Creator Fee", "Protocol Fee", "Total"]}
              rows={[
                ["&lt; $100K", "0.95%", "1.25%", "2.20%"],
                ["$100K – $1M", "0.50%", "1.00%", "1.50%"],
                ["&gt; $1M", "0.05%", "0.30%", "0.35%"],
              ]}
            />

            <Callout type="info" title="Limitations">
              <p>No direct PDA routing — fees go to deployer wallet. RTP keeper handles claiming. Mainnet only. No API key required.</p>
            </Callout>
          </>
        ),
      },
      {
        slug: "bags-fm",
        title: "Bags.fm",
        content: (
          <>
            <p>Bags.fm runs on Meteora DLMM pools. Its standout feature for RTP is <strong>multi-claimer fee sharing</strong> — up to 100 fee claimers with custom basis point splits. Set the RTP treasury PDA as a fee claimer and fees route there automatically.</p>

            <h3>How It Works</h3>
            <ol>
              <li>Trader buys/sells → Meteora DLMM collects fees</li>
              <li>Fee share API splits → 70% treasury PDA, 30% project wallet</li>
              <li>Fees arrive directly in the vault (no keeper needed)</li>
            </ol>

            <h3>Configure Fee Sharing with RTP</h3>
            <CodeBlock>{`const rtpResult = await registerWithRTP(connection, wallet, {
  mint: new PublicKey(tokenMint),
  platform: "bags",
  name: "My Token",
  symbol: "MTK",
});

// Configure Bags.fm fee sharing: 70% to RTP treasury, 30% to project
const configRes = await fetch("https://api.bags.fm/v1/config/create-fee-share", {
  method: "POST",
  headers: {
    "Content-Type": "application/json",
    "x-api-key": BAGS_API_KEY,
  },
  body: JSON.stringify({
    payer: wallet.publicKey.toBase58(),
    baseMint: tokenMint,
    feeClaimers: [
      { user: rtpResult.treasuryPDA, userBps: 7000 },  // 70% to RTP
      { user: wallet.publicKey.toBase58(), userBps: 3000 }, // 30% to project
    ],
  }),
});`}</CodeBlock>

            <h3>Fee Sharing Models</h3>
            <Table
              headers={["Mode", "Pre-Graduation", "Post-Graduation", "Best For"]}
              rows={[
                ["Default", "2%", "2%", "Community tokens"],
                ["Low Pre / High Post", "0.25%", "1%", "Growth phase"],
                ["High Pre / Low Post", "1%", "0.25%", "Established tokens"],
                ["High Flat", "10%", "10%", "Aggressive fee capture"],
              ]}
            />

            <Callout type="tip" title="Best for RTP">
              <p>Bags.fm has the most flexible fee-sharing model. Up to 100 claimers with arbitrary bps splits. Ideal for splitting fees between treasury PDA, project team, and stakeholders.</p>
            </Callout>

            <Callout type="info" title="Requirements">
              <p>Mainnet only. API key required (get one at <a href="https://dev.bags.fm" style={{ color: "var(--coral)" }}>dev.bags.fm</a>). Fee claimers must sum to 10,000 bps.</p>
            </Callout>
          </>
        ),
      },
      {
        slug: "raydium",
        title: "Raydium LaunchLab",
        content: (
          <>
            <p>Raydium is the #1 DEX/AMM on Solana by TVL. LaunchLab supports explicit PDA-based fee collection — the best architectural fit for RTP. Full TypeScript SDK with <code>@raydium-io/raydium-sdk-v2</code>.</p>

            <h3>Why Raydium</h3>
            <ul>
              <li><strong>Explicit PDA fee collection</strong> — documented and recommended by Raydium</li>
              <li><code>updatePlatformCpCreator</code> redirects all creator fees to a PDA</li>
              <li><code>collectMultiCreatorFees</code> for batch collection</li>
              <li><strong>Devnet support</strong> with sUSDC as quote token</li>
              <li><strong>Largest DEX</strong> — maximum liquidity access</li>
            </ul>

            <h3>Create a LaunchLab Token</h3>
            <CodeBlock>{`import { Raydium } from "@raydium-io/raydium-sdk-v2";

const raydium = await Raydium.load({ connection, owner: wallet });

const { execute } = await raydium.launchpad.createLaunchpad({
  programId: RAYDIUM_LAUNCHLAB_PROGRAM_ID,
  mintA: { mint: NATIVE_MINT, decimals: 9 },
  mintB: { mint: tokenMint, decimals: 6 },
  name: "My Token",
  symbol: "MTK",
  curveType: "exponential",
  config: {
    lpFundBps: 5000,
    creatorFundBps: 250,
    protocolFundBps: 250,
  },
});`}</CodeBlock>

            <h3>Redirect Creator Fees to Treasury PDA</h3>
            <CodeBlock>{`const rtpResult = await registerWithRTP(connection, wallet, {
  mint: tokenMint,
  platform: "raydium",
  name: "My Token",
  symbol: "MTK",
});

// After graduation: redirect all creator fees to RTP treasury
await raydium.launchpad.updatePlatformCpCreator({
  poolId: graduatedPoolId,
  newCreator: new PublicKey(rtpResult.treasuryPDA),
});`}</CodeBlock>

            <h3>Fee Structure</h3>
            <Table
              headers={["Stage", "Creator Fee", "Protocol Fee", "LP Fee"]}
              rows={[
                ["Bonding curve", "0.25%", "0.25%", "—"],
                ["Post-migration (CPMM)", "Configurable", "0.25%", "Dynamic"],
              ]}
            />

            <Callout type="tip" title="Devnet Testing">
              <p>Raydium is the only platform in the RTP lineup that supports devnet testing. Use sUSDC as the quote token on devnet to test the full fee-routing flow without spending real SOL.</p>
            </Callout>
          </>
        ),
      },
    ],
  },
  {
    label: "Enterprise",
    items: [
      {
        slug: "enterprise-api",
        title: "Enterprise API (Planned)",
        content: (
          <>
            <Callout type="warning" title="Planned Feature">
              <p>The Enterprise API is in development. This section documents the planned API surface for platform integrators. Endpoints and schemas may change before launch.</p>
            </Callout>

            <p>The RTP Enterprise API lets launchpads offer RTP treasury infrastructure to their token creators via a simple REST API. Your platform adds an &quot;Enable RTP Treasury&quot; button, and the Enterprise API handles PDA creation, fee routing configuration, and treasury state queries — no SDK installation or Solana wallet management required on your end.</p>

            <h3>How It Works</h3>
            <ol>
              <li><strong>Your platform</strong> adds an &quot;Enable RTP Treasury&quot; option for token creators</li>
              <li><strong>Token creator clicks enable</strong> — your frontend calls the Enterprise API</li>
              <li><strong>Enterprise API</strong> creates the treasury PDA, vault PDA, and adopter record on-chain</li>
              <li><strong>Enterprise API</strong> configures fee routing to the treasury PDA</li>
              <li><strong>RTP swarm</strong> begins monitoring the treasury and executing yield strategies</li>
              <li><strong>Yield flows back</strong> — 70% to holders, 20% to project dev, 10% to ecosystem</li>
            </ol>

            <Callout type="info" title="Base URL">
              <p><code>https://api.resilientprotocol.com</code> (planned)</p>
            </Callout>

            <h3>Authentication</h3>
            <CodeBlock language="http">{`Authorization: Bearer rtp_live_abc123...`}</CodeBlock>
            <p>API keys are issued per launchpad. Contact the RTP team to register for early access.</p>

            <h3>Planned Endpoints</h3>

            <h4>POST /v1/adopt</h4>
            <p>Registers a token with RTP. Creates treasury PDA, vault PDA, and adopter record on-chain.</p>
            <CodeBlock language="json">{`{
  "mint": "EPjFWdd5...",
  "platform": "pumpfun",
  "name": "Community Token",
  "symbol": "CMTY",
  "holdersWallet": "ABCx...1234",
  "projectDevWallet": "ABCx...1234",
  "ecosystemWallet": "ABCx...1234"
}`}</CodeBlock>
            <p><strong>Response:</strong></p>
            <CodeBlock language="json">{`{
  "treasuryPDA": "FNQbK1Vw...",
  "vaultPDA": "9xRWo1N4...",
  "adopterPDA": "7zSW...",
  "signature": "5Kq8...",
  "explorerUrl": "https://explorer.solana.com/tx/..."
}`}</CodeBlock>

            <h4>POST /v1/fee-route</h4>
            <p>Configures platform-specific fee routing to the RTP treasury.</p>

            <h4>GET /v1/treasury/:mint</h4>
            <p>Reads on-chain treasury state. No authentication required.</p>

            <h4>POST /v1/crank</h4>
            <p>Triggers the permissionless crank: withdraw fees + redistribute if above threshold.</p>

            <h3>Planned Error Codes</h3>
            <Table
              headers={["Code", "Meaning"]}
              rows={[
                ["400", "Invalid request body or missing fields"],
                ["401", "Missing or invalid API key"],
                ["404", "Token not registered with RTP"],
                ["409", "Token already registered"],
                ["500", "On-chain transaction failed"],
              ]}
            />

            <h3>SDK vs Enterprise API</h3>
            <Table
              headers={["Feature", "SDK (Direct)", "Enterprise API"]}
              rows={[
                ["Signing", "Your wallet", "RTP-managed"],
                ["Chain ops", "Your infra", "RTP infra"],
                ["Latency", "Direct", "~200ms overhead"],
                ["Setup", "More code", "HTTP calls"],
                ["Best for", "Full control", "Simple integration"],
              ]}
            />

            <Callout type="info" title="Want Early Access?">
              <p>The Enterprise API is designed for launchpads that want to offer RTP treasury features without running Solana infrastructure. Reach out to the RTP team to discuss integration.</p>
            </Callout>
          </>
        ),
      },
    ],
  },
  {
    label: "SDK Reference",
    items: [
      {
        slug: "sdk-reference",
        title: "SDK Reference (For Platforms)",
        content: (
          <>
            <p>The <code>@resilient-protocol/sdk</code> TypeScript package is for launchpads and platforms integrating RTP directly. If you&apos;re a token creator, your launchpad handles this — see <a href="#getting-started-creators" style={{ color: "var(--coral)" }}>Getting Started for Token Creators</a>.</p>

            <h3>Installation</h3>
            <CodeBlock>{`npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor`}</CodeBlock>

            <h3>Constants</h3>
            <CodeBlock>{`import { RTP_PROGRAM_ID, RTP_DEVNET_RPC, RTP_MAINNET_RPC } from "@resilient-protocol/sdk";

RTP_PROGRAM_ID  // PublicKey("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB")
RTP_DEVNET_RPC  // "https://api.devnet.solana.com"
RTP_MAINNET_RPC // "https://api.mainnet-beta.solana.com"`}</CodeBlock>

            <h3>Types</h3>

            <h4>RTPRegistrationConfig</h4>
            <CodeBlock>{`interface RTPRegistrationConfig {
  mint: PublicKey;                          // Token mint to register
  platform: "pumpfun" | "bags" | "raydium"; // Launch platform
  name: string;                             // Token display name
  symbol: string;                           // Token ticker symbol
  holdersWallet?: PublicKey;                // 70% recipient (default: payer)
  projectDevWallet?: PublicKey;             // 20% recipient (default: payer)
  ecosystemWallet?: PublicKey;              // 10% recipient (default: payer)
  minRunwayBalance?: number;                // Min runway in lamports (default: 10_000_000)
}`}</CodeBlock>

            <h4>RTPRegistrationResult</h4>
            <CodeBlock>{`interface RTPRegistrationResult {
  mint: string;           // base58 mint address
  signature: string;      // registration tx signature
  explorerUrl: string;    // Solana Explorer link
  treasuryPDA: string;    // Per-mint treasury state account
  vaultPDA: string;       // Token account receiving fees
  adopterPDA: string;     // Adopter registration account
}`}</CodeBlock>

            <h4>TreasuryState</h4>
            <CodeBlock>{`interface TreasuryState {
  mint: string;
  phase: "Sustenance" | "Ecosystem" | "Humanity";
  vaultBalance: number;
  totalFeesWithdrawn: number;
  totalDistributedHolders: number;
  totalDistributedDev: number;
  totalDistributedEcosystem: number;
  totalHydration: number;
  totalFeesReceived: number;
  minRunwayBalance: number;
}`}</CodeBlock>

            <h3>Functions</h3>

            <h4>registerWithRTP()</h4>
            <p>Registers an existing token mint with RTP. Creates treasury PDA, vault PDA, and adopter record.</p>
            <CodeBlock>{`async function registerWithRTP(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  config: RTPRegistrationConfig,
): Promise<RTPRegistrationResult>;`}</CodeBlock>

            <h4>fetchTreasuryState()</h4>
            <p>Read-only. Fetches on-chain treasury state. No signing required.</p>
            <CodeBlock>{`async function fetchTreasuryState(
  connection: Connection,
  mintAddress: string | PublicKey,
): Promise<TreasuryState>;`}</CodeBlock>

            <h4>withdrawAndRedistribute()</h4>
            <p>Permissionless crank. Withdraws fees, then triggers 70/20/10 redistribution if above threshold.</p>
            <CodeBlock>{`async function withdrawAndRedistribute(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  mintAddress: string | PublicKey,
): Promise<{ withdrawSig: string; redistributeSig?: string }>;`}</CodeBlock>

            <h3>PDA Derivation</h3>
            <CodeBlock>{`// Treasury PDA
PublicKey.findProgramAddressSync(
  [Buffer.from("treasury"), mint.toBuffer()],
  RTP_PROGRAM_ID,
);

// Vault PDA
PublicKey.findProgramAddressSync(
  [Buffer.from("treasury"), mint.toBuffer(), Buffer.from("vault")],
  RTP_PROGRAM_ID,
);`}</CodeBlock>
          </>
        ),
      },
    ],
  },
  {
    label: "Architecture",
    items: [
      {
        slug: "treasury-pda",
        title: "Treasury PDA",
        content: (
          <>
            <p>Every token registered with RTP gets its own treasury — a <strong>program-derived address (PDA)</strong> that owns the vault where fees accumulate. The PDA has no private key; it&apos;s controlled exclusively by the on-chain program.</p>

            <h3>Key Properties</h3>
            <ul>
              <li><strong>PDA Ownership</strong> — no private key exists. No one can sign funds away from the treasury.</li>
              <li><strong>CPI-Only Transfers</strong> — all token movements are Cross-Program Invocations, atomic and verifiable.</li>
              <li><strong>Per-Mint Isolation</strong> — each token gets its own treasury, vault, and adopter record.</li>
              <li><strong>Deterministic Derivation</strong> — anyone can derive the PDA from the mint address.</li>
            </ul>

            <h3>Trust Model</h3>
            <h4>Authority-Gated Actions</h4>
            <p>These require the treasury authority&apos;s signature:</p>
            <ul>
              <li><code>initialize</code> — creates treasury</li>
              <li><code>evolve_phase</code> — irreversible phase transitions</li>
              <li><code>register_strategy</code> — promotes strategy to Live</li>
              <li><code>end_beta</code> — ends beta participation</li>
            </ul>

            <h4>Permissionless Actions</h4>
            <p>Anyone can call these — they move funds INTO the PDA (never out) or record state:</p>
            <ul>
              <li><code>withdraw_fees</code> — pulls fees into vault</li>
              <li><code>check_redistribute</code> — triggers 70/20/10 split (deterministic)</li>
              <li><code>hydrate_swarm</code> — proposes funding (gated by strategy status)</li>
            </ul>

            <h3>Program Deployment</h3>
            <Table
              headers={["Network", "Program ID"]}
              rows={[
                ["Devnet", "<code>8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB</code>"],
              ]}
            />
            <p><a href="https://explorer.solana.com/address/FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF?cluster=devnet" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>View demo treasury on Solana Explorer</a></p>
          </>
        ),
      },
      {
        slug: "fee-routing",
        title: "Fee Routing",
        content: (
          <>
            <p>Every RTP-registered token follows the same cycle: trading fees accumulate in the treasury vault, the swarm generates yield, and yield returns for redistribution.</p>

            <h3>Yield Generation from Fees</h3>
            <ol>
              <li><strong>Bridge to Hyperliquid</strong> — SOL converted to USDC via Phantom bridge (0.3% fee)</li>
              <li><strong>Execute strategy</strong> — Trading Wing runs validated SOL/USDT strategy on HL perps</li>
              <li><strong>Collect yield</strong> — USDC profit from closed positions</li>
              <li><strong>Bridge back</strong> — USDC converted back to SOL via Phantom bridge</li>
              <li><strong>Deposit to treasury</strong> — SOL returned to the treasury PDA</li>
            </ol>

            <h3>Capital Safety</h3>
            <ul>
              <li><strong>USDC-margined only</strong> — Hyperliquid positions use USDC, not SOL. SOL is never at risk of liquidation.</li>
              <li><strong>Max 20% position size</strong> — no single trade risks more than 20% of treasury reserves</li>
              <li><strong>Fee-only capital</strong> — the swarm only trades with fee revenue, never with user deposits</li>
            </ul>
          </>
        ),
      },
      {
        slug: "swarm-execution",
        title: "Swarm Execution",
        content: (
          <>
            <p>RTP&apos;s swarm runtime is a Rust-based multi-wing agent system. The Trading Wing is the only wing that touches capital.</p>

            <h3>Trading Wing Lifecycle</h3>
            <ol>
              <li><strong>Receive strategy</strong> — Coordinator delivers validated config from research layer</li>
              <li><strong>Build order</strong> — Constructs EIP-712 signed order payload for Hyperliquid</li>
              <li><strong>Submit</strong> — Sends REST API call to HL testnet</li>
              <li><strong>Track position</strong> — Monitors fills, computes PnL on close</li>
              <li><strong>Return yield</strong> — Bridges USDC profit back to SOL, deposits to treasury PDA</li>
            </ol>

            <h3>Swarm Wings</h3>
            <Table
              headers={["Wing", "Purpose"]}
              rows={[
                ["Trading", "Yield generation + Hyperliquid execution"],
                ["Security", "Threat detection, rate-limiting"],
                ["Evolve", "Self-modification, adaptation, rollback"],
                ["Knowledge", "In-memory knowledge graph"],
                ["Audit", "3-agent tribunal (Skeptic/UserProxy/Optimizer)"],
                ["Future-proof", "Deprecation monitoring, quantum readiness"],
              ]}
            />

            <h3>Validated Strategy</h3>
            <p>Top validated strategy (SOL/USDT):</p>
            <ul>
              <li>Survivor score: 2.69</li>
              <li>OOS Sharpe: +3.96</li>
              <li>Consistency: 100% (9/9 folds profitable)</li>
              <li>Config: signal_threshold=0.3, tp_atr=3.0, sl_atr=1.5, max_hold=36h</li>
            </ul>
          </>
        ),
      },
      {
        slug: "redistribution",
        title: "Redistribution",
        content: (
          <>
            <p>When the treasury vault accumulates enough yield, the on-chain program automatically redistributes funds:</p>
            <Table
              headers={["Recipient", "Share", "Purpose"]}
              rows={[
                ["Holders", "70%", "Direct value return to token holders"],
                ["Project Dev", "20%", "Sustains project development"],
                ["Ecosystem", "10%", "Funds broader ecosystem growth"],
              ]}
            />

            <h3>On-Chain Mechanics</h3>
            <ol>
              <li><strong>Threshold check</strong> — vault balance must exceed <code>min_runway_balance</code></li>
              <li><strong>Calculate splits</strong> — 70%, 20%, 10% of the surplus</li>
              <li><strong>SPL transfers</strong> — atomic CPI transfers to each recipient&apos;s ATA</li>
              <li><strong>Emit event</strong> — <code>Redistribution</code> event logged for auditability</li>
              <li><strong>Update counters</strong> — cumulative totals updated with <code>saturating_add</code></li>
            </ol>

            <h3>Phase Evolution</h3>
            <Table
              headers={["Phase", "Trigger", "Behavior"]}
              rows={[
                ["Sustenance", "Default", "Build runway, fund swarm operations"],
                ["Ecosystem", "Vault > threshold", "Auto-invest in top RTP token LPs"],
                ["Humanity", "Treasury > large threshold", "Fund public goods"],
              ]}
            />
            <p>Phase transitions are <strong>irreversible</strong> — enforced on-chain by the Anchor program.</p>

            <p><a href="https://explorer.solana.com/tx/9HzWgBfwYxs5ModdjF5mT6gdTfayQq8mMYipopyHfGPmYqk6KESHFqgDrc9Mcie573ttcdPqMHSyJP5nNBKK3bR?cluster=devnet" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>View demo redistribution tx on Solana Explorer</a></p>
          </>
        ),
      },
    ],
  },
  {
    label: "Security",
    items: [
      {
        slug: "security",
        title: "Security Model",
        content: (
          <>
            <h3>Multi-Layer Security</h3>
            <Table
              headers={["Layer", "Mechanism", "Scope"]}
              rows={[
                ["On-chain", "Anchor constraints, PDA authority checks", "All fund movements"],
                ["Runtime", "soulguard.rs validates against invariants", "All swarm operations"],
                ["Execution", "Trading Wing position limits", "Capital deployment"],
                ["Governance", "soulcontract.md — constitutional invariants", "All protocol changes"],
              ]}
            />

            <h3>On-Chain Invariants</h3>
            <ol>
              <li><strong>PDA owns treasury</strong> — no private key can sign funds away</li>
              <li><strong>CPI-only transfers</strong> — all movements are atomic, program-controlled</li>
              <li><strong>Phase transitions irreversible</strong> — Sustenance → Ecosystem → Humanity</li>
              <li><strong>Self-hydration gated</strong> — ops funding only if sustenance &gt; 90-day runway</li>
              <li><strong>Counters saturate</strong> — <code>saturating_add</code> prevents overflow</li>
            </ol>

            <h3>Soulcontract Governance</h3>
            <ul>
              <li><strong>No SOL liquidation</strong> — SOL reserves are never sold</li>
              <li><strong>Max 20% position size</strong> — per-trade risk limit</li>
              <li><strong>Agent proposes, human approves</strong> — irreversible actions need sign-off</li>
              <li><strong>Auto-rollback</strong> — if performance drops &gt; 5% post-amendment</li>
              <li><strong>24h monitoring</strong> — no instant self-modification of governance</li>
            </ul>

            <Callout type="info" title="Full Audit">
              <p><a href="https://github.com/tradewife/resilient-token-protocol/blob/main/docs/AUDIT-COPILOT-WK-1.md" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>Read the security audit report</a></p>
            </Callout>
          </>
        ),
      },
    ],
  },
];

// Flatten all sections for lookup
const ALL_SECTIONS = DOC_GROUPS.flatMap((g) => g.items);

export default function DocsPage() {
  const [activeSlug, setActiveSlug] = useState("overview");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const activeSection = ALL_SECTIONS.find((s) => s.slug === activeSlug) || ALL_SECTIONS[0];

  useEffect(() => {
    const hash = window.location.hash.replace("#", "");
    if (hash) {
      const match = ALL_SECTIONS.find((s) => s.slug === hash);
      if (match) setActiveSlug(hash);
    }
  }, []);

  const handleNav = (slug: string) => {
    setActiveSlug(slug);
    setSidebarOpen(false);
    window.location.hash = slug;
  };

  return (
    <div className="page">
      <Topbar activePage="docs" />

      {/* Mobile sidebar toggle */}
      <button
        className="docs-sidebar-toggle"
        onClick={() => setSidebarOpen(!sidebarOpen)}
        aria-label="Toggle docs sidebar"
      >
        {sidebarOpen ? "\u2715" : "\u2630"} Docs
      </button>

      <div className="docs-layout">
        {/* Sidebar */}
        <nav className={`docs-sidebar${sidebarOpen ? " open" : ""}`}>
          {DOC_GROUPS.map((group) => (
            <div key={group.label} className="docs-nav-group">
              <div className="docs-nav-group-label">{group.label}</div>
              {group.items.map((item) => (
                <button
                  key={item.slug}
                  className={`docs-nav-item${activeSlug === item.slug ? " active" : ""}`}
                  onClick={() => handleNav(item.slug)}
                >
                  {item.title}
                </button>
              ))}
            </div>
          ))}
        </nav>

        {/* Content */}
        <main className="docs-content">
          <article className="docs-article">
            <h1 className="docs-title">{activeSection.title}</h1>
            <div className="docs-body">{activeSection.content}</div>
          </article>

          {/* Footer nav */}
          <div className="docs-footer-nav">
            {(() => {
              const idx = ALL_SECTIONS.findIndex((s) => s.slug === activeSlug);
              const prev = idx > 0 ? ALL_SECTIONS[idx - 1] : null;
              const next = idx < ALL_SECTIONS.length - 1 ? ALL_SECTIONS[idx + 1] : null;
              return (
                <>
                  {prev && (
                    <button className="docs-nav-btn" onClick={() => handleNav(prev.slug)}>
                      <span className="docs-nav-btn-dir">&larr; Previous</span>
                      <span className="docs-nav-btn-label">{prev.title}</span>
                    </button>
                  )}
                  <div style={{ flex: 1 }} />
                  {next && (
                    <button className="docs-nav-btn docs-nav-btn-next" onClick={() => handleNav(next.slug)}>
                      <span className="docs-nav-btn-dir">Next &rarr;</span>
                      <span className="docs-nav-btn-label">{next.title}</span>
                    </button>
                  )}
                </>
              );
            })()}
          </div>
        </main>
      </div>

      {/* Footer */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">No RTP Token</span>
          <span className="vital-label">Protocol</span>
        </div>
        <div className="vital">
          <span className="vital-value">MIT</span>
          <span className="vital-label">License</span>
        </div>
        <div className="vital">
          <span className="vital-value">Per-mint PDA</span>
          <span className="vital-label">Treasury</span>
        </div>
        <Link href="/" className="vital-link">Back to Dashboard</Link>
      </footer>
    </div>
  );
}
