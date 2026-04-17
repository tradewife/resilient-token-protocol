"use client";

import React, { useState, useCallback } from "react";
import Link from "next/link";
import { useWallet } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";

// Code snippets

const CODE_INSTALL = `npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor`;

const CODE_CREATE = `import { createRTPToken, RTP_PROGRAM_ID } from "@resilient-protocol/sdk";
import { Connection, Keypair } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // your launchpad's keypair

const result = await createRTPToken(connection, payer, {
  name: "Community Token",
  symbol: "CMTY",
  supply: 1_000_000_000,
  feeBps: 200,             // 2% transfer fee → treasury vault
  holdersWallet: payer.publicKey,    // optional, defaults to payer
  projectDevWallet: payer.publicKey, // optional, defaults to payer
  ecosystemWallet: payer.publicKey,  // optional, defaults to payer
});

console.log("Mint:", result.mint);
console.log("Treasury PDA:", result.treasuryPDA);
console.log("Vault PDA:", result.vaultPDA);
console.log("Explorer:", result.explorerUrl);`;

const CODE_FETCH = `import { fetchTreasuryState } from "@resilient-protocol/sdk";

const state = await fetchTreasuryState(connection, result.mint);

console.log("Phase:", state.phase);               // "Sustenance" | "Ecosystem" | "Humanity"
console.log("Vault balance:", state.vaultBalance);  // token units
console.log("Total distributed:", state.totalDistributedHolders);`;

const CODE_CRANK = `import { withdrawAndRedistribute } from "@resilient-protocol/sdk";

// Call this from a keeper bot or your platform's cron job.
// Permissionless — anyone can call it.
const { withdrawSig, redistributeSig } = await withdrawAndRedistribute(
  connection,
  payer,
  result.mint,
);

console.log("Fees withdrawn:", withdrawSig);
if (redistributeSig) console.log("Redistributed:", redistributeSig);`;

const CARDS = [
  {
    title: "Program-owned vault",
    desc: "No wallet controls the fees. The vault is a PDA derived from your mint — enforced by Solana code.",
  },
  {
    title: "Nightly yield strategies",
    desc: "30,000 parameter configs, 9-fold walk-forward validation, executed on Hyperliquid perps.",
  },
  {
    title: "Automatic redistribution",
    desc: "70% to holders, 20% to dev wallet, 10% to ecosystem fund. Split is enforced on-chain.",
  },
  {
    title: "Phase evolution",
    desc: "Sustenance → Ecosystem → Humanity. Threshold-gated, irreversible transitions enforced by the program.",
  },
  {
    title: "Constraint enforcement",
    desc: "Hard stops, drawdown limits, on-chain audit trail. No rug possible by design.",
  },
];

// Copy button

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [text]);

  return (
    <button
      onClick={handleCopy}
      style={{
        position: "absolute",
        top: 8,
        right: 8,
        background: copied ? "var(--emerald)" : "var(--surface-2)",
        color: copied ? "#fff" : "var(--text-tertiary)",
        border: "none",
        borderRadius: 4,
        padding: "4px 10px",
        fontSize: "0.6875rem",
        cursor: "pointer",
        fontFamily: "var(--font-body)",
        transition: "background 0.15s",
      }}
    >
      {copied ? "Copied" : "Copy"}
    </button>
  );
}

// Section heading

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h2 style={{
      fontFamily: "var(--font-display)",
      fontSize: "1.25rem",
      fontWeight: 400,
      color: "var(--text-primary)",
      marginBottom: "var(--space-lg)",
      letterSpacing: "-0.01em",
    }}>
      {children}
    </h2>
  );
}

// Page

export default function DocsPage() {
  const { publicKey, connected } = useWallet();
  const { setVisible } = useWalletModal();
  const addr = publicKey
    ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}`
    : null;

  return (
    <div className="page">
      {/* Top bar */}
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
          <Link href="/launch" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Platform Demo
          </Link>
          <Link href="/research" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Research
          </Link>
          <Link href="/" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Dashboard
          </Link>
          {connected && publicKey ? (
            <div className="wallet-pill">
              <span className="wallet-indicator" />
              <span className="wallet-addr">{addr}</span>
            </div>
          ) : (
            <button className="btn-connect" onClick={() => setVisible(true)}>
              Connect Wallet
            </button>
          )}
        </div>
      </header>

      <div className="docs-content">

        {/* Hero */}
        <section className="docs-section" style={{ marginTop: "var(--space-xl)", marginBottom: "var(--space-4xl)" }}>
          <h1 style={{
            fontFamily: "var(--font-display)",
            fontSize: "clamp(1.75rem, 4vw, 3rem)",
            fontWeight: 400,
            lineHeight: 1.1,
            letterSpacing: "-0.02em",
            color: "var(--text-primary)",
            marginBottom: "var(--space-md)",
            maxWidth: "28ch",
          }}>
            The yield treasury layer for Solana token launches
          </h1>
          <p style={{
            fontSize: "1.0625rem",
            color: "var(--text-secondary)",
            lineHeight: 1.65,
            maxWidth: "52ch",
            marginBottom: "var(--space-xl)",
          }}>
            Add one function call to your launch flow. Every token
            you launch gets an autonomous treasury — enforced by Solana code,
            not promises.
          </p>
          <div style={{ display: "flex", gap: "var(--space-md)", flexWrap: "wrap" }}>
            <a
              href="https://github.com/tradewife/resilient-token-protocol/blob/main/sdk/index.ts"
              target="_blank"
              rel="noopener noreferrer"
              className="btn-launch"
              style={{ textDecoration: "none" }}
            >
              View on GitHub &rarr;
            </a>
            <a href="#quickstart" className="btn-secondary" style={{ textDecoration: "none" }}>
              Read the SDK &rarr;
            </a>
          </div>
        </section>

        {/* How It Works */}
        <section className="docs-section">
          <SectionHeading>How It Works</SectionHeading>
          <div className="hiw-steps" style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "var(--space-xl)", paddingBottom: 0 }}>
            <div className="hiw-step">
              <span className="hiw-num">1</span>
              <div className="hiw-content">
                <p className="hiw-text">
                  Your launchpad calls <code style={{ color: "var(--coral)", fontSize: "0.8125rem" }}>createRTPToken()</code> at mint creation.
                  One function replaces your existing <code style={{ color: "var(--coral)", fontSize: "0.8125rem" }}>createMint()</code>.
                </p>
              </div>
            </div>
            <div className="hiw-step">
              <span className="hiw-num">2</span>
              <div className="hiw-content">
                <p className="hiw-text">
                  Transfer fees route to a per-mint vault PDA. The program owns the vault — no wallet, no multisig, no key risk.
                </p>
              </div>
            </div>
            <div className="hiw-step">
              <span className="hiw-num">3</span>
              <div className="hiw-content">
                <p className="hiw-text">
                  An agent swarm trades yield on Hyperliquid nightly. Yield returns to the treasury and is distributed 70/20/10 on-chain.
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* Install */}
        <section className="docs-section">
          <SectionHeading>Install</SectionHeading>
          <div className="code-block" style={{ position: "relative" }}>
            <CopyButton text={CODE_INSTALL} />
            <pre><code>{CODE_INSTALL}</code></pre>
          </div>
        </section>

        {/* Quick Start */}
        <section className="docs-section" id="quickstart">
          <SectionHeading>Quick Start</SectionHeading>

          {/* 1. createRTPToken */}
          <div style={{ marginBottom: "var(--space-2xl)" }}>
            <h3 style={{
              fontSize: "0.9375rem",
              color: "var(--text-primary)",
              marginBottom: "var(--space-sm)",
              fontWeight: 500,
            }}>
              1. Create a token with an autonomous treasury
            </h3>
            <div className="code-block" style={{ position: "relative" }}>
              <CopyButton text={CODE_CREATE} />
              <pre><code>{CODE_CREATE}</code></pre>
            </div>
          </div>

          {/* 2. fetchTreasuryState */}
          <div style={{ marginBottom: "var(--space-2xl)" }}>
            <h3 style={{
              fontSize: "0.9375rem",
              color: "var(--text-primary)",
              marginBottom: "var(--space-sm)",
              fontWeight: 500,
            }}>
              2. Read treasury state (for your token dashboard)
            </h3>
            <div className="code-block" style={{ position: "relative" }}>
              <CopyButton text={CODE_FETCH} />
              <pre><code>{CODE_FETCH}</code></pre>
            </div>
          </div>

          {/* 3. withdrawAndRedistribute */}
          <div style={{ marginBottom: "var(--space-2xl)" }}>
            <h3 style={{
              fontSize: "0.9375rem",
              color: "var(--text-primary)",
              marginBottom: "var(--space-sm)",
              fontWeight: 500,
            }}>
              3. Crank fee distribution (permissionless)
            </h3>
            <div className="code-block" style={{ position: "relative" }}>
              <CopyButton text={CODE_CRANK} />
              <pre><code>{CODE_CRANK}</code></pre>
            </div>
          </div>
        </section>

        {/* Integration Checklist */}
        <section className="docs-section">
          <SectionHeading>Integration Checklist</SectionHeading>
          <div className="docs-checklist">
            <div className="docs-check-step">
              <span className="docs-check-num">1</span>
              <div>
                <strong>
                  Replace <code>createMint()</code> with <code>createRTPToken()</code> in your launch flow
                </strong>
                <p className="docs-check-desc">
                  Same inputs you already collect — name, symbol, supply, fee — but your token now has an autonomous treasury.
                </p>
              </div>
            </div>
            <div className="docs-check-step">
              <span className="docs-check-num">2</span>
              <div>
                <strong>
                  Store the returned <code>treasuryPDA</code> alongside your token record
                </strong>
                <p className="docs-check-desc">
                  You&apos;ll need this to query treasury state and trigger redistribution.
                </p>
              </div>
            </div>
            <div className="docs-check-step">
              <span className="docs-check-num">3</span>
              <div>
                <strong>
                  (Optional) Add <code>fetchTreasuryState()</code> to your token detail page
                </strong>
                <p className="docs-check-desc">
                  Show your users the treasury health — phase, vault balance, total distributed.
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* What Your Tokens Get */}
        <section className="docs-section">
          <SectionHeading>What Your Tokens Get</SectionHeading>
          <div style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
            gap: "var(--space-md)",
          }}>
            {CARDS.map((card) => (
              <div
                key={card.title}
                style={{
                  padding: "var(--space-lg)",
                  background: "rgba(255, 255, 255, 0.03)",
                  border: "1px solid var(--border)",
                  borderRadius: 8,
                }}
              >
                <h3 style={{
                  fontSize: "0.9375rem",
                  fontWeight: 500,
                  color: "var(--text-primary)",
                  marginBottom: "var(--space-sm)",
                }}>
                  {card.title}
                </h3>
                <p style={{
                  fontSize: "0.8125rem",
                  color: "var(--text-tertiary)",
                  lineHeight: 1.6,
                }}>
                  {card.desc}
                </p>
              </div>
            ))}
          </div>
        </section>

        {/* API Reference */}
        <section className="docs-section">
          <SectionHeading>API Reference</SectionHeading>

          {/* createRTPToken */}
          <div style={{ marginBottom: "var(--space-2xl)" }}>
            <h3 style={{
              fontFamily: "var(--font-mono)",
              fontSize: "0.9375rem",
              color: "var(--coral)",
              marginBottom: "var(--space-sm)",
              fontWeight: 400,
            }}>
              createRTPToken(connection, payer, config) &rarr; RTPTokenResult
            </h3>
            <div className="docs-returns">
              <span className="docs-returns-label">Config fields</span>
              <div className="docs-returns-grid">
                {[
                  ["name", "string", "Token display name"],
                  ["symbol", "string", "Ticker symbol (max 8 chars)"],
                  ["supply", "number", "Total supply in tokens"],
                  ["feeBps", "number", "Transfer fee in basis points (e.g. 200 = 2%)"],
                  ["holdersWallet?", "PublicKey", "Override 70% recipient (default: payer)"],
                  ["projectDevWallet?", "PublicKey", "Override 20% recipient (default: payer)"],
                  ["ecosystemWallet?", "PublicKey", "Override 10% recipient (default: payer)"],
                  ["minRunwayBalance?", "number", "Sustenance floor (default: 10 tokens)"],
                ].map(([field, type, desc]) => (
                  <div className="docs-return-item" key={field}>
                    <code>{field}</code>
                    <span className="docs-return-type">{type}</span>
                    <span className="docs-return-desc">{desc}</span>
                  </div>
                ))}
              </div>
              <span className="docs-returns-label" style={{ marginTop: "var(--space-md)" }}>Returns: RTPTokenResult</span>
              <div className="docs-returns-grid">
                {[
                  ["mint", "string", "Base58 mint address"],
                  ["treasuryPDA", "string", "Per-mint treasury account"],
                  ["vaultPDA", "string", "Per-mint vault token account"],
                  ["signature", "string", "Transaction signature"],
                  ["explorerUrl", "string", "Solana Explorer link"],
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

          {/* fetchTreasuryState */}
          <div style={{ marginBottom: "var(--space-2xl)" }}>
            <h3 style={{
              fontFamily: "var(--font-mono)",
              fontSize: "0.9375rem",
              color: "var(--coral)",
              marginBottom: "var(--space-sm)",
              fontWeight: 400,
            }}>
              fetchTreasuryState(connection, mintAddress) &rarr; TreasuryState
            </h3>
            <p style={{ fontSize: "0.8125rem", color: "var(--text-tertiary)", marginBottom: "var(--space-md)", lineHeight: 1.6 }}>
              Read-only. No transactions, no signing required. Returns zeros if the treasury doesn&apos;t exist yet.
            </p>
            <span className="docs-returns-label">Returns: TreasuryState</span>
            <div className="docs-returns-grid">
              {[
                ["phase", "string", '"Sustenance" | "Ecosystem" | "Humanity"'],
                ["vaultBalance", "number", "Current vault balance (tokens)"],
                ["totalFeesWithdrawn", "number", "Cumulative fees pulled from mint"],
                ["totalDistributedHolders", "number", "Cumulative 70% distributions"],
                ["totalDistributedDev", "number", "Cumulative 20% distributions"],
                ["totalDistributedEcosystem", "number", "Cumulative 10% distributions"],
                ["totalHydration", "number", "Cumulative self-hydration amount"],
                ["minRunwayBalance", "number", "Sustenance runway floor"],
              ].map(([field, type, desc]) => (
                <div className="docs-return-item" key={field}>
                  <code>{field}</code>
                  <span className="docs-return-type">{type}</span>
                  <span className="docs-return-desc">{desc}</span>
                </div>
              ))}
            </div>
          </div>

          {/* withdrawAndRedistribute */}
          <div style={{ marginBottom: "var(--space-2xl)" }}>
            <h3 style={{
              fontFamily: "var(--font-mono)",
              fontSize: "0.9375rem",
              color: "var(--coral)",
              marginBottom: "var(--space-sm)",
              fontWeight: 400,
            }}>
              withdrawAndRedistribute(connection, payer, mintAddress)
            </h3>
            <div style={{ marginBottom: "var(--space-sm)" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "0.875rem", color: "var(--text-muted)" }}>
                &rarr; &#123; withdrawSig, redistributeSig? &#125;
              </span>
            </div>
            <p style={{ fontSize: "0.8125rem", color: "var(--text-tertiary)", lineHeight: 1.6 }}>
              Permissionless — anyone can call. Withdraws accrued fees from the mint into the treasury vault,
              then attempts redistribution (70/20/10 split) if the vault balance exceeds the runway threshold.
              If below threshold, only the withdraw succeeds and <code style={{ color: "var(--coral)" }}>redistributeSig</code> is undefined.
            </p>
          </div>
        </section>

        {/* Constants */}
        <section className="docs-section">
          <SectionHeading>Constants</SectionHeading>
          <div style={{ border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.875rem" }}>
              <thead>
                <tr>
                  <th style={{
                    textAlign: "left",
                    fontSize: "0.6875rem",
                    letterSpacing: "0.06em",
                    textTransform: "uppercase",
                    color: "var(--text-muted)",
                    padding: "var(--space-sm) var(--space-md)",
                    borderBottom: "1px solid var(--border)",
                  }}>Export</th>
                  <th style={{
                    textAlign: "left",
                    fontSize: "0.6875rem",
                    letterSpacing: "0.06em",
                    textTransform: "uppercase",
                    color: "var(--text-muted)",
                    padding: "var(--space-sm) var(--space-md)",
                    borderBottom: "1px solid var(--border)",
                  }}>Value</th>
                </tr>
              </thead>
              <tbody>
                {[
                  ["RTP_PROGRAM_ID", "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"],
                  ["RTP_DEVNET_RPC", "https://api.devnet.solana.com"],
                  ["RTP_MAINNET_RPC", "https://api.mainnet-beta.solana.com"],
                ].map(([exp, val]) => (
                  <tr key={exp}>
                    <td style={{
                      padding: "var(--space-sm) var(--space-md)",
                      borderBottom: "1px solid rgba(255,255,255,0.04)",
                    }}>
                      <code style={{ color: "var(--coral)", fontSize: "0.8125rem" }}>{exp}</code>
                    </td>
                    <td style={{
                      padding: "var(--space-sm) var(--space-md)",
                      color: "var(--text-secondary)",
                      fontFamily: "var(--font-mono)",
                      fontSize: "0.8125rem",
                      borderBottom: "1px solid rgba(255,255,255,0.04)",
                      wordBreak: "break-all",
                    }}>
                      {val}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        {/* No RTP Token */}
        <section className="docs-section">
          <div className="docs-callout">
            <span className="docs-callout-label">No RTP Token</span>
            <p className="docs-callout-text">
              There is no RTP token. RTP is pure infrastructure. It serves the tokens that adopt it.
            </p>
          </div>
        </section>

        {/* Footer */}
        <footer className="vitals" style={{ marginTop: "var(--space-4xl)" }}>
          <div className="vital">
            <Link href="/" className="vital-link">Dashboard &rarr;</Link>
            <span className="vital-label">Home</span>
          </div>
          <div className="vital">
            <a
              className="vital-link"
              href="https://github.com/tradewife/resilient-token-protocol"
              target="_blank"
              rel="noopener noreferrer"
            >
              GitHub &rarr;
            </a>
            <span className="vital-label">Source</span>
          </div>
          <div className="vital">
            <span className="vital-value" style={{ fontSize: "0.8125rem" }}>
              Built at SWARMs/Canteen &times; Colosseum 2026
            </span>
            <span className="vital-label">Hackathon</span>
          </div>
        </footer>
      </div>
    </div>
  );
}
