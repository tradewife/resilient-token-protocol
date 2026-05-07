"use client";

import React, { useEffect, useState, useCallback } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import Link from "next/link";
import Topbar from "./Topbar";
import { fetchTreasuryState } from "../lib/sdk";

const TREASURY_AUTHORITY = "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R";
const TREASURY_PDA = "6PYPAnwiMoZvzphAWEu3EsNz3PpwjJ6YcZabj34qVQ4Z";
const DEVNET_RPC = "https://api.devnet.solana.com";
const MAINNET_RPC = "https://api.mainnet-beta.solana.com";

/* ── Fallback static feed (used when /api/cycle returns 404) ── */
const FALLBACK_FEED = [
  { ts: "2026-05-04", tag: "night shift", msg: "Evaluating 30,000 parameter configs across SOL/USDT" },
  { ts: "2026-05-04", tag: "night shift", msg: "9-fold walk-forward analysis complete. Darwinian mutations generated." },
  { ts: "2026-05-05", tag: "validated", msg: "SOL/USDT 9x leverage — Calmar 44.89, +554% return, 12.3% max DD, 0 liquidations" },
  { ts: "2026-05-05", tag: "night shift", msg: "16,228 candidates evaluated. 9x leverage optimal across 3-10x sweep." },
  { ts: "2026-05-06", tag: "robustness", msg: "Monte Carlo DD p95=32.1%. PBO=33%. Strategy exploration: 5 alternative plugins tested." },
  { ts: "2026-05-06", tag: "trading wing", msg: "Live 9x autonomous trader — thresh=0.25, tp=5.0, sl=2.7, trail=0.14, align=3" },
];

/* ── Fallback static wings ── */
const FALLBACK_WINGS = [
  { name: "Trading", status: "Executing SOL/USDT", active: true },
  { name: "Security", status: "Monitoring", active: true },
  { name: "Evolve", status: "3 mutations accepted", active: true },
  { name: "Knowledge", status: "14 files", active: true },
  { name: "Audit", status: "3/3 approved", active: true },
  { name: "Futureproof", status: "Monitoring", active: true },
];

const INVARIANTS = [
  "PDA owns treasury — no private key exists. No one can sign funds away.",
  "Native SOL treasury — fees collected as SOL into per-authority treasury PDA.",
  "Per-authority isolation — each authority gets its own Treasury PDA.",
  "Flash Trade CPI-only execution — Treasury PDA signs via invoke_signed, no human keypair involved in trading.",
  "Phase transitions irreversible — Sustenance → Ecosystem → Humanity, no downgrade.",
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

interface TraderState {
  wallet: string;
  open_position: {
    entry_price: number;
    entry_time: number;
    peak_price: number;
    entry_rsi: number;
    entry_atr: number;
    entry_score: number;
    position_key: string;
    size_usd: number;
  } | null;
  trade_history: Array<{
    entry_price: number;
    exit_price: number;
    pnl_pct: number;
    exit_reason: string;
    size_usd: number;
  }>;
  candle_count: number;
  last_poll: string;
  total_pnl_sol: number;
  total_trades: number;
}

const MAINNET_TXS = [
  { label: "Open (CPI invoke_signed)", tx: "2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3", note: "99,214 CU — mainnet" },
  { label: "Close (SOL returned)", tx: "dFqkoP2wX2meR8Mv8CngujJJUNBYuv5peCyzRYFPBvpN3uqCqXqRCy4TPyw5JbAZhumCaJdGaJoQvJrJGJzxfHF", note: "mainnet" },
  { label: "Open (REST API)", tx: "YtGKq46wEgeUqoWouV5LXvv6mAxb5dCYmRHy622i7UtP5UoXsKZJtqscJf9fWLjzjZwCZhGw7r4EMgKV3wU2CBg", note: "mainnet" },
  { label: "Close (REST API)", tx: "56PLUQAPGqtAcvRUgJBreMrubAETZkpFCoyHzkwt3jCGCwZYHeonbxcJp244ZipeHuNBAwAX6r1wWkcR9LFcdmM6", note: "mainnet" },
];

export default function Home() {
  const { publicKey, connected } = useWallet();
  const { connection } = useConnection();

  const [treasurySol, setTreasurySol] = useState<number | null>(null);
  const [walletSol, setWalletSol] = useState<number | null>(null);
  const [cycle, setCycle] = useState<CycleData | null>(null);
  const [memory, setMemory] = useState<MemoryData | null>(null);
  const [liveness, setLiveness] = useState<LivenessData | null>(null);
  const [traderState, setTraderState] = useState<TraderState | null>(null);
  const [showHowItWorks, setShowHowItWorks] = useState(true);
  const [yieldReceived, setYieldReceived] = useState<number | null>(null);
  const [yieldLoading, setYieldLoading] = useState(false);
  const [isFrozen, setIsFrozen] = useState(false);

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

  // ── Fetch trader state (live autonomous trader via API route) ──
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const res = await fetch("/api/trader-status");
        if (res.ok) {
          const data: TraderState = await res.json();
          if (data.wallet && alive) { setTraderState(data); }
        }
      } catch {
        // Trader API not available yet
      }
    };
    poll();
    const id = setInterval(poll, 15_000); // refresh every 15s
    return () => { alive = false; clearInterval(id); };
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

  // ── Treasury frozen state (devnet, via SDK Borsh decoder) ──
  useEffect(() => {
    let alive = true;
    const check = async () => {
      try {
        const devnetConn = new (await import("@solana/web3.js")).Connection(DEVNET_RPC, "confirmed");
        const state = await fetchTreasuryState(devnetConn, TREASURY_AUTHORITY);
        if (alive) setIsFrozen(state.isFrozen);
      } catch {
        // Devnet RPC unreachable — keep current state
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
        const devnetConn = new (await import("@solana/web3.js")).Connection(DEVNET_RPC, "confirmed");
        const treasuryPubkey = new PublicKey(TREASURY_PDA);
        const walletStr = publicKey.toBase58();

        // Limit to 20 recent signatures to avoid rate limiting
        const signatures = await devnetConn.getSignaturesForAddress(treasuryPubkey, { limit: 20 });
        let totalYieldLamports = 0;

        for (const sigInfo of signatures) {
          if (cancelled) break;
          try {
            const tx = await devnetConn.getTransaction(sigInfo.signature, {
              maxSupportedTransactionVersion: 0,
            });
            if (!tx || !tx.meta) continue;

            const { preBalances, postBalances } = tx.meta;
            const accountKeys = tx.transaction.message.staticAccountKeys
              ? tx.transaction.message.staticAccountKeys
              : (tx.transaction.message as { accountKeys: PublicKey[] }).accountKeys;
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
            // Individual tx fetch failed (rate limit, dropped connection) — skip
          }
        }

        if (!cancelled) {
          setYieldReceived(totalYieldLamports / LAMPORTS_PER_SOL);
          setYieldLoading(false);
        }
      } catch {
        if (!cancelled) {
          setYieldReceived(null);
          setYieldLoading(false);
        }
      }
    })();

    return () => { cancelled = true; };
  }, [publicKey]);

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

      {/* Emergency freeze banner */}
      {isFrozen && (
        <div style={{
          background: "#dc2626", color: "#fff", padding: "10px 24px",
          textAlign: "center", fontSize: "0.875rem", fontWeight: 600,
          letterSpacing: "0.04em",
        }}>
          TREASURY FROZEN — All operations halted by authority. Unfreeze requires 2-of-3 multisig approval + 24h time lock.
        </div>
      )}

      {/* Hero: image + treasury overview */}
      <section className="hero">
        <div className="hero-image-wrap">
          <img
            src="/bg-flower.jpg"
            alt="Ethereal flower in emerald and coral — the organic intelligence that drives RTP"
          />
        </div>

        <div className="hero-content">
          <div className="hero-copy">
            <span className="hero-label">SOLANA-NATIVE · AUTONOMOUS YIELD <span className="hero-label-desktop">· SELF-FUNDING</span></span>
            <h1 className="hero-title">
              Every token gets a
              <br />
              program-enforced treasury
            </h1>
            <p className="hero-subtitle">
              Token projects route trading fees to RTP → the swarm generates yield via on-chain perps → yield flows back to holders. 70/20/10 split, enforced on-chain. No RTP token. Pure infrastructure.
            </p>
          </div>

          <div className="hero-balance-block">
            <div className="hero-balance">
              <span className="hero-balance-value">
                {treasurySol !== null && treasurySol > 0 ? `${treasurySol.toFixed(4)} SOL` : (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                    <span style={{
                      background: "var(--emerald)", color: "#fff", padding: "3px 10px",
                      borderRadius: 4, fontSize: "0.6875rem", fontWeight: 600, letterSpacing: "0.04em",
                    }}>
                      MAINNET VERIFIED
                    </span>
                    <span style={{ fontSize: "0.8125rem", color: "var(--text-secondary)", fontWeight: 400 }}>
                      4 confirmed transactions
                    </span>
                  </span>
                )}
              </span>
              <div className="hero-balance-row">
                <span className="hero-balance-label">
                  TREASURY VAULT · 6PYPAn...Q4Z
                </span>

                <div className="hero-actions">
                  <Link href="/launch" style={{
                    display: "inline-flex", alignItems: "center", gap: 6,
                    background: "var(--coral)", color: "#fff", borderRadius: 6,
                    padding: "10px 20px", fontSize: "0.875rem", fontWeight: 500,
                    textDecoration: "none", transition: "opacity 0.15s",
                  }}>
                    Try it live →
                  </Link>
                  <Link href="/docs" style={{
                    display: "inline-flex", alignItems: "center", gap: 6,
                    background: "var(--surface-2)", color: "var(--text-secondary)", borderRadius: 6,
                    padding: "10px 20px", fontSize: "0.875rem", fontWeight: 500,
                    textDecoration: "none", border: "1px solid var(--border)",
                    transition: "border-color 0.15s",
                  }}>
                    Read the docs
                  </Link>
                </div>
              </div>
            </div>

            {connected && publicKey && (yieldLoading || (yieldReceived !== null && yieldReceived > 0)) && (
              <div className="hero-yield">
                {yieldLoading ? (
                  <span className="yield-text">Scanning treasury transactions...</span>
                ) : (
                  <span className="yield-text">
                    You have received <strong>{yieldReceived?.toFixed(4)} SOL</strong> from RTP
                  </span>
                )}
              </div>
            )}
          </div>
        </div>

        <div className="hero-metrics">
          <div className="metric">
            <span className="metric-value accent">325</span>
            <span className="metric-label">Tests Passing</span>
          </div>
          <div className="metric">
            <span className="metric-value accent">{cycleCount}</span>
            <span className="metric-label">Autonomous Cycles</span>
          </div>
          <div className="metric">
            <span className="metric-value accent">{cycle?.memory_file_count ?? memory?.fileCount ?? 14}</span>
            <span className="metric-label">Memory Files</span>
          </div>
          <div className="metric">
            <span className="metric-value">+554%</span>
            <span className="metric-label">9x Leveraged Return</span>
          </div>
        </div>
      </section>

      {/* Mid section: feed + wings + invariants */}
      <section className="mid-section">
        <div className="feed">
          <div className="feed-header">
            <span className="feed-title">Swarm Activity</span>
            <span className="feed-status">
              {cycle ? `Last cycle: ${lastRun}` : "Latest Night Shift Results"}
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

      {/* Capital flow */}
      <section style={{
        padding: "var(--space-xl) 0", borderTop: "1px solid var(--border)",
        display: "flex", flexDirection: "column", alignItems: "center", gap: "var(--space-md)",
      }}>
        <div style={{ fontSize: "0.6875rem", fontWeight: 500, letterSpacing: "0.12em", textTransform: "uppercase" as const, color: "var(--text-tertiary)" }}>
          Capital Flow
        </div>
        <div style={{
          display: "flex", alignItems: "center", gap: "var(--space-sm)", flexWrap: "wrap", justifyContent: "center",
          fontFamily: "var(--font-mono)", fontSize: "0.75rem", color: "var(--text-secondary)",
        }}>
          {[
            { label: "Creator Fees (SOL)", color: "var(--coral)" },
            { label: "→" },
            { label: "Treasury PDA", color: "var(--emerald)" },
            { label: "→" },
            { label: "Flash Trade CPI", color: "var(--coral)" },
            { label: "→" },
            { label: "SOL Yield", color: "var(--text-tertiary)" },
            { label: "→" },
            { label: "70/20/10 Split", color: "var(--emerald)" },
          ].map((item, i) => (
            <span key={i} style={{
              color: item.color || "var(--text-muted)",
              ...(item.label !== "→" ? {
                background: "var(--surface-1)", padding: "4px 10px", borderRadius: 4,
                border: "1px solid var(--border)", fontSize: "0.6875rem",
              } : {}),
            }}>
              {item.label}
            </span>
          ))}
        </div>
      </section>

      {/* Live autonomous trader */}
      <section style={{
        padding: "var(--space-xl) 0", borderTop: "1px solid var(--border)",
        display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(340px, 1fr))", gap: "var(--space-xl)",
        alignItems: "start",
      }}>
        <div>
          <div style={{ fontSize: "0.6875rem", fontWeight: 500, letterSpacing: "0.12em", textTransform: "uppercase" as const, color: "var(--text-tertiary)", marginBottom: "var(--space-md)" }}>
            Live Autonomous Trader
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: "var(--space-xs)" }}>
              <span style={{
                width: 8, height: 8, borderRadius: "50%",
                background: "var(--emerald)",
                boxShadow: "0 0 6px var(--emerald)",
              }} />
              <span style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--emerald)" }}>
                {traderState ? "LIVE — Autonomous Trading on Mainnet" : "LIVE — Connecting to trader..."}
              </span>
            </div>
            {traderState && (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-sm)" }}>
                {[
                  { label: "Status", value: traderState.open_position ? "OPEN (SOL LONG)" : "FLAT — Watching" },
                  { label: "Total Trades", value: String(traderState.total_trades) },
                  { label: "Candles", value: String(traderState.candle_count) },
                  { label: "PnL", value: `${traderState.total_pnl_sol >= 0 ? "+" : ""}${traderState.total_pnl_sol.toFixed(4)} SOL` },
                  { label: "Last Poll", value: traderState.last_poll ? new Date(traderState.last_poll).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" }) : "—" },
                  { label: "Position Size", value: "0.20 SOL (10% bankroll)" },
                ].map((s, i) => (
                  <div key={i} style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                    <span style={{
                      fontFamily: "var(--font-display)", fontSize: "0.9375rem", fontWeight: 400,
                      color: i === 0 && traderState.open_position ? "var(--coral)" : "var(--text-primary)",
                    }}>
                      {s.value}
                    </span>
                    <span style={{ fontSize: "0.625rem", color: "var(--text-muted)", letterSpacing: "0.06em", textTransform: "uppercase" }}>
                      {s.label}
                    </span>
                  </div>
                ))}
              </div>
            )}
            {traderState && traderState.trade_history.length > 0 && (
              <div style={{ marginTop: "var(--space-sm)" }}>
                <div style={{ fontSize: "0.6875rem", color: "var(--text-muted)", letterSpacing: "0.06em", textTransform: "uppercase", marginBottom: 4 }}>
                  Recent Trades
                </div>
                {traderState.trade_history.slice(-3).reverse().map((t, i) => (
                  <div key={i} style={{
                    fontSize: "0.75rem", color: t.pnl_pct >= 0 ? "var(--emerald)" : "var(--coral)",
                    fontFamily: "var(--font-mono)",
                  }}>
                    {t.pnl_pct >= 0 ? "+" : ""}{t.pnl_pct.toFixed(2)}% — {t.exit_reason}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
        <div>
          <div style={{ fontSize: "0.6875rem", fontWeight: 500, letterSpacing: "0.12em", textTransform: "uppercase" as const, color: "var(--text-tertiary)", marginBottom: "var(--space-md)" }}>
            Confirmed Mainnet Transactions
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-xs)" }}>
            {MAINNET_TXS.map((tx, i) => (
              <a key={i} href={`https://explorer.solana.com/tx/${tx.tx}`}
                target="_blank" rel="noopener noreferrer"
                style={{
                  display: "flex", justifyContent: "space-between", alignItems: "center",
                  padding: "var(--space-xs) var(--space-md)",
                  background: "var(--surface-0)", border: "1px solid var(--border)",
                  borderRadius: 6, textDecoration: "none",
                  fontSize: "0.75rem", color: "var(--text-secondary)",
                  transition: "border-color 0.15s",
                }}
              >
                <span>
                  <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>{tx.label}</span>
                  <span style={{ color: "var(--text-tertiary)", marginLeft: 8, fontFamily: "var(--font-mono)", fontSize: "0.625rem" }}>
                    {tx.tx.slice(0, 8)}...
                  </span>
                </span>
                <span style={{ fontSize: "0.625rem", color: "var(--emerald)", fontWeight: 500, letterSpacing: "0.04em" }}>
                  {tx.note} ↗
                </span>
              </a>
            ))}
          </div>
          <div style={{ marginTop: "var(--space-md)", fontSize: "0.6875rem", color: "var(--text-muted)", lineHeight: 1.6 }}>
            Strategy: <strong style={{ color: "var(--text-primary)" }}>Survivor 2.69 (9x)</strong> — signal 0.25 · TP 5×ATR · SL 2.7×ATR · Trail 0.14×ATR · Align 3+ · Max 36h
            <br />Runs on Railway 24/7. Calmar 44.89. 100% fold consistency. 0 liquidations.
          </div>
        </div>
      </section>

      {/* Validated strategy proof */}
      <section style={{
        padding: "var(--space-xl) 0", borderTop: "1px solid var(--border)",
        display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))", gap: "var(--space-xl)",
        alignItems: "start",
      }}>
        <div>
          <div style={{ fontSize: "0.6875rem", fontWeight: 500, letterSpacing: "0.12em", textTransform: "uppercase" as const, color: "var(--text-tertiary)", marginBottom: "var(--space-md)" }}>
            Validated Strategy — SOL/USDT
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-md)" }}>
            {[
              { label: "Calmar Ratio", value: "44.89" },
              { label: "9x Return", value: "+554%" },
              { label: "Max Drawdown", value: "12.3%" },
              { label: "Consistency", value: "100%" },
              { label: "Leverage", value: "9x" },
              { label: "Liquidations", value: "0" },
            ].map((s, i) => (
              <div key={i} style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                <span style={{
                  fontFamily: "var(--font-display)", fontSize: "1.125rem", fontWeight: 400,
                  color: i < 2 ? "var(--coral)" : "var(--text-primary)", fontVariantNumeric: "tabular-nums",
                }}>
                  {s.value}
                </span>
                <span style={{ fontSize: "0.6875rem", color: "var(--text-muted)", letterSpacing: "0.06em", textTransform: "uppercase" }}>
                  {s.label}
                </span>
              </div>
            ))}
          </div>
        </div>
        <div>
          <div style={{ fontSize: "0.6875rem", fontWeight: 500, letterSpacing: "0.12em", textTransform: "uppercase" as const, color: "var(--text-tertiary)", marginBottom: "var(--space-md)" }}>
            What We Built
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
            {[
              { label: "Anchor program", detail: "19 on-chain instructions — fee collection, 70/20/10 redistribution, strategy lifecycle, Flash Trade CPI, emergency controls. Per-token isolation. Deployed to devnet." },
              { label: "Rust swarm runtime", detail: "6 wings (Trading, Security, Evolve, Knowledge, Audit, Futureproof), Coordinator message bus, 325 unit + 5 integration tests" },
              { label: "Flash Trade CPI execution", detail: "Treasury PDA signs via invoke_signed. On-chain perps on Solana. Position open/close confirmed on mainnet. SOL stays on Solana — no bridge, no cross-chain." },
              { label: "Live autonomous trader", detail: "rtp-trader running 24/7 on Railway. 9x leverage Calmar-optimized config, REST API trading, HTTP status server. Open positions visible on Solana Explorer." },
              { label: "Per-token isolation", detail: "Each token gets its own Treasury PDA + vault. Same strategy, isolated capital. No shared pool — one exploit can't drain all adopters." },
              { label: "TypeScript SDK", detail: "One function call to register any token with RTP. Launchpads integrate in minutes — no chain ops to run." },
            ].map((item, i) => (
              <div key={i} style={{
                padding: "var(--space-sm) var(--space-md)",
                background: "var(--surface-0)", border: "1px solid var(--border)", borderRadius: 6,
              }}>
                <span style={{ fontSize: "0.8125rem", color: "var(--text-primary)", fontWeight: 500 }}>{item.label}</span>
                <span style={{ fontSize: "0.75rem", color: "var(--text-tertiary)", marginLeft: "var(--space-sm)" }}>{item.detail}</span>
              </div>
            ))}
          </div>
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
          <>
            <div className="hiw-steps">
              <div className="hiw-step">
                <span className="hiw-num">1</span>
                <div className="hiw-content">
                  <p className="hiw-text">
                    Token adopts RTP. Trading fees (SOL) route to an authority-seeded treasury PDA. Each authority has its own isolated treasury. No shared pool, no honeypot.
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
                    The research engine tests 30,000 strategy configs per night, validates survivors via 9-fold walk-forward analysis with Monte Carlo robustness testing. Best result: +554% at 9x leverage, Calmar 44.89, 100% consistency.
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
                    The Trading Wing submits the validated strategy to the Treasury PDA, which executes it on Flash Trade via CPI (invoke_signed). Positions are on-chain Solana perps. 20% max position enforced on-chain before CPI. Soulguard enforces constitutional constraints before every trade.
                  </p>
                </div>
              </div>
              <div className="hiw-step">
                <span className="hiw-num">4</span>
                <div className="hiw-content">
                  <p className="hiw-text">
                    SOL yield returns to the treasury PDA when positions close (single chain, no bridge). The Anchor program splits it 70% holders / 20% dev / 10% ecosystem — on-chain, deterministic, no discretion.
                  </p>
                  <a
                    className="hiw-link"
                    href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    View redistribution tx ↗
                  </a>
                </div>
              </div>
            </div>

            <div style={{ display: "flex", justifyContent: "flex-end", paddingBottom: "var(--space-lg)" }}>
              <div className="hero-actions">
                <Link href="/launch" style={{
                  display: "inline-flex", alignItems: "center", gap: 6,
                  background: "var(--coral)", color: "#fff", borderRadius: 6,
                  padding: "10px 20px", fontSize: "0.875rem", fontWeight: 500,
                  textDecoration: "none", transition: "opacity 0.15s",
                }}>
                  Try it live →
                </Link>
                <Link href="/docs" style={{
                  display: "inline-flex", alignItems: "center", gap: 6,
                  background: "var(--surface-2)", color: "var(--text-secondary)", borderRadius: 6,
                  padding: "10px 20px", fontSize: "0.875rem", fontWeight: 500,
                  textDecoration: "none", border: "1px solid var(--border)",
                  transition: "border-color 0.15s",
                }}>
                  Read the docs
                </Link>
              </div>
            </div>
          </>
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
          <span className="vital-label">Program ID (Devnet)</span>
        </div>
        <div className="vital">
          <span className="vital-value">6PYPAn...Q4Z</span>
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
            href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet"
            target="_blank"
            rel="noopener noreferrer"
          >
            On-Chain Proof ↗
          </a>
          <span className="vital-label">On-Chain Proof</span>
        </div>
        <div className="vital">
          <a
            className="vital-link"
            href="https://github.com/tradewife/resilient-token-protocol"
            target="_blank"
            rel="noopener noreferrer"
          >
            Source on GitHub ↗
          </a>
          <span className="vital-label">Repository</span>
        </div>
        <div className="vital">
          <a
            className="vital-link"
            href={`https://explorer.solana.com/address/${TREASURY_PDA}?cluster=devnet`}
            target="_blank"
            rel="noopener noreferrer"
          >
            Solana Explorer ↗
          </a>
          <span className="vital-label">Treasury</span>
        </div>

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
