"use client";

import React, { useEffect, useState } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";

const TREASURY_PDA = "FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF";
const PROGRAM_ID = "4LvsHbe9LLwgogcDbH7ieTsGcWZctjYFZkzZwaHDM8Ad";
const MAINNET_RPC = "https://api.mainnet-beta.solana.com";

const FEED_LINES = [
  { ts: "10:04:12", tag: "night shift", msg: "Evaluating 30,000 parameter configs across SOL/USDT" },
  { ts: "10:18:45", tag: "night shift", msg: "9-fold walk-forward analysis complete. Darwinian mutations generated." },
  { ts: "10:19:02", tag: "validated", msg: "SOL/USDT Survivor 2.69 — +118.3% PnL, 78% consistency, 429 trades" },
  { ts: "10:20:15", tag: "trading wing", msg: "Requesting ExecutePermit via soulguard..." },
  { ts: "10:20:18", tag: "audit wing", msg: "3-agent tribunal verifying compliance against soulcontract" },
  { ts: "10:20:45", tag: "approved", msg: "Constraints satisfied. ExecutePermit granted." },
  { ts: "10:21:05", tag: "trading wing", msg: "Hyperliquid POST /exchange — BUY 0.12 SOL @ $142.50" },
  { ts: "10:21:08", tag: "trading wing", msg: "Fill confirmed. Position opened." },
  { ts: "10:35:22", tag: "trading wing", msg: "Hyperliquid POST /exchange — SELL 0.12 SOL @ $160.00" },
  { ts: "10:35:25", tag: "validated", msg: "Realized PnL: +$0.175 USDC. Depositing yield to treasury PDA." },
  { ts: "10:36:01", tag: "treasury", msg: "SPL transfer_checked → FNQbK1...otF. Signature: 45DrjL8q..." },
  { ts: "10:40:00", tag: "memory", msg: "Project memory promoted: cycle_2 → overview tier. 3 files persisted." },
];

const WINGS = [
  { name: "Trading", status: "Executing SOL/USDT", active: true },
  { name: "Security", status: "Monitoring", active: true },
  { name: "Evolve", status: "Idle", active: false },
  { name: "Knowledge", status: "Idle", active: false },
  { name: "Audit", status: "3/3 approved", active: true },
  { name: "Futureproof", status: "Idle", active: false },
];

const INVARIANTS = [
  "PDA owns treasury — no private key risk",
  "TransferFeeConfig immutable after mint",
  "CPI-only transfers — atomic & verifiable",
  "Agent proposes, human approves irreversible actions",
  "Phase transitions irreversible: Sustenance → Ecosystem → Humanity",
];

export default function Home() {
  const { publicKey, connected, disconnect } = useWallet();
  const { connection } = useConnection(); // devnet via ConnectionProvider
  const { setVisible } = useWalletModal();

  const [treasurySol, setTreasurySol] = useState<number | null>(null);
  const [walletSol, setWalletSol] = useState<number | null>(null);

  // ── Treasury balance (devnet) ──────────────────────────────
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const lamports = await connection.getBalance(new PublicKey(TREASURY_PDA));
        if (alive) setTreasurySol(lamports / LAMPORTS_PER_SOL);
      } catch { /* retry next tick */ }
    };
    poll();
    const id = setInterval(poll, 10_000);
    return () => { alive = false; clearInterval(id); };
  }, [connection]);

  // ── Connected wallet balance (mainnet) ─────────────────────
  useEffect(() => {
    if (!publicKey) { setWalletSol(null); return; }
    let alive = true;
    const poll = async () => {
      try {
        const res = await fetch(MAINNET_RPC, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            method: "getBalance",
            params: [publicKey.toBase58()],
          }),
        });
        const json = await res.json();
        const lamports: number = json?.result?.value ?? 0;
        if (alive) setWalletSol(lamports / LAMPORTS_PER_SOL);
      } catch { /* retry */ }
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, [publicKey]);

  const tBal = treasurySol !== null ? treasurySol.toFixed(4) : "—";
  const uBal = walletSol !== null ? walletSol.toFixed(4) : null;
  const addr = publicKey ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}` : null;

  return (
    <div className="page">
      {/* ── Top bar ────────────────────────────────────────── */}
      <header className="topbar">
        <div className="brand">
          <span className="brand-dot" />
          <span className="brand-name">RESILIENT TOKEN PROTOCOL</span>
        </div>
        <div className="topbar-actions">
          <span className="network-badge">Devnet</span>
          {connected && publicKey ? (
            <div className="wallet-pill">
              <span className="wallet-indicator" />
              <span className="wallet-addr">{addr}</span>
              <button className="btn-disconnect" onClick={disconnect} title="Disconnect">&times;</button>
            </div>
          ) : (
            <button className="btn-connect" onClick={() => setVisible(true)}>
              Connect Wallet
            </button>
          )}
        </div>
      </header>

      {/* ── Hero: image + treasury overview ─────────────────── */}
      <section className="hero">
        <div className="hero-image-wrap">
          <img
            src="/bg-flower.jpg"
            alt="Ethereal flower in emerald and coral — the organic intelligence that drives RTP"
          />
        </div>

        <div className="hero-content">
          <span className="hero-label">EVOLVING ALGORITHMS · COMPOUND CREATOR FEES</span>
          <h1 className="hero-title">
            From the Trenches
            <br />
            we will Bloom
          </h1>

          <div className="hero-balance">
            <span className="hero-balance-value">{tBal} SOL</span>
            <span className="hero-balance-label">
              Treasury PDA · FNQbK1...otF
            </span>
          </div>

          <div className="hero-metrics">
            <div className="metric">
              <span className="metric-value">{tBal}</span>
              <span className="metric-label">Treasury SOL</span>
            </div>
            <div className="metric">
              <span className="metric-value">{uBal ?? "—"}</span>
              <span className="metric-label">{connected ? "Your Balance" : "Wallet"}</span>
            </div>
            <div className="metric">
              <span className="metric-value">{PROGRAM_ID.slice(0, 4)}...{PROGRAM_ID.slice(-4)}</span>
              <span className="metric-label">Program</span>
            </div>
          </div>
        </div>
      </section>

      {/* ── Mid section: feed + wings + invariants ──────────── */}
      <section className="mid-section">
        <div className="feed">
          <div className="feed-header">
            <span className="feed-title">Swarm Activity</span>
            <span className="feed-status">Demo Replay</span>
          </div>
          <div className="feed-body">
            {FEED_LINES.map((line, i) => (
              <div className="feed-line" key={i}>
                <span className="feed-ts">{line.ts}</span>
                <span
                  className={`feed-tag ${
                    line.tag === "validated"
                      ? "validated"
                      : line.tag === "approved"
                      ? "approved"
                      : ""
                  }`}
                >
                  {line.tag}
                </span>
                <span className="feed-msg">{line.msg}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="wings">
          <div className="wings-header">Wings</div>
          <ul className="wing-list">
            {WINGS.map((w) => (
              <li className="wing-item" key={w.name}>
                <span className="wing-name">{w.name}</span>
                <span className={`wing-status ${w.active ? "active" : ""}`}>
                  {w.status}
                </span>
              </li>
            ))}
          </ul>
        </div>

        <div className="invariants">
          <div className="invariants-header">Constitutional Invariants</div>
          {INVARIANTS.map((inv, i) => (
            <div className="invariant-item" key={i}>
              <span className="invariant-check">✓</span>
              <span>{inv}</span>
            </div>
          ))}
        </div>
      </section>

      {/* ── Bottom vitals ──────────────────────────────────── */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">4LvsHb...M8Ad</span>
          <span className="vital-label">Program ID</span>
        </div>
        <div className="vital">
          <span className="vital-value">FNQbK1...otF</span>
          <span className="vital-label">Treasury PDA</span>
        </div>
        <div className="vital">
          <span className="vital-value">70 / 20 / 10</span>
          <span className="vital-label">Redistribution Split</span>
        </div>
        <div className="vital">
          <span className="vital-value">Sustenance</span>
          <span className="vital-label">Current Phase</span>
        </div>
        <a
          className="vital-link"
          href="https://explorer.solana.com/address/FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF?cluster=devnet"
          target="_blank"
          rel="noopener noreferrer"
        >
          View on Explorer ↗
        </a>
      </footer>
    </div>
  );
}
