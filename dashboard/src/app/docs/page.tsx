"use client";

import React from "react";
import Link from "next/link";

const CODE_BLOCK_STYLE = {
  background: "rgba(0, 0, 0, 0.4)",
  border: "1px solid var(--border)",
  borderRadius: "8px",
  padding: "16px",
  overflowX: "auto" as const,
  marginBottom: "24px",
};

const INSTALL_CODE = `npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor`;

const CREATE_CODE = `import { createRTPToken } from "@resilient-protocol/sdk";
import { Connection, Keypair } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // your launchpad's keypair

const result = await createRTPToken(connection, payer, {
  name: "Community Token",
  symbol: "CMTY",
  supply: 1_000_000_000,     // total supply in tokens
  feeBps: 200,               // 2% transfer fee → treasury vault
  holdersWallet: payer.publicKey,    // optional, defaults to payer
  projectDevWallet: payer.publicKey, // optional, defaults to payer
  ecosystemWallet: payer.publicKey,  // optional, defaults to payer
});

console.log("Mint:", result.mint);
console.log("Treasury PDA:", result.treasuryPDA);
console.log("Vault PDA:", result.vaultPDA);`;

const FETCH_CODE = `import { fetchTreasuryState } from "@resilient-protocol/sdk";

const state = await fetchTreasuryState(connection, result.mint);

console.log("Phase:", state.phase);               // "Sustenance" | "Ecosystem" | "Humanity"
console.log("Vault balance:", state.vaultBalance);  // token units
console.log("Total distributed (holders):", state.totalDistributedHolders);
console.log("Total fees received:", state.totalFeesReceived);`;

const CRANK_CODE = `import { withdrawAndRedistribute } from "@resilient-protocol/sdk";

const { withdrawSig, redistributeSig } = await withdrawAndRedistribute(
  connection,
  payer,
  result.mint,
);

console.log("Fees withdrawn:", withdrawSig);
if (redistributeSig) console.log("Redistributed 70/20/10:", redistributeSig);
// redistributeSig is undefined if vault balance < minRunwayBalance`;

const CONSTANTS_TABLE = [
  ["RTP_PROGRAM_ID", "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"],
  ["RTP_DEVNET_RPC", "https://api.devnet.solana.com"],
  ["RTP_MAINNET_RPC", "https://api.mainnet-beta.solana.com"],
];

interface CodeBlockProps {
  code: string;
}

function CodeBlock({ code }: CodeBlockProps) {
  return (
    <div style={CODE_BLOCK_STYLE}>
      <pre style={{ margin: 0 }}>
        <code style={{
          fontFamily: '"SF Mono", "Fira Code", monospace',
          fontSize: "0.8125rem",
          color: "var(--text-secondary)",
          lineHeight: 1.65,
          whiteSpace: "pre",
        }}>
          {code}
        </code>
      </pre>
    </div>
  );
}

export default function DocsPage() {
  return (
    <div className="page">
      {/* ── Top bar ────────────────────────────────────────── */}
      <header className="topbar">
        <div className="brand">
          <img className="brand-icon" src="/icon.svg" alt="RTP" />
          <Link href="/" className="brand-name" style={{ textDecoration: "none", color: "inherit" }}>
            RESILIENT TOKEN PROTOCOL
          </Link>
        </div>
        <div className="topbar-actions">
          <span className="network-badge">Devnet</span>
          <Link href="/docs" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px", borderColor: "var(--coral-dim)", color: "var(--coral)" }}>
            Docs
          </Link>
          <Link href="/research" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Research
          </Link>
          <Link href="/launch" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Launch
          </Link>
          <Link href="/" className="btn-connect" style={{ textDecoration: "none" }}>
            Dashboard
          </Link>
        </div>
      </header>

      {/* ── Hero ──────────────────────────────────────────── */}
      <section className="launch-hero">
        <h1 className="launch-title">SDK Documentation</h1>
        <p className="launch-subtitle">
          Add RTP to your launch flow in one function call. Every token you launch
          gets an autonomous yield treasury enforced by Solana code.
        </p>
      </section>

      {/* ── Content ───────────────────────────────────────── */}
      <section className="docs-content">

        {/* Install */}
        <div className="docs-section">
          <h2 className="section-title">Install</h2>
          <CodeBlock code={INSTALL_CODE} />
        </div>

        {/* createRTPToken */}
        <div className="docs-section">
          <h2 className="section-title">createRTPToken()</h2>
          <p className="docs-body">
            Creates a Token-2022 mint with transfer fees routing to a per-mint treasury vault PDA,
            initializes the on-chain treasury program, and mints the initial supply.
            One call — your token now has an autonomous treasury.
          </p>
          <CodeBlock code={CREATE_CODE} />
          <div className="docs-returns">
            <span className="docs-returns-label">Returns</span>
            <div className="docs-returns-grid">
              <div className="docs-return-item">
                <code>result.mint</code>
                <span className="docs-return-type">string</span>
                <span className="docs-return-desc">Token-2022 mint address</span>
              </div>
              <div className="docs-return-item">
                <code>result.treasuryPDA</code>
                <span className="docs-return-type">string</span>
                <span className="docs-return-desc">On-chain treasury state account (per-mint)</span>
              </div>
              <div className="docs-return-item">
                <code>result.vaultPDA</code>
                <span className="docs-return-type">string</span>
                <span className="docs-return-desc">Treasury vault token account (per-mint)</span>
              </div>
              <div className="docs-return-item">
                <code>result.signature</code>
                <span className="docs-return-type">string</span>
                <span className="docs-return-desc">Mint creation transaction signature</span>
              </div>
              <div className="docs-return-item">
                <code>result.explorerUrl</code>
                <span className="docs-return-type">string</span>
                <span className="docs-return-desc">Solana Explorer link</span>
              </div>
            </div>
          </div>
        </div>

        {/* fetchTreasuryState */}
        <div className="docs-section">
          <h2 className="section-title">fetchTreasuryState()</h2>
          <p className="docs-body">
            Read-only. No transactions, no signing. Returns the on-chain treasury state
            for any mint that has adopted RTP. Use this to power your token dashboard.
          </p>
          <CodeBlock code={FETCH_CODE} />
          <div className="docs-returns">
            <span className="docs-returns-label">TreasuryState Fields</span>
            <div className="docs-returns-grid">
              {[
                ["phase", '"Sustenance" | "Ecosystem" | "Humanity"', "Current protocol phase"],
                ["vaultBalance", "number", "Tokens in the treasury vault"],
                ["totalFeesWithdrawn", "number", "Cumulative fees swept from mint"],
                ["totalDistributedHolders", "number", "Cumulative 70% holder distributions"],
                ["totalDistributedDev", "number", "Cumulative 20% dev distributions"],
                ["totalDistributedEcosystem", "number", "Cumulative 10% ecosystem distributions"],
                ["totalFeesReceived", "number", "Total fees recorded from all adopters"],
                ["totalHydration", "number", "Total tokens sent to swarm operations"],
                ["minRunwayBalance", "number", "Minimum runway floor (90-day invariant)"],
              ].map(([field, type, desc]) => (
                <div className="docs-return-item" key={field}>
                  <code>{field}</code>
                  <span className="docs-return-type">{type}</span>
                  <span className="docs-return-desc">{desc}</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* withdrawAndRedistribute */}
        <div className="docs-section">
          <h2 className="section-title">withdrawAndRedistribute()</h2>
          <p className="docs-body">
            Permissionless crank. Anyone can call this — your launchpad, a keeper bot, any user.
            Sweeps accumulated fees from the mint into the treasury vault, then triggers the
            70/20/10 split if the vault balance exceeds the runway threshold.
          </p>
          <CodeBlock code={CRANK_CODE} />
        </div>

        {/* What your token gets */}
        <div className="docs-section">
          <h2 className="section-title">What Your Token Gets</h2>
          <ul className="docs-list">
            <li>
              <span className="invariant-check">&#10003;</span>
              <span>Transfer fees route to a <strong>program-owned vault</strong> — not a wallet anyone controls</span>
            </li>
            <li>
              <span className="invariant-check">&#10003;</span>
              <span>An agent swarm trades yield strategies on Hyperliquid nightly — validated by backtesting + walk-forward analysis</span>
            </li>
            <li>
              <span className="invariant-check">&#10003;</span>
              <span>Yield returns to the treasury &rarr; redistributed <strong>70/20/10 on-chain</strong> (holders / dev / ecosystem)</span>
            </li>
            <li>
              <span className="invariant-check">&#10003;</span>
              <span>Phase evolution: Sustenance &rarr; Ecosystem &rarr; Humanity — <strong>irreversible</strong>, threshold-gated</span>
            </li>
            <li>
              <span className="invariant-check">&#10003;</span>
              <span>The program enforces constraints — <strong>no rug is possible by design</strong></span>
            </li>
          </ul>
        </div>

        {/* Integration checklist */}
        <div className="docs-section">
          <h2 className="section-title">Integration Checklist</h2>
          <div className="docs-checklist">
            <div className="docs-check-step">
              <span className="docs-check-num">1</span>
              <div>
                <strong>Replace <code>createMint()</code> with <code>createRTPToken()</code></strong>
                <p className="docs-check-desc">Same inputs, but your token now has an autonomous treasury</p>
              </div>
            </div>
            <div className="docs-check-step">
              <span className="docs-check-num">2</span>
              <div>
                <strong>Store the returned <code>treasuryPDA</code></strong>
                <p className="docs-check-desc">Save it alongside your token record in your database</p>
              </div>
            </div>
            <div className="docs-check-step">
              <span className="docs-check-num">3</span>
              <div>
                <strong>Add <code>fetchTreasuryState()</code> to your token detail page</strong>
                <p className="docs-check-desc">Show treasury health, phase, and distribution history</p>
              </div>
            </div>
          </div>
        </div>

        {/* Constants */}
        <div className="docs-section">
          <h2 className="section-title">Constants</h2>
          <table className="research-table" style={{ maxWidth: 600 }}>
            <thead>
              <tr>
                <th>Export</th>
                <th>Value</th>
              </tr>
            </thead>
            <tbody>
              {CONSTANTS_TABLE.map(([name, value]) => (
                <tr key={name}>
                  <td className="sym"><code>{name}</code></td>
                  <td style={{ fontSize: "0.8125rem", wordBreak: "break-all" }}>{value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* No RTP token */}
        <div className="docs-section">
          <div className="docs-callout">
            <span className="docs-callout-label">No RTP Token</span>
            <p className="docs-callout-text">
              There is no RTP token. RTP is infrastructure. It serves the tokens that adopt it.
            </p>
          </div>
        </div>
      </section>

      {/* ── Footer ──────────────────────────────────────────── */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">MIT</span>
          <span className="vital-label">License</span>
        </div>
        <div className="vital">
          <span className="vital-value">3 functions</span>
          <span className="vital-label">SDK Surface</span>
        </div>
        <div className="vital">
          <span className="vital-value">Per-mint PDA</span>
          <span className="vital-label">Treasury Architecture</span>
        </div>
        <Link href="/" className="vital-link">
          Back to Dashboard
        </Link>
      </footer>
    </div>
  );
}
