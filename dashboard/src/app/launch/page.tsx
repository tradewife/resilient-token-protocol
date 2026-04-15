"use client";

import React, { useState } from "react";
import Link from "next/link";

const TREASURY_VAULT = "FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF";

interface FormData {
  projectName: string;
  tokenSymbol: string;
  totalSupply: string;
  feeBps: string;
  contactEmail: string;
}

export default function LaunchPage() {
  const [form, setForm] = useState<FormData>({
    projectName: "",
    tokenSymbol: "",
    totalSupply: "1000000000",
    feeBps: "200",
    contactEmail: "",
  });
  const [submitted, setSubmitted] = useState(false);
  const [codeSnippet, setCodeSnippet] = useState(false);

  const update = (field: keyof FormData, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitted(true);
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
// Fee destination: ${TREASURY_VAULT} (hardcoded, immutable)`;

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
          <Link href="/research" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>
            Research
          </Link>
          <Link href="/" className="btn-connect" style={{ textDecoration: "none" }}>
            Dashboard
          </Link>
        </div>
      </header>

      <section className="launch-hero">
        <h1 className="launch-title">Launch Your Token with RTP</h1>
        <p className="launch-subtitle">
          Create a Token-2022 mint whose transfer fees permanently route to the RTP treasury vault.
          No RTP token. No middleman. Just enforced economics.
        </p>
      </section>

      {!submitted ? (
        <section className="launch-form-section">
          <form className="launch-form" onSubmit={handleSubmit}>
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

            <div className="form-group">
              <label className="form-label" htmlFor="contactEmail">
                Contact Email
              </label>
              <input
                id="contactEmail"
                className="form-input"
                type="email"
                placeholder="team@example.com"
                value={form.contactEmail}
                onChange={(e) => update("contactEmail", e.target.value)}
                required
              />
            </div>

            <div className="form-note">
              The transfer fee destination is hardcoded to the RTP treasury vault
              (<code>{TREASURY_VAULT.slice(0, 8)}...{TREASURY_VAULT.slice(-4)}</code>).
              This cannot be changed — it is a constitutional invariant of the protocol.
            </div>

            <button type="submit" className="btn-launch">
              Generate Integration Code
            </button>
          </form>
        </section>
      ) : (
        <section className="launch-result-section">
          <div className="launch-success">
            <span className="success-check">✓</span>
            <h2>Your Integration Code</h2>
            <p className="success-subtitle">
              Copy this into your launchpad backend. The SDK handles mint creation,
              fee configuration, and initial supply distribution.
            </p>
          </div>

          <div className="code-block">
            <pre><code>{snippet}</code></pre>
          </div>

          <div className="result-actions">
            <button
              className="btn-launch"
              onClick={() => {
                navigator.clipboard.writeText(snippet);
                setCodeSnippet(true);
                setTimeout(() => setCodeSnippet(false), 2000);
              }}
            >
              {codeSnippet ? "Copied!" : "Copy to Clipboard"}
            </button>
            <button className="btn-secondary" onClick={() => setSubmitted(false)}>
              Edit Configuration
            </button>
          </div>

          <div className="result-info">
            <div className="info-card">
              <span className="info-label">Fee Destination</span>
              <span className="info-value">{TREASURY_VAULT}</span>
              <span className="info-note">Hardcoded — cannot be redirected</span>
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
          <span className="vital-value">{TREASURY_VAULT.slice(0, 8)}...{TREASURY_VAULT.slice(-4)}</span>
          <span className="vital-label">Treasury Vault (immutable)</span>
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
