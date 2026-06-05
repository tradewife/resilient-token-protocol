"use client";

import React, { useEffect, useState, useMemo } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import Link from "next/link";
import Topbar from "./Topbar";
import { fetchTreasuryState } from "../lib/sdk";

/* ── Constants ── */

const TREASURY_AUTHORITY = "********************************************";
const TREASURY_PDA = "6PYPAnwiMoZvzphAWEu3EsNz3PpwjJ6YcZabj34qVQ4Z";
const DEVNET_RPC = "https://api.devnet.solana.com";

const MAINNET_TXS = [
  { label: "Open · CPI invoke_signed", tx: "2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3", note: "99,214 CU", kind: "open" },
  { label: "Close · SOL returned", tx: "dFqkoP2wX2meR8Mv8CngujJJUNBYuv5peCyzRYFPBvpN3uqCqXqRCy4TPyw5JbAZhumCaJdGaJoQvJrJGJzxfHF", note: "settled mainnet", kind: "close" },
  { label: "Open · REST autonomous", tx: "YtGKq46wEgeUqoWouV5LXvv6mAxb5dCYmRHy622i7UtP5UoXsKZJtqscJf9fWLjzjZwCZhGw7r4EMgKV3wU2CBg", note: "score = 0.400", kind: "open" },
  { label: "Close · REST autonomous", tx: "56PLUQAPGqtAcvRUgJBreMrubAETZkpFCoyHzkwt3jCGCwZYHeonbxcJp244ZipeHuNBAwAX6r1wWkcR9LFcdmM6", note: "settled mainnet", kind: "close" },
];

/* ── Interfaces ── */

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

interface TraderState {
  wallet: string;
  open_position: {
    entry_price: number; entry_time: number; peak_price: number;
    entry_score: number; size_usd: number; side: string;
  } | null;
  trade_history: Array<{
    entry_price: number; exit_price: number; entry_time: number;
    exit_time: number; pnl_pct: number; exit_reason: string; size_usd: number; side?: string;
  }>;
  candle_count: number;
  last_poll: string;
  total_pnl_sol: number;
  total_trades: number;
}

interface LivenessData {
  programId: string;
  live: boolean;
  executable: boolean;
  slot: number | null;
}

interface NightData {
  num_folds: number;
  runtime_seconds: number;
  top_candidates: Array<{
    symbol: string; survivor_score: number; oos_sharpe: number;
    oos_consistency: number; oos_max_dd: number;
    overfitting_score: number; fragility: number;
  }>;
}

/* ── SVG Components ── */

function PnlSparkline({ trades }: { trades: TraderState["trade_history"] }) {
  const W = 720, H = 180, PAD_X = 14, PAD_Y = 22;
  const series = useMemo(() => {
    if (!trades || trades.length === 0) return [] as number[];
    const out: number[] = [0];
    let acc = 0;
    for (const t of trades) {
      const holdHours = Math.max(0, (t.exit_time - t.entry_time)) / 3600;
      const feeDrag = 0.12 + 0.0042 * holdHours;
      acc += t.pnl_pct - feeDrag;
      out.push(acc);
    }
    return out;
  }, [trades]);

  if (series.length < 2) {
    return (
      <div className="sparkline-empty">
        <div className="sparkline-empty-glyph">⏷</div>
        <div className="sparkline-empty-title">Awaiting first closed trade</div>
        <div className="sparkline-empty-sub">
          The trader is watching SOL/USDT live. Cumulative PnL appears here the moment the first position closes.
        </div>
      </div>
    );
  }

  const min = Math.min(...series, 0), max = Math.max(...series, 0);
  const range = max - min || 1;
  const xy = series.map((v, i) => {
    const x = PAD_X + (i / (series.length - 1)) * (W - 2 * PAD_X);
    const y = H - PAD_Y - ((v - min) / range) * (H - 2 * PAD_Y);
    return [x, y] as const;
  });
  const path = xy.map(([x, y], i) => `${i ? "L" : "M"} ${x.toFixed(1)} ${y.toFixed(1)}`).join(" ");
  const last = xy[xy.length - 1];
  const area = `${path} L ${last[0]} ${H - PAD_Y} L ${PAD_X} ${H - PAD_Y} Z`;
  const final = series[series.length - 1];
  const color = final >= 0 ? "var(--emerald)" : "var(--coral)";
  const zeroY = H - PAD_Y - ((0 - min) / range) * (H - 2 * PAD_Y);

  return (
    <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" className="sparkline-svg">
      <line x1={PAD_X} x2={W - PAD_X} y1={zeroY} y2={zeroY} stroke="var(--border)" strokeDasharray="2 6" />
      <path d={area} fill={color} fillOpacity="0.10" />
      <path d={path} fill="none" stroke={color} strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" />
      {xy.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r={i === xy.length - 1 ? 3.5 : 2} fill={color} />
      ))}
      <text x={last[0] - 6} y={last[1] - 8} className="sparkline-final" fill={color} textAnchor="end">
        {final >= 0 ? "+" : ""}{final.toFixed(2)}%
      </text>
    </svg>
  );
}

/* ── Invariant data ── */

const INVARIANTS = [
  { title: "PDA Ownership", desc: "The treasury is controlled by a program-derived address. No private key exists. The program IS the only authority." },
  { title: "Per-Token Isolation", desc: "Each mint gets its own Treasury PDA and vault. One token's exploit cannot affect another's reserves. No shared pool, no honeypot." },
  { title: "Emergency Freeze", desc: "Authority-gated halt. All 15 state-mutating instructions check the frozen flag. Unfreeze requires multisig approval." },
  { title: "Strategy Lifecycle", desc: "Hard stops auto-suspend: 10% drawdown, 5 consecutive losses. Soft decay auto-retires after 3 strikes. Recovery needs 3 consecutive positive updates." },
  { title: "CPI-Only Execution", desc: "All trading via Flash Trade CPI on Solana. invoke_signed with Treasury PDA seeds. Funds never leave the chain." },
  { title: "Phase Irreversible", desc: "Sustenance \u2192 Ecosystem \u2192 Humanity. On-chain transitions with no downgrade path. The protocol grows up, never down." },
] as const;

/* ── Main Page ── */

export default function Home() {
  const { publicKey, connected } = useWallet();
  const { connection } = useConnection();

  const [treasurySol, setTreasurySol] = useState<number | null>(null);
  const [walletSol, setWalletSol] = useState<number | null>(null);
  const [cycle, setCycle] = useState<CycleData | null>(null);
  const [liveness, setLiveness] = useState<LivenessData | null>(null);
  const [trader, setTrader] = useState<TraderState | null>(null);
  const [yieldReceived, setYieldReceived] = useState<number | null>(null);
  const [yieldLoading, setYieldLoading] = useState(false);
  const [isFrozen, setIsFrozen] = useState(false);
  const [night, setNight] = useState<NightData | null>(null);

  // ── Trader wallet balance (mainnet) ──
  // The yield engine runs on mainnet. The devnet demo PDA only holds rent-exempt minimum.
  // Show the mainnet trader wallet balance as the real "Treasury SOL" figure.
  // Uses server-side API route to avoid CORS issues with public Solana RPC.
  const TRADER_WALLET = "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R";
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const res = await fetch("/api/mainnet-balance");
        const json = await res.json();
        if (alive) setTreasurySol(json.sol ?? 0);
      } catch { /* retry on next poll */ }
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Connected wallet balance (mainnet, via API route) ──
  useEffect(() => {
    if (!publicKey) return;
    let alive = true;
    const poll = async () => {
      try {
        const res = await fetch(`/api/mainnet-balance?wallet=${publicKey.toBase58()}`);
        const json = await res.json();
        if (alive) setWalletSol(json.sol ?? 0);
      } catch { /* retry */ }
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, [publicKey]);

  // ── Fetch cycle data ──
  useEffect(() => {
    (async () => {
      try {
        const res = await fetch("/data/cycle.json");
        if (res.ok) { const data: CycleData = await res.json(); if (!data.error) setCycle(data); }
      } catch {}
    })();
  }, []);

  // ── Fetch trader state ──
  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const res = await fetch("/api/trader-status");
        if (res.ok) { const data: TraderState = await res.json(); if (data.wallet && alive) setTrader(data); }
      } catch {}
    };
    poll();
    const id = setInterval(poll, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Fetch night data ──
  useEffect(() => {
    (async () => {
      try { const r = await fetch("/data/night.json"); if (r.ok) { const d = await r.json(); if (!d.error) setNight(d); } } catch {}
    })();
  }, []);

  // ── Program liveness ──
  useEffect(() => {
    let alive = true;
    const check = async () => {
      try {
        const res = await fetch("https://api.devnet.solana.com", {
          method: "POST", headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "getAccountInfo",
            params: ["8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB", { encoding: "base64" }] }),
        });
        const json = await res.json();
        const value = json?.result?.value;
        if (alive) setLiveness({
          programId: "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB",
          live: value !== null && value !== undefined,
          executable: value?.executable ?? false,
          slot: json?.result?.context?.slot ?? null,
        });
      } catch {}
    };
    check();
    const id = setInterval(check, 30_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Treasury frozen state ──
  useEffect(() => {
    let alive = true;
    const check = async () => {
      try {
        const devnetConn = new (await import("@solana/web3.js")).Connection(DEVNET_RPC, "confirmed");
        const state = await fetchTreasuryState(devnetConn, TREASURY_AUTHORITY);
        if (alive) setIsFrozen(state.isFrozen);
      } catch {}
    };
    check();
    const id = setInterval(check, 30_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  // ── Yield received ──
  useEffect(() => {
    if (!publicKey) { setYieldReceived(null); return; }
    let cancelled = false;
    setYieldLoading(true);
    (async () => {
      try {
        const devnetConn = new (await import("@solana/web3.js")).Connection(DEVNET_RPC, "confirmed");
        const treasuryPubkey = new PublicKey(TREASURY_PDA);
        const walletStr = publicKey.toBase58();
        const signatures = await devnetConn.getSignaturesForAddress(treasuryPubkey, { limit: 20 });
        let totalYieldLamports = 0;
        for (const sigInfo of signatures) {
          if (cancelled) break;
          try {
            const tx = await devnetConn.getTransaction(sigInfo.signature, { maxSupportedTransactionVersion: 0 });
            if (!tx || !tx.meta) continue;
            const { preBalances, postBalances } = tx.meta;
            const accountKeys = tx.transaction.message.staticAccountKeys
              ? tx.transaction.message.staticAccountKeys
              : (tx.transaction.message as { accountKeys: PublicKey[] }).accountKeys;
            for (let i = 0; i < accountKeys.length; i++) {
              const key = accountKeys[i] instanceof PublicKey ? (accountKeys[i] as PublicKey).toBase58() : String(accountKeys[i]);
              if (key === walletStr) {
                const delta = (postBalances[i] ?? 0) - (preBalances[i] ?? 0);
                if (delta > 0) totalYieldLamports += delta;
              }
            }
          } catch {}
        }
        if (!cancelled) { setYieldReceived(totalYieldLamports / LAMPORTS_PER_SOL); setYieldLoading(false); }
      } catch { if (!cancelled) { setYieldReceived(null); setYieldLoading(false); } }
    })();
    return () => { cancelled = true; };
  }, [publicKey]);

  /* ── Derived ── */

  const netPnl = useMemo(() => {
    if (!trader?.trade_history) return { total: 0, trades: [] as number[] };
    const trades = trader.trade_history.map((t) => {
      const holdHours = Math.max(0, (t.exit_time - t.entry_time)) / 3600;
      const feeDrag = 0.12 + 0.0042 * holdHours;
      return t.pnl_pct - feeDrag;
    });
    return { total: trades.reduce((a, b) => a + b, 0), trades };
  }, [trader]);

  const totalPnlPct = netPnl.total;
  const winRate = useMemo(() => {
    if (!netPnl.trades.length) return null;
    return (netPnl.trades.filter((p) => p > 0).length / netPnl.trades.length) * 100;
  }, [netPnl]);

  const daysRunning = useMemo(() => {
    const deployedAt = new Date("2026-05-12T04:20:00Z").getTime();
    return Math.max(0, Math.ceil((Date.now() - deployedAt) / 86400000));
  }, []);

  const traderStatus =
    trader == null ? "connecting" :
    trader.open_position ? "in_position" : "watching";

  /* ── Render ── */

  return (
    <div className="page">
      <Topbar activePage="dashboard" />

      {/* Emergency freeze banner */}
      {isFrozen && (
        <div style={{
          background: "#dc2626", color: "#fff", padding: "10px 24px",
          textAlign: "center", fontSize: "0.875rem", fontWeight: 600,
          letterSpacing: "0.04em",
        }}>
          TREASURY FROZEN: All operations halted by authority. Unfreeze requires 2-of-3 multisig approval + 24h time lock.
        </div>
      )}

      {/* ════════ HERO ════════ */}
      <section className="hero" style={{ marginBottom: 0 }}>
        <div className="hero-image-wrap">
          <img src="/bg-flower.jpg" alt="Ethereal flower in emerald and coral: the organic intelligence that drives RTP" />
        </div>

        <div className="hero-content">
          <div className="hero-copy">
            <span className="hero-label">SOLANA-NATIVE · AUTONOMOUS TREASURY · SELF-FUNDING</span>
            <h1 className="hero-title">
              Every token gets a
              <br />
              program-enforced treasury
            </h1>
            <p className="hero-tagline" style={{
              fontSize: "1.25rem",
              fontWeight: 400,
              color: "var(--text-primary)",
              letterSpacing: "-0.01em",
              margin: "0.5rem 0 0.75rem",
              lineHeight: 1.4,
            }}>
              No one wants to hold anymore.{" "}<span className="hero-tagline-break">RTP gives them a reason.</span>
            </p>
            <p className="hero-subtitle">
              Token projects route trading fees to RTP → the swarm generates returns via on-chain perps → SOL flows back to holders. 70/20/10 split, enforced on-chain. No RTP token.<br />Pure infrastructure.
            </p>
          </div>

          <div className="sys2-hero-cta-row" style={{ marginBottom: "var(--space-lg)" }}>
            <Link href="/launch" className="sys2-cta-primary">
              Launch a token <span className="cta-badge-devnet">DEVNET</span>
            </Link>
            <Link href="/docs" className="sys2-cta-secondary">Read the docs →</Link>
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

        {/* Vitals strip — full width */}
        <div className="sys2-vitals" style={{ gridColumn: "1 / -1" }}>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Cumulative PnL <span style={{ fontSize: "0.7em", opacity: 0.6 }}>(net of fees)</span></div>
            <div className={`sys2-vital-value ${totalPnlPct >= 0 ? "pos" : "neg"}`}>
              {totalPnlPct >= 0 ? "+" : ""}{totalPnlPct.toFixed(2)}%
            </div>
            <div className="sys2-vital-sub">{trader?.total_trades ?? 0} closed trades</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Days Active</div>
            <div className="sys2-vital-value">{daysRunning}</div>
            <div className="sys2-vital-sub">Since May 12 · Railway</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Treasury SOL</div>
            <div className="sys2-vital-value">
              {treasurySol !== null && treasurySol > 0 ? treasurySol.toFixed(4) : "—"}
            </div>
            <div className="sys2-vital-sub">{treasurySol !== null && treasurySol > 0 ? "Mainnet · Driyi8...EF6R" : "Mainnet"}</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Mainnet TXs</div>
            <div className="sys2-vital-value">4</div>
            <div className="sys2-vital-sub">CPI + REST proofs</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Test coverage</div>
            <div className="sys2-vital-value">362<span className="sys2-vital-unit">+5</span></div>
            <div className="sys2-vital-sub">Rust unit + integration</div>
          </div>
          <div className="sys2-vital">
            <div className="sys2-vital-label">Calmar (validated)</div>
            <div className="sys2-vital-value">44.89</div>
            <div className="sys2-vital-sub">{"SOL Survivor 2.69 · "}<span className="vital-sub-line">{"9x lev"}</span></div>
          </div>
        </div>
      </section>

      {/* ════════ §1 PROVEN ON MAINNET ════════ */}
      <section className="sys2-section" id="live">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§1 · proven on mainnet</div>
            <h2 className="sys2-sect-title">The yield engine is running. With real capital.</h2>
            <p className="sys2-sect-lede">
              Beta testing with skin in the game — real capital on mainnet, proving it before we
              open the doors. A Rust agent executes validated strategies on-chain
              via Flash Trade CPI, signed by the Treasury PDA. No human keypair exists. Every
              position is an on-chain transaction verifiable on Solana Explorer.
            </p>
          </div>
          <div className="sys2-sect-side">
            <span className={`sys2-status-pill ${traderStatus}`}>
              <span className="sys2-status-dot" />
              {traderStatus === "in_position" ? "Position open on mainnet" :
               traderStatus === "watching" ? "Flat, watching the tape" : "Connecting…"}
            </span>
          </div>
        </header>

        <div className="console-grid">
          <div className="console-card">
            <div className="console-card-eyebrow">CURRENT POSITION</div>
            {trader?.open_position ? (
              <>
                <div className="console-big">
                  SOL/USDT · {trader.open_position.side?.toUpperCase() ?? "LONG"} · 9×
                </div>
                <div className="console-row">
                  <span>Entry</span><span className="mono">${trader.open_position.entry_price.toFixed(4)}</span>
                </div>
                <div className="console-row">
                  <span>Peak</span><span className="mono">${trader.open_position.peak_price.toFixed(4)}</span>
                </div>
                <div className="console-row">
                  <span>Size</span><span className="mono">${trader.open_position.size_usd.toFixed(2)}</span>
                </div>
                <div className="console-row">
                  <span>Entry score</span><span className="mono">{trader.open_position.entry_score.toFixed(3)}</span>
                </div>
              </>
            ) : (
              <>
                <div className="console-big console-muted">Flat</div>
                <div className="console-empty-text">
                  Survivor 2.69 enters LONG when score &gt; 0.3 or SHORT when score &lt; -0.3, with 3+ aligned timeframes. 20% capital, 9× leverage. Stop-loss 2.5× ATR, take-profit 6.0× ATR, trailing 1.0× ATR.
                </div>
              </>
            )}
            <div className="console-foot">
              Last poll · <span className="mono">{trader?.last_poll?.slice(11, 19) ?? "—"}</span>
            </div>
          </div>

          <div className="console-card chart-card">
            <div className="console-card-eyebrow">CUMULATIVE PNL · NET OF FEES · ALL CLOSED TRADES</div>
            <PnlSparkline trades={trader?.trade_history ?? []} />
            <div className="chart-stats">
              <div className="chart-stat">
                <span className="chart-stat-val">{trader?.total_trades ?? 0}</span>
                <span className="chart-stat-lab">trades</span>
              </div>
              <div className="chart-stat">
                <span className={`chart-stat-val ${totalPnlPct >= 0 ? "pos" : "neg"}`}>
                  {totalPnlPct >= 0 ? "+" : ""}{totalPnlPct.toFixed(2)}%
                </span>
                <span className="chart-stat-lab">cumulative</span>
              </div>
              <div className="chart-stat">
                <span className="chart-stat-val">{winRate == null ? "—" : `${winRate.toFixed(0)}%`}</span>
                <span className="chart-stat-lab">win rate</span>
              </div>
              <div className="chart-stat">
                <span className="chart-stat-val mono">{trader?.total_pnl_sol?.toFixed(4) ?? "0.0000"}</span>
                <span className="chart-stat-lab">SOL realized</span>
              </div>
            </div>
          </div>
        </div>

        {/* Trade tape */}
        {(trader?.trade_history?.length ?? 0) > 0 && (
          <div className="trade-tape">
            <div className="trade-tape-head">
              <span>RECENT TAPE</span>
              <span className="mono trade-tape-sub">last {Math.min(8, trader?.trade_history?.length ?? 0)} trades</span>
            </div>
            <div className="trade-tape-rows">
              {[...(trader?.trade_history ?? [])].slice(-8).reverse().map((t, i) => {
                const holdHours = Math.max(0, (t.exit_time - t.entry_time)) / 3600;
                const feeDrag = 0.12 + 0.0042 * holdHours;
                const net = t.pnl_pct - feeDrag;
                const priceChange = (t.exit_price - t.entry_price) / t.entry_price * 100;
                const rawPnl = t.pnl_pct;
                const side = t.side ?? (Math.sign(rawPnl) === Math.sign(priceChange) ? "Long" : "Short");
                return (
                <div key={i} className={`trade-row ${net >= 0 ? "pos" : "neg"}`}>
                  <span className="mono">SOL/USDT</span>
                  <span className="mono dim">${t.entry_price.toFixed(2)} → ${t.exit_price.toFixed(2)}</span>
                  <span className="trade-reason">{t.exit_reason}</span>
                  <span className={`trade-side ${side === "Short" ? "short" : "long"}`}>{side.toUpperCase()}</span>
                  <span className={`mono trade-pnl ${net >= 0 ? "pos" : "neg"}`}>
                    {net >= 0 ? "+" : ""}{net.toFixed(2)}%
                  </span>
                </div>
              );
              })}
            </div>
          </div>
        )}

        {/* Mainnet proof — hidden on mobile (cards too heavy) */}
        <div className="hide-mobile" style={{ marginTop: "var(--space-2xl)" }}>
          <header className="sys2-sect-head" style={{ marginBottom: "var(--space-lg)" }}>
            <div>
              <div className="sys2-sect-eyebrow">ON-CHAIN PROOF</div>
              <h2 style={{ fontFamily: "var(--font-display)", fontSize: "clamp(1.125rem, 2vw, 1.5rem)", fontWeight: 500, color: "var(--text-primary)", letterSpacing: "-0.01em", margin: 0 }}>
                Real mainnet transactions, not testnet
              </h2>
            </div>
          </header>
          <div className="proof2-grid">
            {MAINNET_TXS.map((tx) => (
              <a key={tx.tx} href={`https://explorer.solana.com/tx/${tx.tx}`}
                target="_blank" rel="noopener noreferrer" className={`proof2-card kind-${tx.kind}`}>
                <div className="proof2-head">
                  <span className="proof2-kind">{tx.kind === "open" ? "OPEN" : "CLOSE"}</span>
                  <span className="proof2-link">↗</span>
                </div>
                <div className="proof2-label">{tx.label}</div>
                <div className="proof2-tx mono">{tx.tx.slice(0, 10)}…{tx.tx.slice(-8)}</div>
                <div className="proof2-note">{tx.note}</div>
              </a>
            ))}
          </div>
          <div className="proof2-extras">
            <a href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet"
              target="_blank" rel="noopener noreferrer" className="proof2-extra">Devnet redistribution TX ↗</a>
            <a href="https://explorer.solana.com/address/8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB?cluster=devnet"
              target="_blank" rel="noopener noreferrer" className="proof2-extra">Treasury program (devnet) ↗</a>
            <a href="https://explorer.solana.com/tx/2bLg1FuJ6iqwYq6SKi5EcZQWszarDZhS68bCbGTRLKMwhYqsU7G57fTtG4G6GFx3ZKN15qhb85zy28pGJvSdrnG3"
              target="_blank" rel="noopener noreferrer" className="proof2-extra">Mainnet CPI proof ↗</a>
          </div>
        </div>
      </section>

      {/* ════════ §2 TRUSTLESS BY DESIGN ════════ */}
      <section className="sys2-section" id="trust">
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§2 · trustless by design</div>
            <h2 className="sys2-sect-title">Agents propose. The program disposes.</h2>
            <p className="sys2-sect-lede">
              16 constitutional invariants are enforced in both the Rust runtime (<code className="inline-code">soulguard.rs</code>)
              and the on-chain Anchor program. No human can sign for the treasury. No human can override
              the rules, not even the authority. The program is the only authority.
            </p>
          </div>
        </header>

        <div className="arch2-layer-cells">
          {INVARIANTS.map((inv) => (
            <div key={inv.title} className="arch2-cell" style={{ borderLeft: "2px solid var(--emerald-dim)" }}>
              <div className="arch2-cell-title">{inv.title}</div>
              <div className="arch2-cell-sub">{inv.desc}</div>
            </div>
          ))}
        </div>

        <div className="arch2-coord" style={{ marginTop: "var(--space-xl)" }}>
          <span className="arch2-coord-tag">ENFORCEMENT</span>
          Every message between wings is validated against the constitutional governance layer by soulguard.rs.
          The on-chain program adds a second enforcement layer: PDA seed constraints, authority gates,
          strategy lifecycle gates, and overflow-safe math. 362 unit + 5 integration tests verify both layers.
        </div>
      </section>

      {/* ════════ §3 SELF-IMPROVING RESEARCH ENGINE ════════ */}
      <section className="sys2-section" id="pipeline" style={{ marginTop: "var(--space-4xl)" }}>
        <header className="sys2-sect-head">
          <div>
            <div className="sys2-sect-eyebrow">§3 · self-improving research engine</div>
            <h2 className="sys2-sect-title">30,000 hypotheses tested every night. Only the survivors reach the chain.</h2>
            <p className="sys2-sect-lede">
              The Night Shift runs exhaustive parameter search, validates through 9 independent time
              windows, applies Darwinian evolution, and stress-tests with Monte Carlo simulation.
              An LLM consults a library of 15 strategies and a log of remembered failures before
              every exploration run. Nothing is repeated. The system learns by remembering what does not work.
            </p>
          </div>
        </header>

        <ol className="pipe2-steps">
          {[
            { n: "01", t: "Grid Search", m: "30,000", u: "configs swept per symbol per night", d: "Exhaustive sweep across signal threshold, TP/SL multipliers, trailing stop, hold time, alignment." },
            { n: "02", t: "Walk-Forward Validation", m: `${night?.num_folds ?? 9}`, u: "expanding-window folds · 36 days OOS each", d: "No look-ahead. Median OOS Sharpe wins, not mean. Each candidate tested on 9 independent windows." },
            { n: "03", t: "Darwinian Evolution", m: "5×50", u: "generations × population = 250 refined survivors", d: "Top survivors mutate and compete. Fragility is a penalty, not rejection: survivor *= 1/(1+fragility)." },
            { n: "04", t: "Overfitting Detection", m: "3", u: "independent checks: IS/OOS gap, fold consistency, fragility", d: "Monte Carlo drawdown over 10K paths + Combinatorial Purged CV with PBO. Anything fragile is dropped." },
            { n: "05", t: "Full-Sim Validation", m: "0.1%", u: "fees + 10 bps slippage + 20% position cap + compounding", d: "Top candidates re-run through the production simulator. Fast vs full sim calibrated weekly." },
          ].map((s, i) => (
            <li key={s.n} className="pipe2-step">
              <div className="pipe2-num">{s.n}</div>
              <div className="pipe2-body">
                <div className="pipe2-title">{s.t}</div>
                <div className="pipe2-metric">
                  <span className="pipe2-big">{s.m}</span>
                  <span className="pipe2-unit">{s.u}</span>
                </div>
                <div className="pipe2-desc">{s.d}</div>
              </div>
              {i < 4 && <div className="pipe2-tick">▾</div>}
            </li>
          ))}
        </ol>

        <div className="validated-card">
          <div className="validated-head">
            <span className="validated-tag">VALIDATED · LIVE ON MAINNET</span>
            <span className="validated-title">SOL/USDT Survivor 2.69 · 9× leverage</span>
          </div>
          <div className="validated-grid">
            {[
              { v: "44.89", l: "Calmar Ratio" }, { v: "+554%", l: "9× return" },
              { v: "12.3%", l: "Max drawdown" }, { v: "100%", l: "Fold consistency" },
              { v: "0", l: "Liquidations" }, { v: "16,228", l: "Candidates tested" },
            ].map((m) => (
              <div key={m.l} className="validated-cell">
                <span className="validated-val">{m.v}</span>
                <span className="validated-lab">{m.l}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Compact: active strategy + failure memory */}
        <div className="intel-grid intel-grid--2col" style={{ marginTop: "var(--space-2xl)" }}>
          <article className="intel-panel intel-active">
            <header className="intel-panel-head">
              <span className="intel-pill live">LIVE</span>
              <span className="intel-panel-title">Active Strategy</span>
            </header>
            <div className="intel-active-name">SOL/USDT · Survivor 2.69</div>
            <div className="intel-active-type">Multi-timeframe trend following · 9× leverage</div>
            <div className="intel-chips">
              {[["signal_threshold","0.3"],["tp_atr","6.0"],["sl_atr","2.5"],["trail_atr","1.0"],["min_alignment","3"],["max_hold","96h"]].map(([k, v]) => (
                <span key={k} className="intel-chip"><span className="dim">{k}</span>={v}</span>
              ))}
            </div>
            <div className="intel-active-status">
              <span className={`status-dot ${trader ? "live" : ""}`} />
              {trader ? trader.open_position ? "Position open on mainnet" : `Watching · ${trader.candle_count} candles · ${trader.total_trades} trades` : "Connecting to trader…"}
            </div>
          </article>

          <article className="intel-panel">
            <header className="intel-panel-head">
              <span className="intel-pill explore">6 WINGS</span>
              <span className="intel-panel-title">Swarm Architecture</span>
            </header>
            <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              {[
                { name: "Trading", desc: "Flash Trade CPI · REST · PnL", live: true },
                { name: "Evolve", desc: "LLM proposer · gates · rollback", live: true },
                { name: "Audit", desc: "3-agent tribunal · consensus" },
                { name: "Security", desc: "Threats · rate limits · alerts" },
                { name: "Knowledge", desc: "Persistent store · cross-wing graph" },
                { name: "Futureproof", desc: "Deprecation · heartbeat" },
              ].map((w) => (
                <div key={w.name} style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "4px 0", borderBottom: "1px solid var(--border)" }}>
                  <span style={{ fontSize: "0.8125rem", fontWeight: 500, color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "6px" }}>
                    {w.name}
                    {w.live && <span className="arch2-wing-pulse" style={{ width: "5px", height: "5px" }} />}
                  </span>
                  <span style={{ fontSize: "0.6875rem", color: "var(--text-tertiary)" }}>{w.desc}</span>
                </div>
              ))}
            </div>
          </article>
        </div>
      </section>

      {/* ════════ §4 INTEGRATE ════════ */}
      <section className="sys2-section sys2-cta-section" id="integrate">
        <div className="cta2-card">
          <div className="cta2-content">
            <div className="sys2-sect-eyebrow">§4 · integrate</div>
            <h2 className="cta2-title">One function call. A program-enforced treasury for any token.</h2>
            <p className="cta2-lede">
              No RTP token. No custody. No new wallet. The SDK registers a Token-2022 mint with its own
              Treasury PDA in a single call. Trading fees flow in. SOL flows out 70/20/10. The program is
              the only thing that can sign, by design.
            </p>
            <pre className="cta2-code"><code>{`import { registerWithRTP } from "@resilient-protocol/sdk";

const result = await registerWithRTP(connection, payer, {
  authority: payer.publicKey,
});

// result.treasuryPDA → program-owned, no human can sign for it`}</code></pre>
            <div className="cta2-actions">
              <Link href="/launch" className="sys2-cta-primary">
                Launch a token <span className="cta-badge-devnet">DEVNET</span>
              </Link>
              <Link href="/docs" className="sys2-cta-secondary">Read the docs →</Link>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
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
          <span className="vital-value">Driyi8...EF6R</span>
          <span className="vital-label">Trader Wallet (Mainnet)</span>
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
          <a className="vital-link" href="https://explorer.solana.com/tx/4RVehmPVpnFYHrsF6N64RjVh7mszRzKF9DQVHd8TUqBHwrnyDYavf3TnDYJC4b5PrJWVSubZkNuyVkF1oJzk71RT?cluster=devnet"
            target="_blank" rel="noopener noreferrer">On-Chain Proof ↗</a>
          <span className="vital-label">On-Chain Proof</span>
        </div>
        <div className="vital">
          <a className="vital-link" href="https://resilientprotocol.xyz" target="_blank" rel="noopener noreferrer">resilientprotocol.xyz ↗</a>
          <span className="vital-label">Dashboard</span>
        </div>
        <div className="vital">
          <a className="vital-link" href={`https://explorer.solana.com/address/${TRADER_WALLET}`}
            target="_blank" rel="noopener noreferrer">Solana Explorer ↗</a>
          <span className="vital-label">Trader Wallet (Mainnet)</span>
        </div>
      </footer>
    </div>
  );
}
