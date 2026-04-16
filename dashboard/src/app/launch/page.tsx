"use client";

import React, { useState } from "react";
import Link from "next/link";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { Connection, PublicKey } from "@solana/web3.js";
import {
  createRTPToken,
  fetchTreasuryState,
  RTP_PROGRAM_ID,
  type RTPTokenResult,
  type TreasuryState,
} from "../../lib/sdk";

const PROGRAM_ID_SHORT = RTP_PROGRAM_ID.toBase58();
const CLUSTER = "devnet";

interface FormData {
  projectName: string;
  tokenSymbol: string;
  totalSupply: string;
  feeBps: string;
}

type LaunchPhase = "form" | "confirming" | "launching" | "success" | "error";

export default function LaunchPage() {
  const { publicKey, connected, signTransaction } = useWallet();
  const { connection: rpcConnection } = useConnection();
  const { setVisible } = useWalletModal();

  const [form, setForm] = useState<FormData>({
    projectName: "",
    tokenSymbol: "",
    totalSupply: "1000000000",
    feeBps: "200",
  });
  const [phase, setPhase] = useState<LaunchPhase>("form");
  const [result, setResult] = useState<RTPTokenResult | null>(null);
  const [treasuryState, setTreasuryState] = useState<TreasuryState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [codeSnippet, setCodeSnippet] = useState(false);

  const addr = publicKey
    ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}`
    : null;

  const update = (field: keyof FormData, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const handleLaunch = async () => {
    if (!publicKey || !signTransaction) return;
    setPhase("launching");
    setError(null);

    try {
      const walletAdapter = {
        publicKey,
        signTransaction,
      };

      const launchResult = await createRTPToken(rpcConnection, walletAdapter, {
        name: form.projectName || "My Token",
        symbol: form.tokenSymbol || "TKN",
        supply: parseInt(form.totalSupply) || 1_000_000_000,
        feeBps: parseInt(form.feeBps) || 200,
      });

      setResult(launchResult);
      setPhase("success");

      // Fetch treasury state after a brief delay to let it propagate
      setTimeout(async () => {
        try {
          const state = await fetchTreasuryState(rpcConnection, launchResult.mint);
          setTreasuryState(state);
        } catch {
          // Treasury state fetch is best-effort
        }
      }, 3000);
    } catch (err: any) {
      setError(err?.message || String(err));
      setPhase("error");
    }
  };

  const snippet = `import { createRTPToken } from "@resilient-protocol/sdk";
import { Connection, Keypair } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // your launchpad keypair

const result = await createRTPToken(connection, payer, {
  name: "${form.projectName || "My Token"}",
  symbol: "${form.tokenSymbol || "TKN"}",
  supply: ${form.totalSupply || "1_000_000_000"},
  feeBps: ${form.feeBps || "200"},  // ${(parseInt(form.feeBps) / 100).toFixed(0)}% transfer fee
});

console.log("Mint:", result.mint);
console.log("Treasury PDA:", result.treasuryPDA);
console.log("Vault PDA:", result.vaultPDA);
// Fee destination: per-mint vault PDA (program-owned, immutable)`;

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
          <Link href="/docs" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
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

      <section className="launch-hero">
        <h1 className="launch-title">Platform Integration Preview</h1>
        <p className="launch-subtitle">
          This is what your token launch form looks like after a one-day RTP integration.
          The form below generates a working SDK call for your backend.
        </p>
      </section>

      {/* ── Wallet connect prompt ── */}
      {!connected && (
        <section className="launch-form-section" style={{ textAlign: "center", padding: "48px 24px" }}>
          <p style={{ color: "var(--text-secondary)", marginBottom: "24px", fontSize: "1.1rem" }}>
            Connect your Phantom wallet to launch a token on devnet.
          </p>
          <button
            className="btn-launch"
            onClick={() => setVisible(true)}
          >
            Connect Phantom Wallet
          </button>
        </section>
      )}

      {/* ── Launch form (visible when connected, before launch) ── */}
      {connected && (phase === "form" || phase === "error") && (
        <section className="launch-form-section">
          <h3 style={{
            fontSize: "0.8125rem",
            fontWeight: 500,
            letterSpacing: "0.08em",
            textTransform: "uppercase",
            color: "var(--text-tertiary)",
            marginBottom: "var(--space-lg)",
            textAlign: "center",
          }}>
            Configure your token parameters &rarr;
          </h3>
          <form
            className="launch-form"
            onSubmit={(e) => {
              e.preventDefault();
              setPhase("confirming");
            }}
          >
            <div className="form-group">
              <label className="form-label" htmlFor="projectName">
                Project Name
              </label>
              <input
                id="projectName"
                className="form-input"
                type="text"
                placeholder="e.g. My Launchpad Token"
                value={form.projectName}
                onChange={(e) => update("projectName", e.target.value)}
                required
              />
            </div>

            <div className="form-group">
              <label className="form-label" htmlFor="tokenSymbol">
                Token Symbol
              </label>
              <input
                id="tokenSymbol"
                className="form-input"
                type="text"
                placeholder="e.g. MLT"
                maxLength={8}
                value={form.tokenSymbol}
                onChange={(e) => update("tokenSymbol", e.target.value.toUpperCase())}
                required
              />
            </div>

            <div className="form-row">
              <div className="form-group">
                <label className="form-label" htmlFor="totalSupply">
                  Total Supply (tokens)
                </label>
                <input
                  id="totalSupply"
                  className="form-input"
                  type="number"
                  min="1"
                  value={form.totalSupply}
                  onChange={(e) => update("totalSupply", e.target.value)}
                  required
                />
              </div>

              <div className="form-group">
                <label className="form-label" htmlFor="feeBps">
                  Transfer Fee (bps)
                  <span className="form-hint">{(parseInt(form.feeBps || "0") / 100).toFixed(1)}%</span>
                </label>
                <input
                  id="feeBps"
                  className="form-input"
                  type="number"
                  min="0"
                  max="500"
                  step="10"
                  value={form.feeBps}
                  onChange={(e) => update("feeBps", e.target.value)}
                  required
                />
              </div>
            </div>

            <div className="form-note">
              The transfer fee destination is a per-mint vault PDA derived from the program ID
              (<code>{PROGRAM_ID_SHORT.slice(0, 8)}...{PROGRAM_ID_SHORT.slice(-4)}</code>).
              Each token gets its own treasury — no shared vault, no single point of failure.
            </div>

            <button type="submit" className="btn-launch">
              Launch Token on Devnet
            </button>
          </form>

          {error && (
            <div style={{
              marginTop: "16px",
              padding: "12px 16px",
              background: "rgba(220, 38, 38, 0.1)",
              border: "1px solid rgba(220, 38, 38, 0.3)",
              borderRadius: "8px",
              color: "#f87171",
              fontSize: "0.875rem",
            }}>
              <strong>Error:</strong> {error}
            </div>
          )}

          {/* Code snippet section (always visible when connected) */}
          <div style={{ marginTop: "32px", paddingTop: "24px", borderTop: "1px solid var(--border)" }}>
            <h3 style={{ fontSize: "0.9375rem", color: "var(--text-secondary)", marginBottom: "12px" }}>
              Generated SDK call:
            </h3>
            <div className="code-block">
              <pre><code>{snippet}</code></pre>
            </div>
            <button
              className="btn-secondary"
              onClick={() => {
                navigator.clipboard.writeText(snippet);
                setCodeSnippet(true);
                setTimeout(() => setCodeSnippet(false), 2000);
              }}
            >
              {codeSnippet ? "Copied!" : "Copy to Clipboard"}
            </button>
            <p style={{
              marginTop: "16px",
              fontSize: "0.8125rem",
              color: "var(--text-tertiary)",
              lineHeight: 1.6,
              padding: "var(--space-sm) var(--space-md)",
              background: "rgba(255, 255, 255, 0.02)",
              border: "1px solid var(--border)",
              borderRadius: 6,
            }}>
              This call creates the mint AND initializes the treasury program in one transaction.
              Your token is RTP-enabled from block one.
            </p>
            <Link
              href="/docs"
              style={{
                display: "inline-block",
                marginTop: "var(--space-md)",
                fontSize: "0.8125rem",
                color: "var(--coral)",
                textDecoration: "underline",
                textDecorationColor: "var(--border)",
                textUnderlineOffset: 3,
              }}
            >
              Read the full integration guide &rarr;
            </Link>
          </div>
        </section>
      )}

      {/* ── Confirmation dialog ── */}
      {phase === "confirming" && (
        <section className="launch-form-section" style={{ textAlign: "center" }}>
          <h2 style={{ fontSize: "1.25rem", marginBottom: "16px" }}>Confirm Token Launch</h2>
          <div style={{
            background: "rgba(0,0,0,0.3)",
            border: "1px solid var(--border)",
            borderRadius: "8px",
            padding: "20px",
            marginBottom: "24px",
            textAlign: "left",
            maxWidth: 480,
            margin: "0 auto 24px",
          }}>
            <div style={{ marginBottom: "8px" }}><strong>Name:</strong> {form.projectName}</div>
            <div style={{ marginBottom: "8px" }}><strong>Symbol:</strong> {form.tokenSymbol}</div>
            <div style={{ marginBottom: "8px" }}><strong>Supply:</strong> {parseInt(form.totalSupply).toLocaleString()}</div>
            <div style={{ marginBottom: "8px" }}><strong>Fee:</strong> {(parseInt(form.feeBps) / 100).toFixed(1)}%</div>
            <div style={{ marginBottom: "8px" }}><strong>Payer:</strong> {addr}</div>
            <div><strong>Network:</strong> Solana Devnet</div>
          </div>
          <p style={{ color: "var(--text-secondary)", fontSize: "0.875rem", marginBottom: "24px" }}>
            Phantom will prompt you to sign 4 transactions: mint creation, treasury init, ATA creation, supply mint.
          </p>
          <div style={{ display: "flex", gap: "12px", justifyContent: "center" }}>
            <button className="btn-launch" onClick={handleLaunch}>
              Sign & Launch
            </button>
            <button className="btn-secondary" onClick={() => setPhase("form")}>
              Cancel
            </button>
          </div>
        </section>
      )}

      {/* ── Launching spinner ── */}
      {phase === "launching" && (
        <section className="launch-form-section" style={{ textAlign: "center", padding: "64px 24px" }}>
          <div style={{ fontSize: "2rem", marginBottom: "16px" }}>⏳</div>
          <h2 style={{ fontSize: "1.25rem", marginBottom: "8px" }}>Launching your token...</h2>
          <p style={{ color: "var(--text-secondary)", fontSize: "0.875rem" }}>
            Signing transactions via Phantom. Check your wallet for approval prompts.
          </p>
        </section>
      )}

      {/* ── Success result ── */}
      {phase === "success" && result && (
        <section className="launch-result-section">
          <div className="launch-success">
            <span className="success-check">✓</span>
            <h2>Token Launched!</h2>
            <p className="success-subtitle">
              Your Token-2022 mint is live on devnet with fees routing to a per-mint treasury vault PDA.
            </p>
          </div>

          <div className="result-info" style={{ marginBottom: "24px" }}>
            <div className="info-card">
              <span className="info-label">Mint Address</span>
              <span className="info-value" style={{ fontSize: "0.75rem", wordBreak: "break-all" }}>
                {result.mint}
              </span>
              <a
                href={`https://explorer.solana.com/address/${result.mint}?cluster=${CLUSTER}`}
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: "var(--coral)", fontSize: "0.75rem" }}
              >
                View on Explorer ↗
              </a>
            </div>
            <div className="info-card">
              <span className="info-label">Treasury PDA</span>
              <span className="info-value" style={{ fontSize: "0.75rem", wordBreak: "break-all" }}>
                {result.treasuryPDA}
              </span>
              <a
                href={`https://explorer.solana.com/address/${result.treasuryPDA}?cluster=${CLUSTER}`}
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: "var(--coral)", fontSize: "0.75rem" }}
              >
                View on Explorer ↗
              </a>
            </div>
            <div className="info-card">
              <span className="info-label">Vault PDA</span>
              <span className="info-value" style={{ fontSize: "0.75rem", wordBreak: "break-all" }}>
                {result.vaultPDA}
              </span>
            </div>
            <div className="info-card">
              <span className="info-label">Transaction</span>
              <a
                href={result.explorerUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: "var(--coral)", fontSize: "0.75rem", wordBreak: "break-all" }}
              >
                {result.signature.slice(0, 20)}...↗
              </a>
            </div>
          </div>

          {/* Treasury state (if loaded) */}
          {treasuryState && (
            <div style={{
              background: "rgba(0,0,0,0.3)",
              border: "1px solid var(--border)",
              borderRadius: "8px",
              padding: "16px",
              marginBottom: "24px",
            }}>
              <h3 style={{ fontSize: "0.875rem", color: "var(--coral)", marginBottom: "12px" }}>
                Treasury State (on-chain)
              </h3>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px", fontSize: "0.8125rem" }}>
                <div><span style={{ color: "var(--text-secondary)" }}>Phase:</span> {treasuryState.phase}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Vault Balance:</span> {treasuryState.vaultBalance}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Fees Withdrawn:</span> {treasuryState.totalFeesWithdrawn}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Runway Floor:</span> {treasuryState.minRunwayBalance}</div>
              </div>
            </div>
          )}

          <div className="result-actions">
            <button className="btn-secondary" onClick={() => {
              setPhase("form");
              setResult(null);
              setTreasuryState(null);
              setError(null);
            }}>
              Launch Another Token
            </button>
          </div>

          <div className="result-info">
            <div className="info-card">
              <span className="info-label">Fee Destination</span>
              <span className="info-value">Per-mint vault PDA</span>
              <span className="info-note">Program-owned — derived from your mint address</span>
            </div>
            <div className="info-card">
              <span className="info-label">Token Standard</span>
              <span className="info-value">Token-2022 (TransferFeeConfig)</span>
              <span className="info-note">Transfer fees enforced at mint level</span>
            </div>
            <div className="info-card">
              <span className="info-label">Redistribution</span>
              <span className="info-value">70% holders / 20% dev / 10% ecosystem</span>
              <span className="info-note">Enforced on-chain by Anchor program</span>
            </div>
          </div>
        </section>
      )}

      {/* ── Footer ──────────────────────────────────────────── */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">No RTP Token</span>
          <span className="vital-label">Protocol</span>
        </div>
        <div className="vital">
          <span className="vital-value">{PROGRAM_ID_SHORT.slice(0, 8)}...{PROGRAM_ID_SHORT.slice(-4)}</span>
          <span className="vital-label">Program ID</span>
        </div>
        <div className="vital">
          <span className="vital-value">Per-mint PDA</span>
          <span className="vital-label">Treasury Vault</span>
        </div>
        <div className="vital">
          <span className="vital-value">MIT</span>
          <span className="vital-label">License</span>
        </div>
        <Link href="/" className="vital-link">
          Back to Dashboard ↗
        </Link>
      </footer>
    </div>
  );
}
