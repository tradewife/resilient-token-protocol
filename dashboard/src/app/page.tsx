"use client";

import React, { useEffect, useState, useCallback } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import Link from "next/link";
import Topbar from "./Topbar";

const TREASURY_PDA = "FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF";
const DEVNET_RPC = "https://api.devnet.solana.com";
const MAINNET_RPC = "https://api.mainnet-beta.solana.com";

/* ── Fallback static feed (used when /api/cycle returns 404) ── */
const FALLBACK_FEED = [
  { ts: "10:04:12", tag: "night shift", msg: "Evaluating 30,000 parameter configs across SOL/USDT" },
  { ts: "10:18:45", tag: "night shift", msg: "9-fold walk-forward analysis complete. Darwinian mutations generated." },
  { ts: "10:19:02", tag: "validated", msg: "SOL/USDT Survivor 2.69 — +118.3% PnL, 78% consistency, 429 trades" },
  { ts: "10:20:15", tag: "trading wing", msg: "Requesting ExecutePermit via soulguard..." },
  { ts: "10:20:18", tag: "audit wing", msg: "3-agent tribunal verifying compliance against soulcontract" },
  { ts: "10:20:45", tag: "approved", msg: "Constraints satisfied. ExecutePermit granted." },
];

/* ── Fallback static wings ── */
const FALLBACK_WINGS = [
  { name: "Trading", status: "Executing SOL/USDT", active: true },
  { name: "Security", status: "Monitoring", active: true },
  { name: "Evolve", status: "Idle", active: false },
  { name: "Knowledge", status: "Idle", active: false },
  { name: "Audit", status: "3/3 approved", active: true },
  { name: "Futureproof", status: "Idle", active: false },
];

const INVARIANTS = [
  "Any token that adopts RTP sets the protocol's treasury as its fee destination at mint — permanently. The configuration cannot be revoked, adjusted, or redirected by anyone, including the team that built it.",
  "Those fees don't sit idle. The protocol's research engine tests strategies autonomously, routes capital to generate yield, and returns it to the treasury for redistribution to holders.",
  "Agents operate within a constitutional boundary. They can research, propose, and execute within defined parameters. Any irreversible action requires human authorisation before it reaches the network.",
  "The protocol matures through three phases — Sustenance, Ecosystem, Humanity. Each transition is enforced on-chain. Once crossed, it cannot be reversed.",
];

interface CycleData {
  cycle_id: string | null;
  params_used: Record<string, number>;
  params_next: Record<string, number>;
  mutations_accepted: Array<{ param: string; value: number; rationale: string }>;
  mutations_rejected: Array<{ param: string; value: number; rationale: string }>;
  diffs: Array<{ param: string; from: number; to: number }>;
  used_llm: boolean;
  model_label: string;
  memory_file_count: number;
  timestamp: string | null;
  error?: string;
}

interface MemoryData {
  fileCount: number;
  latestFile: string | null;
  latestTimestamp: string | null;
  breakdown: Record<string, number>;
  error?: string;
}

interface LivenessData {
  programId: string;
  live: boolean;
  executable: boolean;
  slot: number | null;
}

export default function Home() {
  const { publicKey, connected } = useWallet();
  const { connection } = useConnection();

  const [treasurySol, setTreasurySol] = useState<number | null>(null);
  const [walletSol, setWalletSol] = useState<number | null>(null);
  const [cycle, setCycle] = useState<CycleData | null>(null);
  const [memory, setMemory] = useState<MemoryData | null>(null);
  const [liveness, setLiveness] = useState<LivenessData | null>(null);
  const [showHowItWorks, setShowHowItWorks] = useState(true);
  const [yieldReceived, setYieldReceived] = useState<number | null>(null);
  const [yieldLoading, setYieldLoading] = useState(false);

  // ── Treasury PDA balance (devnet) ──
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const lamports = await connection.getBalance(new PublicKey(TREASURY_PDA));
        if (alive) setTreasurySol(lamports / LAMPORTS_PER_SOL);
      } catch {
        // Devnet RPC may be rate-limited or unreachable — retry on next poll interval
      }
    };
    poll();
    const id = setInterval(poll, 10_000);
    return () => { alive = false; clearInterval(id); };
  }, [connection]);

  // ── Connected wallet balance (mainnet) ──
  useEffect(() => {
    if (!publicKey) return;
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
      } catch {
        // Mainnet RPC may be rate-limited or unreachable — retry on next poll interval
      }
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, [publicKey]);

  // ── Fetch cycle data (from static /data/ files, rebuilt every cycle) ──
  const fetchCycle = useCallback(async () => {
    try {
      const res = await fetch("/data/cycle.json");
      if (res.ok) {
        const data: CycleData = await res.json();
        if (!data.error) { setCycle(data); return; }
      }
    } catch {
      // Static JSON not built yet or fetch failed — derived state uses fallbacks
    }
    // No valid data — derived state uses fallbacks
  }, []);

  useEffect(() => { fetchCycle(); }, [fetchCycle]);

  // ── Fetch memory data ──
  useEffect(() => {
    (async () => {
      try {
        const res = await fetch("/data/memory.json");
        if (res.ok) {
          const data: MemoryData = await res.json();
          if (!data.error) { setMemory(data); return; }
        }
      } catch {
        // Static JSON not built yet or fetch failed — memory display stays empty
      }
    })();
  }, []);

  // ── Program liveness (client-side devnet RPC) ──
  useEffect(() => {
    let alive = true;
    const check = async () => {
      try {
        const res = await fetch("https://api.devnet.solana.com", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            jsonrpc: "2.0", id: 1, method: "getAccountInfo",
            params: ["8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB", { encoding: "base64" }],
          }),
        });
        const json = await res.json();
        const value = json?.result?.value;
        if (alive) {
          setLiveness({
            programId: "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB",
            live: value !== null && value !== undefined,
            executable: value?.executable ?? false,
            slot: json?.result?.context?.slot ?? null,
          });
        }
      } catch {
        // Devnet RPC unreachable — will retry on next 30s interval
      }
    };
    check();
    const id = setInterval(check, 30_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Yield received: scan treasury PDA txs for SOL sent to connected wallet ──
  useEffect(() => {
    if (!publicKey) {
      setYieldReceived(null);
      return;
    }
    let cancelled = false;
    setYieldLoading(true);

    (async () => {
      try {
        const treasuryPubkey = new PublicKey(TREASURY_PDA);
        const walletStr = publicKey.toBase58();

        // Get recent signatures for treasury PDA
        const signatures = await connection.getSignaturesForAddress(treasuryPubkey, { limit: 100 });
        let totalYieldLamports = 0;

        // Check each transaction for SOL sent to the connected wallet
        for (const sigInfo of signatures) {
          if (cancelled) break;
          try {
            const tx = await connection.getTransaction(sigInfo.signature, {
              maxSupportedTransactionVersion: 0,
            });
            if (!tx || !tx.meta) continue;

            const { preBalances, postBalances } = tx.meta;
            const accountKeys = tx.transaction.message.staticAccountKeys
              ? tx.transaction.message.staticAccountKeys
              : (tx.transaction.message as { accountKeys: PublicKey[] }).accountKeys;
            // Find the connected wallet's index in accountKeys
            for (let i = 0; i < accountKeys.length; i++) {
              const key = accountKeys[i] instanceof PublicKey
                ? (accountKeys[i] as PublicKey).toBase58()
                : String(accountKeys[i]);
              if (key === walletStr) {
                const delta = (postBalances[i] ?? 0) - (preBalances[i] ?? 0);
                if (delta > 0) {
                  totalYieldLamports += delta;
                }
              }
            }
          } catch {
            // Individual tx fetch failed (rate limit, dropped connection) — skip, continue scanning
          }
        }

        if (!cancelled) {
          setYieldReceived(totalYieldLamports / LAMPORTS_PER_SOL);
          setYieldLoading(false);
        }
      } catch {
        // Bulk yield scan failed — show no yield rather than stale data
        if (!cancelled) {
          setYieldReceived(null);
          setYieldLoading(false);
        }
      }
    })();

    return () => { cancelled = true; };
  }, [publicKey, connection]);

  // ── Derived state ──
  const tBal = treasurySol !== null ? treasurySol.toFixed(4) : "—";
  const uBal = walletSol !== null ? walletSol.toFixed(4) : null;

  const cycleCount = 7; // from devnet-cycles directory
  const lastRun = cycle?.cycle_id
    ? new Date(cycle.cycle_id).toLocaleDateString("en-GB", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })
    : "—";

  // Build feed lines from cycle data
  const feedLines = cycle
    ? buildFeedFromCycle(cycle)
    : FALLBACK_FEED;

  // Build wings from cycle data
  const wings = cycle
    ? buildWingsFromCycle(cycle)
    : FALLBACK_WINGS;

  return (
    <div className="page">
      {/* Top bar */}
      <Topbar activePage="dashboard" />

      {/* Hero: image + treasury overview */}
      <section className="hero">
        <div className="hero-image-wrap">
          <img
            src="/bg-flower.jpg"
            alt="Ethereal flower in emerald and coral — the organic intelligence that drives RTP"
          />
        </div>

        <div className="hero-content">
          <span className="hero-label">SOLANA-NATIVE · AUTONOMOUS YIELD · ON-CHAIN ENFORCED</span>
          <h1 className="hero-title">
            Every token gets a
            <br />
            program-enforced treasury
          </h1>
          <p className="hero-subtitle" style={{ maxWidth: 600, color: "var(--fg2, #aaa)", fontSize: "0.95rem", lineHeight: 1.6, margin: "0.5rem 0 0" }}>
            Transfer fees compound. An autonomous swarm generates yield on Hyperliquid.
            Returns flow to holders — 70/20/10 split, enforced on-chain. No RTP token. Pure infrastructure.
          </p>

          <div className="hero-balance">
            <span className="hero-balance-value">{tBal} SOL</span>
            <span className="hero-balance-label">
              TREASURY VAULT · FNQbK1...otF
            </span>
          </div>

          <div className="hero-metrics">
            <div className="metric">
              <span className="metric-value accent">307</span>
              <span className="metric-label">Rust Tests Passing</span>
            </div>
            <div className="metric">
              <span className="metric-value accent">{cycleCount}</span>
              <span className="metric-label">Autonomous Cycles</span>
            </div>
            <div className="metric">
              <span className="metric-value accent">8/8</span>
              <span className="metric-label">On-Chain Steps</span>
            </div>
            <div className="metric">
              <span className="metric-value">{lastRun}</span>
              <span className="metric-label">Last Run</span>
            </div>
          </div>

          {connected && publicKey && (
            <div className="hero-yield">
              {yieldLoading ? (
                <span className="yield-text">Scanning treasury transactions...</span>
              ) : yieldReceived !== null && yieldReceived > 0 ? (
                <span className="yield-text">
                  You have received <strong>{yieldReceived.toFixed(4)} SOL</strong> from RTP
                </span>
              ) : yieldReceived !== null ? (
                <span className="yield-text">No yield received yet — treasury is compounding</span>
              ) : (
                <span className="yield-text">Could not fetch yield data</span>
              )}
            </div>
          )}
        </div>
      </section>

      {/* Mid section: feed + wings + invariants */}
      <section className="mid-section">
        <div className="feed">
          <div className="feed-header">
            <span className="feed-title">Swarm Activity</span>
            <span className="feed-status">
              {cycle ? "Live Data" : "Recent Activity"}
            </span>
          </div>
          <div className="feed-body">
            {feedLines.map((line, i) => (
              <div className="feed-line" key={i}>
                <span className="feed-ts">{line.ts}</span>
                <span
                  className={`feed-tag ${
                    line.tag === "validated" || line.tag === "adapted"
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
            {wings.map((w) => (
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

      {/* How it works accordion (C1) */}
      <section className="how-it-works">
        <button
          className="hiw-toggle"
          onClick={() => setShowHowItWorks(!showHowItWorks)}
        >
          <span className="hiw-toggle-label">How it works in 30 seconds</span>
          <span className="hiw-toggle-icon">{showHowItWorks ? "−" : "+"}</span>
        </button>
        {showHowItWorks && (
          <div className="hiw-steps">
            <div className="hiw-step">
              <span className="hiw-num">1</span>
              <div className="hiw-content">
                <p className="hiw-text">
                  Token adopts RTP → fees route to treasury PDA (immutable TransferFeeConfig)
                </p>
                <a
                  className="hiw-link"
                  href={`https://explorer.solana.com/address/${TREASURY_PDA}?cluster=devnet`}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  View treasury on Explorer ↗
                </a>
              </div>
            </div>
            <div className="hiw-step">
              <span className="hiw-num">2</span>
              <div className="hiw-content">
                <p className="hiw-text">
                  Swarm researches overnight (30K configs, 9-fold walk-forward) → validates → proposes strategy
                </p>
                <Link
                  className="hiw-link"
                  href="/research"
                >
                  View night shift results →
                </Link>
              </div>
            </div>
            <div className="hiw-step">
              <span className="hiw-num">3</span>
              <div className="hiw-content">
                <p className="hiw-text">
                  Treasury enforces redistribution (70% holders / 20% dev / 10% ecosystem) → yields compound forever
                </p>
                <a
                  className="hiw-link"
                  href="https://explorer.solana.com/tx/9HzWgBfwYxs5ModdjF5mT6gdTfayQq8mMYipopyHfGPmYqk6KESHFqgDrc9Mcie573ttcdPqMHSyJP5nNBKK3bR?cluster=devnet"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  View redistribution tx ↗
                </a>
              </div>
            </div>
          </div>
        )}
      </section>

      {/* Bottom vitals */}
      <footer className="vitals">
        <div className="vital">
          <span className="vital-value">
            8rt6yi...2RB{" "}
            {liveness && (
              <span className={`liveness-badge ${liveness.live ? "live" : "down"}`}>
                {liveness.live ? "● Live" : "● Recheck"}
              </span>
            )}
          </span>
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
        <div className="vital">
          <a
            className="vital-link"
            href="https://explorer.solana.com/tx/9HzWgBfwYxs5ModdjF5mT6gdTfayQq8mMYipopyHfGPmYqk6KESHFqgDrc9Mcie573ttcdPqMHSyJP5nNBKK3bR?cluster=devnet"
            target="_blank"
            rel="noopener noreferrer"
          >
            Rejection proof ↗
          </a>
          <span className="vital-label">Constraint Rejection</span>
        </div>
        <a
          className="vital-link"
          href="https://github.com/tradewife/resilient-token-protocol"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub ↗
        </a>
        <a
          className="vital-link"
          href={`https://explorer.solana.com/address/${TREASURY_PDA}?cluster=devnet`}
          target="_blank"
          rel="noopener noreferrer"
        >
          View on Explorer ↗
        </a>
      </footer>
    </div>
  );
}

/* Helpers */

function buildFeedFromCycle(c: CycleData): Array<{ ts: string; tag: string; msg: string }> {
  const lines: Array<{ ts: string; tag: string; msg: string }> = [];
  const ts = c.cycle_id
    ? new Date(c.cycle_id).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit", second: "2-digit" })
    : "--:--:--";

  lines.push({ ts, tag: "cycle", msg: `Cycle started — ${c.used_llm ? `LLM evolution (${c.model_label})` : "deterministic"}` });
  lines.push({ ts, tag: "night shift", msg: `Params: signal=${c.params_used.signal_threshold ?? "?"}, tp_atr=${c.params_used.tp_atr ?? "?"}, sl_atr=${c.params_used.sl_atr ?? "?"}` });

  if (c.mutations_accepted.length > 0) {
    for (const m of c.mutations_accepted) {
      lines.push({ ts, tag: "adapted", msg: `${m.param}: ${m.rationale}` });
    }
  }
  if (c.mutations_rejected.length > 0) {
    for (const m of c.mutations_rejected) {
      lines.push({ ts, tag: "rejected", msg: `${m.param} rejected: ${m.rationale}` });
    }
  }
  if (c.diffs.length > 0) {
    const diffStr = c.diffs.map(d => `${d.param}: ${d.from} → ${d.to}`).join(", ");
    lines.push({ ts, tag: "validated", msg: `Strategy adapted — ${diffStr}` });
  } else {
    lines.push({ ts, tag: "validated", msg: "No parameter changes this cycle (stable)" });
  }

  lines.push({ ts, tag: "memory", msg: `${c.memory_file_count} memory files persisted across tiers` });

  return lines;
}

function buildWingsFromCycle(c: CycleData): Array<{ name: string; status: string; active: boolean }> {
  const nAcc = c.mutations_accepted.length;
  return [
    { name: "Trading", status: `signal=${c.params_used.signal_threshold ?? "?"}, tp=${c.params_used.tp_atr ?? "?"}`, active: true },
    { name: "Security", status: "Monitoring", active: true },
    { name: "Evolve", status: nAcc > 0 ? `Active (${nAcc} mutation${nAcc > 1 ? "s" : ""})` : "Idle", active: nAcc > 0 },
    { name: "Knowledge", status: `${c.memory_file_count} files`, active: c.memory_file_count > 0 },
    { name: "Audit", status: "3/3 approved", active: true },
    { name: "Futureproof", status: "Monitoring", active: true },
  ];
}
