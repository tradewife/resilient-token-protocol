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
            <p>RTP (Resilient Token Protocol) builds <strong>bespoke trading engines</strong> on Solana. One client, one strategy, engineered around your specifics — risk budget, drawdown limit, accumulation target, horizon — and run on self-custodied, on-chain-verifiable rails. There is no RTP token. RTP is a service backed by infrastructure.</p>

            <h3>What We Are Not</h3>
            <ul>
              <li><strong>Not investment advice.</strong> We do not manage your money and we do not tell you what to buy. We deliver research output, validated configurations, and execution rails. You keep custody and every decision.</li>
              <li><strong>Not a fund or vault product.</strong> Your capital never touches ours. It stays in your wallet; the engine trades through scoped permission you can revoke instantly.</li>
              <li><strong>Not loyal to any venue.</strong> No venue pays us. Every engine is priced at fees we measure on-chain ourselves, and we migrate venues when measurement says so.</li>
            </ul>

            <Callout type="tip" title="The onboarding we wish we had">
              <p>When we first got into the trenches, nobody told us where the traps were: venues that shut down overnight, fee schedules that silently eat edges, custody mistakes that cost everything. RTP is that onboarding, built as a service — engineered around lessons paid for in the market.</p>
            </Callout>

            <h3>The Service</h3>
            <ol>
              <li><strong>Intake:</strong> you state capital size, max drawdown (hard limit), horizon, constraints — ten minutes, structured</li>
              <li><strong>Build:</strong> the research pipeline engineers a strategy around your terms — not from a template library</li>
              <li><strong>Validate:</strong> ten fixed gates, identical for every client, run at venue fees measured on-chain — never docs, never assumptions</li>
              <li><strong>Verdict:</strong> written pass / conditional / fail with full configuration, machine-readable and independently verifiable</li>
              <li><strong>Debrief:</strong> 45–60 minutes walking through the verdict and the risk envelope</li>
            </ol>

            <h3>Built For A Fast-Moving Ecosystem</h3>
            <p>Solana perps venues change fee schedules, mechanics, and even existence on short notice. In August 2026 our execution venue (Flash Trade) announced a wind-down — our response was a same-session re-validation of the live engine on the replacement venue&apos;s on-chain-measured costs, passing all ten gates. That loop — measure, validate, migrate, document — is the product. The ecosystem evolves; the engine keeps its edge because the cost basis is always current.</p>

            <h3>The PDA System, In Plain Terms</h3>
            <p>Client capital sits behind a <strong>PDA</strong> — a program-derived address. A vault with no keys. It is controlled by code, not people: no password to steal, no insider with access, no signature that can be forged. Funds only move when the on-chain rules allow it, and those rules — position limits, drawdown stops, emergency halt — are enforced the same way for everyone. You retain custody of your wallet and hold the kill switch: execution runs through permission you grant, and revoking it stops the engine instantly.</p>

            <h3>Why RTP, Not a Multisig or Yield Aggregator?</h3>
            <Table
              headers={["Dimension", "Squads Multisig", "Yield Aggregator", "RTP"]}
              rows={[
                ["Who controls funds?", "Multi-sig signers (humans)", "Smart contract (immutable)", "You — self-custody with scoped execution permission"],
                ["Strategy", "Manual / none", "Preset, shared by all users", "Engineered per client; no shared edges"],
                ["Cost model", "None", "Assumed", "Measured on-chain per venue, re-verified on migration"],
                ["Venue risk", "None (no yield)", "Protocol risk", "Measured + migration-capable; venue health monitored"],
                ["Trust model", "Trust the signers", "Trust the contract", "Trust the program + audit every transaction"],
                ["Adaptation", "None", "None", "Nightly research (30K configs), LLM evolution, memory"],
              ]}
            />

            <h3>What&apos;s Been Built</h3>
            <Table
              headers={["Component", "Status", "Detail"]}
              rows={[
                ["Live blueprint", "✅ Real capital since May 2026", "SOL/USDT Survivor 2.69 — the specimen engine, every trade on-chain"],
                ["Venue migration", "✅ Proven live (Aug 2026)", "Flash wind-down → GMTrade re-validation on measured on-chain costs, 10/10 gates"],
                ["Research pipeline", "✅ Nightly", "30K configs/night, 9-fold walk-forward, Darwinian evolution"],
                ["Gate suite", "✅ Fixed across clients", "10 gates at measured costs; the standard every engine clears"],
                ["Rust swarm runtime", "✅ 325 tests passing", "6 wings: Trading, Security, Evolve, Knowledge, Audit, Futureproof"],
                ["Anchor treasury program", "✅ Deployed", "PDA-owned vault, 19 instructions, constitutional constraints on-chain"],
                ["Token treasury heritage", "✅ Shipped", "Per-token PDAs + SDK for token projects (below)"],
                ["Dashboard", "✅ Live", "<a href='https://resilientprotocol.xyz' style='color: var(--coral)'>resilientprotocol.xyz</a>: live engine state, wallet connect"],
              ]}
            />

            <Callout type="info" title="Token project or platform?">
              <p>RTP began as token-treasury infrastructure and that path remains open: per-token PDAs, fee routing from launchpads, SDK integration. See <a href="#getting-started-creators" style={{ color: "var(--coral)" }}>Getting Started — Token Projects</a> and <a href="#getting-started-platforms" style={{ color: "var(--coral)" }}>Getting Started — Platforms</a>.</p>
            </Callout>
          </>
        ),
      },
      {
        slug: "diagnostic",
        title: "On-Chain Compatibility Check",
        content: (
          <>
            <p>The <strong>On-Chain Compatibility Check</strong> is the entry point: a 90-second scorecard that maps custody posture, patience, horizon, and build path, then forks you to the right next step.</p>

            <h3>Two paths after the check</h3>
            <ul>
              <li><strong>Advisory build (A$4,500 one-time).</strong> Structured intake, dedicated engine around your terms, ten-gate validation at measured venue fees, paper-traded report + full config, and up to 4× 45–60 min implementation consultations. Limited to 3–4 concurrent engagements so edges stay uncrowded.</li>
              <li><strong>Self-serve developer docs.</strong> Open specifications, explorer wallet ledger, and the live specimen dashboard — no engagement required.</li>
            </ul>

            <h3>Advisory deliverables</h3>
            <ul>
              <li>Bespoke strategy configuration built around your terms</li>
              <li>Full ten-gate validation at current, on-chain-measured venue fees</li>
              <li>Written verdict (pass / conditional / fail) with supporting analysis</li>
              <li>Machine-readable config and risk report you can verify independently</li>
              <li>Up to 4× 45–60 minute 1-on-1 implementation consultations</li>
            </ul>

            <h3>Advisory terms</h3>
            <Table
              headers={["Term", "Value"]}
              rows={[
                ["Price", "A$4,500 one-time"],
                ["Slots", "3–4 at any time, strictly limited"],
                ["Turnaround", "Initial report typically 5–8 business days after intake"],
                ["Consultations", "Up to 4 × 45–60 min included"],
                ["Capital", "None — paper verdict only"],
                ["Custody", "100% yours, throughout"],
                ["Advisory", "None — research and infrastructure output only"],
              ]}
            />

            <Callout type="info" title="Regulatory framing">
              <p>This is research and infrastructure output — not discretionary management and not financial advice. Live deployment, if a client wants it, is a separate conversation after the paper verdict is accepted.</p>
            </Callout>

            <p><a href="/diagnostic" style={{ color: "var(--coral)" }}>Start the Compatibility Check →</a></p>
          </>
        ),
      },
      {
        slug: "getting-started-creators",
        title: "Getting Started — Token Projects",
        content: (
          <>
            <p>Two ways to get an RTP treasury for your token: launch from our site, or register an existing token.</p>

            <h3>Path A: Launch From RTP</h3>
            <p>Use the <a href="/launch" style={{ color: "var(--coral)" }}>RTP launch page</a>. Pick your platform, fill in token details, sign with your Solana wallet. The token goes live on-chain and the RTP treasury is initialized automatically.</p>
            <ol>
              <li><strong>Pick a platform:</strong> Pump.fun, Bags.fm, or Raydium LaunchLab</li>
              <li><strong>Fill in token details:</strong> name, symbol, image, description</li>
              <li><strong>Sign with your wallet:</strong> one transaction creates the token on-chain</li>
              <li><strong>RTP treasury auto-initializes:</strong> treasury PDA and adopter record created</li>
            </ol>

            <h3>Path B: Register An Existing Token</h3>
            <p>Already have a token on Pump.fun, Bags.fm, or Raydium? Register it with RTP programmatically:</p>
            <CodeBlock>{`import { registerWithRTP } from "@resilient-protocol/sdk";
import { Connection, PublicKey } from "@solana/web3.js";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const result = await registerWithRTP(connection, wallet, {
  authority: publicKey,
});

// result.treasuryPDA: your token's treasury
// result.adopterPDA: adopter record`}</CodeBlock>

            <h3>After Registration: What Happens</h3>
            <ol>
              <li><strong>Fees accumulate:</strong> trading fees flow into your treasury PDA as native SOL</li>
              <li><strong>Swarm activates:</strong> Treasury PDA executes validated strategies on Solana-native perps (invoke_signed)</li>
              <li><strong>Yield returns:</strong> generated yield flows back to the treasury PDA</li>
              <li><strong>Redistribution:</strong> 70% to holders, 20% to project dev, 10% to ecosystem (enforced on-chain)</li>
            </ol>

            <h3>Fee Routing Per Platform</h3>
            <p>Trading fees must reach the RTP treasury. The mechanism differs per platform:</p>
            <Table
              headers={["Platform", "Can route to RTP?", "Changeable?", "How"]}
              rows={[
                ["Pump.fun", "Yes", "Once only", "One-time fee redirect to treasury PDA. RTP keeper claims from deployer wallet."],
                ["Bags.fm", "Yes", "Anytime", "Set treasury PDA as fee claimer. Update claimers anytime via admin API."],
                ["Raydium", "Yes", "Limited", "Creator fees go to pool_creator wallet. Forward to RTP manually or via platform redirect."],
              ]}
            />

            <Callout type="info" title="Pump.fun One-Time Redirect">
              <p>Pump.fun allows only <strong>one</strong> post-launch fee redirect per token. If you redirect to the RTP treasury PDA, that&apos;s your one shot; make it count. After redirecting, the RTP keeper handles claiming and forwarding automatically.</p>
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
              <li>A Solana wallet (for signing transactions)</li>
            </ul>

            <h3>Installation</h3>
            <CodeBlock>{`npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor`}</CodeBlock>

            <h3>Three-Step Integration</h3>

            <h4>Step 1: Register the token with RTP</h4>
            <p>After your platform creates a token, call <code>registerWithRTP</code>:</p>
            <CodeBlock>{`import { registerWithRTP } from "@resilient-protocol/sdk";

const result = await registerWithRTP(connection, wallet, {
  authority: publicKey,
  holdersWallet: publicKey,
  projectDevWallet: publicKey,
  ecosystemWallet: publicKey,
});`}</CodeBlock>
            <p>This creates the treasury PDA and adopter record on-chain.</p>

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

const state = await fetchTreasuryState(connection, rtpResult.authority);
// state.phase, state.solBalance, state.availableSolLamports, state.totalFeesWithdrawn`}</CodeBlock>

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
              <li><code>deposit_sol</code> records deposit on-chain</li>
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
  authority: publicKey,
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
              <p>No direct PDA routing; fees go to deployer wallet. RTP keeper handles claiming. Mainnet only. No API key required.</p>
            </Callout>
          </>
        ),
      },
      {
        slug: "bags-fm",
        title: "Bags.fm",
        content: (
          <>
            <p>Bags.fm runs on Meteora DLMM pools. Its standout feature for RTP is <strong>multi-claimer fee sharing:</strong> up to 100 fee claimers with custom basis point splits. Set the RTP treasury PDA as a fee claimer and fees route there automatically.</p>

            <h3>How It Works</h3>
            <ol>
              <li>Trader buys/sells → Meteora DLMM collects fees</li>
              <li>Fee share API splits → 70% treasury PDA, 30% project wallet</li>
              <li>Fees arrive directly in the vault (no keeper needed)</li>
            </ol>

            <h3>Configure Fee Sharing with RTP</h3>
            <CodeBlock>{`const rtpResult = await registerWithRTP(connection, wallet, {
  authority: publicKey,
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
            <p>Raydium is the #1 DEX/AMM on Solana by TVL. LaunchLab supports explicit PDA-based fee collection, the best architectural fit for RTP. Full TypeScript SDK with <code>@raydium-io/raydium-sdk-v2</code>.</p>

            <h3>Why Raydium</h3>
            <ul>
              <li><strong>Explicit PDA fee collection:</strong> documented and recommended by Raydium</li>
              <li><code>updatePlatformCpCreator</code> redirects all creator fees to a PDA</li>
              <li><code>collectMultiCreatorFees</code> for batch collection</li>
              <li><strong>Devnet support</strong> with sUSDC as quote token</li>
              <li><strong>Largest DEX:</strong> maximum liquidity access</li>
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
  authority: publicKey,
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
            <Callout type="warning" title="Planned Feature: Not Yet Available">
              <p>The Enterprise API is a planned REST API for launchpads that prefer not to run chain operations themselves. It will offer the same functionality as the SDK via simple HTTP calls: <code>POST /v1/adopt</code>, <code>POST /v1/fee-route</code>, <code>GET /v1/treasury/:authority</code>, <code>POST /v1/crank</code>.</p>
              <p style={{ marginTop: 8 }}>Today, use the <a href="#sdk-reference" style={{ color: "var(--coral)" }}>TypeScript SDK</a> for direct integration. Core functions: <code>registerWithRTP</code>, <code>fetchTreasuryState</code>, <code>depositSol</code>, <code>checkRedistribute</code>.</p>
            </Callout>

            <Table
              headers={["Feature", "SDK (Available Now)", "Enterprise API (Planned)"]}
              rows={[
                ["Signing", "Your wallet", "RTP-managed"],
                ["Chain ops", "Your infra", "RTP infra"],
                ["Latency", "Direct", "~200ms overhead"],
                ["Setup", "npm install", "HTTP calls"],
                ["Best for", "Full control", "Simple integration"],
              ]}
            />

            <Callout type="info" title="Want Early Access?">
              <p>Reach out to the RTP team to discuss Enterprise API integration.</p>
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
            <p>The <code>@resilient-protocol/sdk</code> TypeScript package is for launchpads and platforms integrating RTP directly. If you&apos;re a token creator, your launchpad handles this. See <a href="#getting-started-creators" style={{ color: "var(--coral)" }}>Getting Started for Token Creators</a>.</p>

            <h3>Installation</h3>
            <CodeBlock>{`npm install @resilient-protocol/sdk @solana/web3.js @coral-xyz/anchor`}</CodeBlock>

            <h3>Constants</h3>
            <CodeBlock>{`import { RTP_PROGRAM_ID, RTP_DEVNET_RPC, RTP_MAINNET_RPC } from "@resilient-protocol/sdk";

RTP_PROGRAM_ID  // PublicKey("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB")
RTP_DEVNET_RPC  // "https://api.devnet.solana.com"
RTP_MAINNET_RPC // "https://api.mainnet-beta.solana.com"`}</CodeBlock>

            <h3>Types</h3>

            <h4>RTPRegistrationConfig</h4>
            <CodeBlock>{`interface RTPRegistrationConfig {
  authority: PublicKey;                      // Treasury owner (used as PDA seed)
  holdersWallet?: PublicKey;                // 70% recipient (default: payer)
  projectDevWallet?: PublicKey;             // 20% recipient (default: payer)
  ecosystemWallet?: PublicKey;              // 10% recipient (default: payer)
  minRunwayBalance?: number;                // Min runway in lamports (default: 10_000_000)
}`}</CodeBlock>

            <h4>RTPRegistrationResult</h4>
            <CodeBlock>{`interface RTPRegistrationResult {
  authority: string;     // base58 authority pubkey
  signature: string;      // registration tx signature
  explorerUrl: string;    // Solana Explorer link
  treasuryPDA: string;    // Per-token treasury state account
  adopterPDA: string;     // Adopter registration account
}`}</CodeBlock>

            <h4>TreasuryState</h4>
            <CodeBlock>{`interface TreasuryState {
  authority: string;
  phase: "Sustenance" | "Ecosystem" | "Humanity";
  isFrozen: boolean;
  solBalance: number;          // native SOL lamports
  committedSolLamports: number; // committed to open perp positions
  availableSolLamports: number; // solBalance - committed - rent_exempt
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
            <p>Initializes a new treasury PDA for the given authority. Creates treasury PDA and adopter record.</p>
            <CodeBlock>{`async function registerWithRTP(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  config: RTPRegistrationConfig,
): Promise<RTPRegistrationResult>;`}</CodeBlock>

            <h4>fetchTreasuryState()</h4>
            <p>Read-only. Fetches on-chain treasury state. No signing required.</p>
            <CodeBlock>{`async function fetchTreasuryState(
  connection: Connection,
  authorityAddress: string | PublicKey,
): Promise<TreasuryState>;`}</CodeBlock>

            <h4>depositSol()</h4>
            <p>Permissionless. Deposits native SOL into the treasury.</p>
            <CodeBlock>{`async function depositSol(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  amountLamports: number,
): Promise<{ signature: string }>;`}</CodeBlock>

            <h4>checkRedistribute()</h4>
            <p>Permissionless crank. Triggers 70/20/10 redistribution if above threshold.</p>
            <CodeBlock>{`async function checkRedistribute(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
): Promise<{ redistributeSig?: string }>;`}</CodeBlock>

            <h4>registerAdopterBeta()</h4>
            <p>Registers a beta adopter with an expiry timestamp. Free until beta period ends.</p>
            <CodeBlock>{`async function registerAdopterBeta(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  adopterId: string,
  betaExpiresAt: number,
): Promise<{ signature: string; adopterPDA: string }>;`}</CodeBlock>

            <h4>fetchAdopterState()</h4>
            <p>Read-only. Fetches on-chain adopter record (beta status, fee contributions).</p>
            <CodeBlock>{`async function fetchAdopterState(
  connection: Connection,
  authorityAddress: string | PublicKey,
  adopterId: string,
): Promise<AdopterState>;`}</CodeBlock>

            <h4>freezeTreasury()</h4>
            <p>Authority-gated emergency freeze. Halts all state-mutating operations. No time lock (emergency speed).</p>
            <CodeBlock>{`async function freezeTreasury(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
): Promise<{ signature: string }>;`}</CodeBlock>

            <h4>unfreezeTreasury()</h4>
            <p>Authority-gated unfreeze. Resumes operations. Post-launch: Squads 2-of-3 + 24h time lock.</p>
            <CodeBlock>{`async function unfreezeTreasury(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
): Promise<{ signature: string }>;`}</CodeBlock>

            <h3>PDA Derivation</h3>
            <CodeBlock>{`// Treasury PDA (authority-seeded)
PublicKey.findProgramAddressSync(
  [Buffer.from("treasury"), authority.toBuffer()],
  RTP_PROGRAM_ID,
);

// Adopter PDA (adopter_id is any string)
PublicKey.findProgramAddressSync(
  [Buffer.from("adopter"), treasuryPDA.toBuffer(), Buffer.from(adopterId)],
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
            <p>Every token registered with RTP gets its own treasury, a <strong>program-derived address (PDA)</strong> where SOL fees accumulate. The PDA has no private key; it&apos;s controlled exclusively by the on-chain program.</p>

            <h3>Key Properties</h3>
            <ul>
              <li><strong>PDA Ownership:</strong> no private key exists. No one can sign funds away from the treasury.</li>
              <li><strong>CPI-Only Transfers:</strong> all SOL movements are Cross-Program Invocations, atomic and verifiable.</li>
              <li><strong>Per-Token Isolation:</strong> each token gets its own Treasury PDA and vault. Seeds: <code>[&quot;treasury&quot;, authority]</code>. One token&apos;s reserves are invisible to every other PDA.</li>
              <li><strong>Deterministic Derivation:</strong> anyone can derive the PDA from the authority pubkey.</li>
              <li><strong>No shared pool:</strong> there is no single treasury holding all tokens&apos; fees. Each PDA is its own isolated vault. This eliminates the honeypot risk: exploiting one treasury does not expose any other.</li>
            </ul>

            <Callout type="tip" title="Why Per-Token Isolation Matters">
              <p>Aggregating many tokens&apos; fees into a shared pool creates a high-value target. Per-token PDAs mean each treasury is independently secured — one token&apos;s activity can never drain another&apos;s vault. Zero cross-contamination, by construction.</p>
            </Callout>

            <h3>Trust Model</h3>
            <h4>Authority-Gated Actions</h4>
            <p>These require the treasury authority&apos;s signature:</p>
            <ul>
              <li><code>initialize</code>: creates treasury</li>
              <li><code>evolve_phase</code>: irreversible phase transitions</li>
              <li><code>register_strategy</code>: promotes strategy to Live</li>
              <li><code>force_retire_strategy</code>: emergency strategy retirement</li>
              <li><code>end_beta</code>: ends beta participation</li>
              <li><code>freeze_treasury</code>: emergency halt (no time lock)</li>
              <li><code>unfreeze_treasury</code>: resume operations</li>
            </ul>

            <h4>Permissionless Actions</h4>
            <p>Anyone can call these. They move funds INTO the PDA (never out) or record state:</p>
            <ul>
              <li><code>deposit_sol</code>: deposits native SOL into treasury</li>
              <li><code>check_redistribute</code>: triggers 70/20/10 split (deterministic)</li>
              <li><code>hydrate_swarm</code>: proposes funding (gated by strategy status)</li>
            </ul>

            <h3>Program Deployment</h3>
            <Table
              headers={["Network", "Program ID"]}
              rows={[
                ["Devnet", "<code>8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB</code>"],
              ]}
            />
            <p><a href="https://explorer.solana.com/address/6PYPAnwiMoZvzphAWEu3EsNz3PpwjJ6YcZabj34qVQ4Z?cluster=devnet" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>View demo treasury on Solana Explorer</a></p>
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
              <li><strong>Commit SOL via CPI:</strong> Treasury PDA commits SOL via invoke_signed → venue CPI opens position</li>
              <li><strong>Execute strategy:</strong> on-chain Solana perps position opened via venue CPI (Treasury PDA signs)</li>
              <li><strong>Collect yield:</strong> SOL returned when position closes via venue CPI</li>
              <li><strong>Return to treasury:</strong> SOL yield deposited back to the treasury PDA (single chain)</li>
              <li><strong>Deposit to treasury:</strong> SOL returned to the treasury PDA</li>
            </ol>

            <h3>Capital Safety</h3>
            <ul>
              <li><strong>Per-token isolation:</strong> each token&apos;s fees and yield are in a separate PDA. No cross-contamination between tokens.</li>
              <li><strong>SOL throughout:</strong> positions opened with SOL via venue CPI. No cross-chain bridge.</li>
              <li><strong>Max 20% position size:</strong> no single trade risks more than 20% of treasury reserves</li>
              <li><strong>Fee-only capital:</strong> the swarm only trades with fee revenue, never with user deposits</li>
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
              <li><strong>Receive strategy:</strong> Coordinator delivers validated config from research layer</li>
              <li><strong>Build instruction:</strong> Constructs Anchor instruction for open_flash_position (Treasury PDA signer)</li>
              <li><strong>Submit:</strong> invoke_signed via Treasury PDA → venue CPI (on-chain Solana perps)</li>
              <li><strong>Track position:</strong> Monitors fills, computes PnL on close</li>
              <li><strong>Return yield:</strong> close_flash_position returns SOL to treasury PDA (single chain)</li>
            </ol>

            <h3>Swarm Wings</h3>
            <Table
              headers={["Wing", "Purpose"]}
              rows={[
                ["Trading", "Yield generation + venue execution (CPI/API)"],
                ["Security", "Threat detection, rate-limiting"],
                ["Evolve", "Self-modification, adaptation, rollback"],
                ["Knowledge", "Persistent knowledge store (file-backed)"],
                ["Audit", "3-agent tribunal (Skeptic/UserProxy/Optimizer)"],
                ["Future-proof", "Deprecation monitoring, quantum readiness"],
              ]}
            />

            <h3>Validated Strategy</h3>
            <p>Top validated strategy (SOL/USDT):</p>
            <ul>
              <li>Calmar ratio: 44.89</li>
              <li>Compounded return: +554% at 9x leverage</li>
              <li>Consistency: 100% (all WFA folds profitable)</li>
              <li>Config: signal_threshold=0.25, tp_atr=5.0, sl_atr=2.7, trail=0.14, min_alignment=3, leverage=9x</li>
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
              <li><strong>Threshold check:</strong> treasury SOL balance must exceed <code>min_runway_balance</code></li>
              <li><strong>Calculate splits:</strong> 70%, 20%, 10% of the surplus</li>
              <li><strong>Native SOL transfers:</strong> atomic lamport debit from treasury, credit to each recipient wallet</li>
              <li><strong>Emit event:</strong> <code>Redistribution</code> event logged for auditability</li>
              <li><strong>Update counters:</strong> cumulative totals updated with <code>saturating_add</code></li>
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
            <p>Phase transitions are <strong>irreversible</strong>, enforced on-chain by the Anchor program.</p>

            <p><a href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>View demo redistribution tx on Solana Explorer</a></p>
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
                ["Isolation", "Per-token Treasury PDA + vault: no shared pool", "Exploit containment"],
                ["Runtime", "soulguard.rs validates against invariants", "All swarm operations"],
                ["Execution", "Trading Wing position limits", "Capital deployment"],
                ["Governance", "Constitutional invariants (soulcontract)", "All protocol changes"],
              ]}
            />

            <h3>On-Chain Invariants</h3>
            <ol>
              <li><strong>PDA owns treasury:</strong> no private key can sign funds away</li>
              <li><strong>Per-token isolation:</strong> each mint has its own PDA + vault. No shared pool.</li>
              <li><strong>CPI-only transfers:</strong> all movements are atomic, program-controlled</li>
              <li><strong>Phase transitions irreversible:</strong> Sustenance → Ecosystem → Humanity</li>
              <li><strong>Self-hydration gated:</strong> ops funding only if sustenance &gt; 90-day runway</li>
              <li><strong>Counters saturate:</strong> <code>saturating_add</code> prevents overflow</li>
            </ol>

            <h3>Soulcontract Governance</h3>
            <ul>
              <li><strong>No SOL liquidation:</strong> SOL reserves are never sold</li>
              <li><strong>Max 20% position size:</strong> per-trade risk limit</li>
              <li><strong>Auto-rollback:</strong> if performance drops &gt; 5% post-amendment</li>
              <li><strong>24h monitoring:</strong> no instant self-modification of governance</li>
            </ul>

            <Callout type="info" title="Full Audit">
              <p>See the on-chain invariants and enforcement layers detailed below.</p>
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
    const handleHashChange = () => {
      const hash = window.location.hash.replace("#", "");
      if (hash) {
        const match = ALL_SECTIONS.find((s) => s.slug === hash);
        if (match) setActiveSlug(hash);
      }
    };
    handleHashChange();
    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  const handleNav = (slug: string) => {
    setActiveSlug(slug);
    setSidebarOpen(false);
    window.location.hash = slug;
    window.scrollTo({ top: 0, behavior: "smooth" });
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
            <h1 className="docs-title" style={{
              fontFamily: "var(--font-display)",
              fontSize: "clamp(1.375rem, 2.6vw, 2rem)",
              fontWeight: 500,
              letterSpacing: "-0.015em",
              color: "var(--text-primary)",
              lineHeight: 1.15,
              marginBottom: "var(--space-lg)",
            }}>{activeSection.title}</h1>
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
          <span className="vital-value">BSL 1.1</span>
          <span className="vital-label">License</span>
        </div>
        <div className="vital">
          <span className="vital-value">Per-mint PDA</span>
          <span className="vital-label">Treasury</span>
        </div>
        <div className="vital">
          <a className="vital-link" href="https://resilientprotocol.xyz" target="_blank" rel="noopener noreferrer">resilientprotocol.xyz ↗</a>
          <span className="vital-label">Dashboard</span>
        </div>
        <Link href="/" className="vital-link">← Dashboard</Link>
      </footer>
    </div>
  );
}
