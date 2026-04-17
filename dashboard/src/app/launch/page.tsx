"use client";

import React, { useState } from "react";
import Link from "next/link";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import {
  createRTPToken,
  fetchTreasuryState,
  RTP_PROGRAM_ID,
  type RTPTokenResult,
  type TreasuryState,
} from "../../lib/sdk";

const PROGRAM_ID_SHORT = RTP_PROGRAM_ID.toBase58();
const CLUSTER = "devnet";

// ── Platform types ──────────────────────────────────────────

type Platform = "rtp" | "metaplex" | "pumpfun" | "bags";

interface PlatformDef {
  id: Platform;
  name: string;
  color: string;
  desc: string;
  token: string;
}

const PLATFORMS: PlatformDef[] = [
  {
    id: "rtp",
    name: "RTP Direct",
    color: "var(--coral)",
    desc: "Token-2022 with TransferFeeConfig",
    token: "Token-2022",
  },
  {
    id: "metaplex",
    name: "Metaplex",
    color: "#4169E1",
    desc: "Fair launch via Genesis Launch Pool",
    token: "SPL (standard)",
  },
  {
    id: "pumpfun",
    name: "Pump.fun",
    color: "#00d18c",
    desc: "Bonding curve memecoin launch",
    token: "SPL (bonding curve)",
  },
  {
    id: "bags",
    name: "Bags.fm",
    color: "#7C3AED",
    desc: "Fee sharing on Meteora DLMM",
    token: "SPL (Meteora DLMM)",
  },
];

// ── Code generators ─────────────────────────────────────────

function generateRTPSnippet(f: { projectName: string; tokenSymbol: string; totalSupply: string; feeBps: string }): string {
  const name = f.projectName || "My Token";
  const symbol = f.tokenSymbol || "TKN";
  const supply = f.totalSupply || "1_000_000_000";
  const feeBps = f.feeBps || "200";
  return `import { createRTPToken } from "@resilient-protocol/sdk";
import { Connection, Keypair } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // your launchpad keypair

const result = await createRTPToken(connection, payer, {
  name: "${name}",
  symbol: "${symbol}",
  supply: ${supply},
  feeBps: ${feeBps},  // ${(parseInt(feeBps) / 100).toFixed(0)}% transfer fee
});

console.log("Mint:", result.mint);
console.log("Treasury PDA:", result.treasuryPDA);
console.log("Vault PDA:", result.vaultPDA);
// Fee destination: per-mint vault PDA (program-owned, immutable)`;
}

function generateMetaplexSnippet(f: { projectName: string; tokenSymbol: string; metaSupply: string; launchType: string; pricePerToken: string }): string {
  const name = f.projectName || "My Token";
  const symbol = f.tokenSymbol || "TKN";
  const supply = f.metaSupply || "1_000_000_000";
  const lt = f.launchType || "launchpool";
  const raiseGoal = f.pricePerToken || "250";
  return `import {
  createAndRegisterLaunch,
  type CreateLaunchInput,
  genesis,
} from "@metaplex-foundation/genesis";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { keypairIdentity } from "@metaplex-foundation/umi";
import { createRTPToken } from "@resilient-protocol/sdk";
import { Connection, Keypair } from "@solana/web3.js";

// Genesis SDK uses Umi (not the old Metaplex JS SDK)
const umi = createUmi("https://api.mainnet-beta.solana.com").use(genesis());
const payer = Keypair.generate();
umi.use(keypairIdentity(payer));

const input: CreateLaunchInput = {
  wallet: umi.identity.publicKey,
  token: {
    name: "${name}",
    symbol: "${symbol}",
    image: "https://gateway.irys.xyz/YOUR_IMAGE_CID",
    description: "Launched via RTP + Metaplex Genesis",
  },
  launchType: "${lt}",
  launch: {
    launchpool: {
      tokenAllocation: ${supply},       // tokens to sell
      depositStartTime: new Date(Date.now() + 48 * 60 * 60 * 1000),
      raiseGoal: ${raiseGoal},            // SOL to raise (min 250)
      raydiumLiquidityBps: 5000,          // 50% to Raydium LP
      fundsRecipient: umi.identity.publicKey,
    },
  },
};

// One call: creates accounts, signs, sends, registers
const result = await createAndRegisterLaunch(umi, {}, input);
console.log("Launch live at:", result.launch.link);
console.log("Mint:", result.mintAddress);

// Initialize RTP treasury for the new mint
const connection = new Connection("https://api.mainnet-beta.solana.com");
const rtpResult = await createRTPToken(connection, payer, {
  name: "${name}",
  symbol: "${symbol}",
  supply: ${supply},
  feeBps: 200,  // 2% transfer fee to treasury
});
console.log("RTP Treasury PDA:", rtpResult.treasuryPDA);`;
}

function generatePumpSnippet(f: { projectName: string; tokenSymbol: string; description: string; imageUrl: string; website: string; twitter: string; telegram: string; devBuyAmount: string }): string {
  const name = f.projectName || "My Token";
  const symbol = f.tokenSymbol || "TKN";
  const buyAmt = f.devBuyAmount || "0.1";
  return `import {
  Connection,
  Keypair,
  VersionedTransaction,
} from "@solana/web3.js";
import { createRTPToken } from "@resilient-protocol/sdk";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const mintKeypair = Keypair.generate();
const payer = Keypair.generate(); // your wallet keypair

// 1. Upload metadata to IPFS (e.g. Pinata)
//    POST image to https://uploads.pinata.cloud/v3/files
//    Then POST JSON metadata { name, symbol, image, description, ... }
//    to get a metadata URI (https://ipfs.io/ipfs/CID)

// 2. Get the create transaction from PumpPortal
const response = await fetch("https://pumpportal.fun/api/trade-local", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    publicKey: payer.publicKey.toBase58(),
    action: "create",
    tokenMetadata: {
      name: "${name}",
      symbol: "${symbol}",
      uri: "https://ipfs.io/ipfs/YOUR_METADATA_CID",
    },
    mint: mintKeypair.publicKey.toBase58(),
    denominatedInSol: "true",
    amount: ${buyAmt},       // dev buy in SOL
    slippage: 10,
    priorityFee: 0.00005,
    pool: "pump",
  }),
});

if (!response.ok) throw new Error("PumpPortal create failed: " + response.statusText);
const txData = await response.arrayBuffer();

// 3. Sign with mint keypair + payer, then send
const tx = VersionedTransaction.deserialize(new Uint8Array(txData));
tx.sign([mintKeypair, payer]);
const sig = await connection.sendTransaction(tx);
console.log("Pump.fun tx:", sig);

// 4. Initialize RTP treasury (separate step — Pump.fun uses standard SPL)
const rtpResult = await createRTPToken(connection, payer, {
  name: "${name}",
  symbol: "${symbol}",
  supply: 1_000_000_000,
  feeBps: 200,
});
console.log("RTP Treasury PDA:", rtpResult.treasuryPDA);`;
}

function generateBagsSnippet(f: { projectName: string; tokenSymbol: string; description: string; imageUrl: string; website: string; twitter: string; telegram: string; bagsBuyAmount: string; feeClaimers: string }): string {
  const name = f.projectName || "My Token";
  const symbol = f.tokenSymbol || "TKN";
  const desc = f.description || "A new token on Bags.fm";
  const img = f.imageUrl || "https://example.com/token.png";
  const buyAmt = f.bagsBuyAmount || "0.1";
  const claimers = f.feeClaimers || "";
  const socialParams = [
    f.twitter ? `    twitter: "${f.twitter}",` : null,
    f.website ? `    website: "${f.website}",` : null,
    f.telegram ? `    telegram: "${f.telegram}",` : null,
  ].filter(Boolean).join("\n");
  return `import {
  BagsSDK,
  signAndSendTransaction,
} from "@bagsfm/bags-sdk";
import { Connection, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import bs58 from "bs58";
import { createRTPToken } from "@resilient-protocol/sdk";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const keypair = Keypair.fromSecretKey(bs58.decode("YOUR_BASE58_KEY"));
const sdk = new BagsSDK("YOUR_BAGS_API_KEY", connection, "processed");

// Step 1: Create token info + metadata (IPFS handled by Bags API)
const tokenInfo = await sdk.tokenLaunch.createTokenInfoAndMetadata({
  imageUrl: "${img}",
  name: "${name}",
  symbol: "${symbol}".toUpperCase().replace("$", ""),
  description: "${desc}",
${socialParams}
});
console.log("Token mint:", tokenInfo.tokenMint);

// Step 2: Create fee share config
${claimers ? `// Set RTP treasury PDA as a fee claimer — fees route to treasury automatically
const feeClaimers = [{ user: keypair.publicKey, userBps: 7000 }, { user: new PublicKey("${claimers}"), userBps: 3000 }];
` : `// All fees to creator — set fee claimer to RTP treasury PDA if desired
const feeClaimers = [{ user: keypair.publicKey, userBps: 10000 }];
`}const configResult = await sdk.config.createBagsFeeShareConfig({
  payer: keypair.publicKey,
  baseMint: new PublicKey(tokenInfo.tokenMint),
  feeClaimers,
});
for (const tx of configResult.transactions || []) {
  await signAndSendTransaction(connection, "processed", tx, keypair);
}
console.log("Config key:", configResult.meteoraConfigKey.toString());

// Step 3: Get token launch transaction
const launchTx = await sdk.tokenLaunch.createLaunchTransaction({
  metadataUrl: tokenInfo.tokenMetadata,
  tokenMint: new PublicKey(tokenInfo.tokenMint),
  launchWallet: keypair.publicKey,
  initialBuyLamports: ${buyAmt} * LAMPORTS_PER_SOL,
  configKey: configResult.meteoraConfigKey,
});
const sig = await signAndSendTransaction(connection, "processed", launchTx, keypair);
console.log("Launch sig:", sig);

// Step 4: Initialize RTP treasury
const rtpResult = await createRTPToken(connection, keypair, {
  name: "${name}",
  symbol: "${symbol}",
  supply: 1_000_000_000,
  feeBps: 200,
});
console.log("RTP Treasury PDA:", rtpResult.treasuryPDA);
console.log("View: https://bags.fm/" + tokenInfo.tokenMint);`;
}

// ── Copy button ─────────────────────────────────────────────

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  return (
    <button
      onClick={() => {
        navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }}
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

// ── Page ────────────────────────────────────────────────────

type LaunchPhase = "form" | "confirming" | "launching" | "success" | "error";

export default function LaunchPage() {
  const { publicKey, connected, signTransaction } = useWallet();
  const { connection: rpcConnection } = useConnection();
  const { setVisible } = useWalletModal();

  const [platform, setPlatform] = useState<Platform>("rtp");
  const [phase, setPhase] = useState<LaunchPhase>("form");
  const [result, setResult] = useState<RTPTokenResult | null>(null);
  const [treasuryState, setTreasuryState] = useState<TreasuryState | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Shared fields
  const [projectName, setProjectName] = useState("");
  const [tokenSymbol, setTokenSymbol] = useState("");

  // RTP-specific
  const [totalSupply, setTotalSupply] = useState("1000000000");
  const [feeBps, setFeeBps] = useState("200");

  // Metaplex-specific
  const [metaSupply, setMetaSupply] = useState("500000000");
  const launchType = "launchpool" as const;
  const [raiseGoal, setRaiseGoal] = useState("250");

  // Pump.fun / Bags shared
  const [description, setDescription] = useState("");
  const [imageUrl, setImageUrl] = useState("");
  const [website, setWebsite] = useState("");
  const [twitter, setTwitter] = useState("");
  const [telegram, setTelegram] = useState("");

  // Pump.fun-specific
  const [devBuyAmount, setDevBuyAmount] = useState("0.1");

  // Bags-specific
  const [bagsBuyAmount, setBagsBuyAmount] = useState("0.1");
  const [feeClaimers, setFeeClaimers] = useState("");

  const addr = publicKey
    ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}`
    : null;

  const getSnippet = (): string => {
    const shared = { projectName, tokenSymbol };
    switch (platform) {
      case "rtp":
        return generateRTPSnippet({ ...shared, totalSupply, feeBps });
      case "metaplex":
        return generateMetaplexSnippet({ ...shared, metaSupply, launchType, pricePerToken: raiseGoal });
      case "pumpfun":
        return generatePumpSnippet({ ...shared, description, imageUrl, website, twitter, telegram, devBuyAmount });
      case "bags":
        return generateBagsSnippet({ ...shared, description, imageUrl, website, twitter, telegram, bagsBuyAmount, feeClaimers });
    }
  };

  const handleLaunch = async () => {
    if (!publicKey || !signTransaction) return;
    setPhase("launching");
    setError(null);

    try {
      const launchResult = await createRTPToken(rpcConnection, { publicKey, signTransaction }, {
        name: projectName || "My Token",
        symbol: tokenSymbol || "TKN",
        supply: parseInt(totalSupply) || 1_000_000_000,
        feeBps: parseInt(feeBps) || 200,
      });

      setResult(launchResult);
      setPhase("success");

      setTimeout(async () => {
        try {
          const state = await fetchTreasuryState(rpcConnection, launchResult.mint);
          setTreasuryState(state);
        } catch {
          // best-effort
        }
      }, 3000);
    } catch (err: any) {
      setError(err?.message || String(err));
      setPhase("error");
    }
  };

  const platformNote = (p: Platform): string => {
    switch (p) {
      case "rtp":
        return "Creates Token-2022 mint with TransferFeeConfig on devnet via Phantom wallet. Executed in-browser.";
      case "metaplex":
        return "Generates a Metaplex Genesis SDK script. Run from your backend or CLI. Mainnet.";
      case "pumpfun":
        return "Generates a PumpPortal local transaction script. Run from your backend. Standard SPL — RTP treasury initialized separately.";
      case "bags":
        return "Generates a Bags.fm SDK script with fee sharing. Run from your backend. Fee claimers can route to RTP treasury PDA.";
    }
  };

  const snippet = getSnippet();

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
          <Link href="/launch" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px", borderColor: "var(--coral-dim)", color: "var(--coral)" }}>
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
          Choose your launch platform. The form generates a working code snippet
          for your backend. RTP Direct executes in-browser on devnet.
        </p>
      </section>

      {/* ── Platform selector ── */}
      <section style={{ maxWidth: 720, margin: "0 auto var(--space-2xl)", padding: "0 var(--space-lg)" }}>
        <div style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: "var(--space-md)",
        }}>
          {PLATFORMS.map((p) => (
            <button
              key={p.id}
              onClick={() => {
                setPlatform(p.id);
                setPhase("form");
                setError(null);
              }}
              style={{
                background: platform === p.id
                  ? "rgba(255, 255, 255, 0.06)"
                  : "rgba(255, 255, 255, 0.02)",
                border: `1px solid ${platform === p.id ? p.color : "var(--border)"}`,
                borderRadius: 8,
                padding: "var(--space-md)",
                cursor: "pointer",
                textAlign: "left" as const,
                transition: "border-color 0.15s, background 0.15s",
                color: "inherit",
                fontFamily: "var(--font-body)",
              }}
            >
              <div style={{
                fontSize: "0.875rem",
                fontWeight: 500,
                color: p.color,
                marginBottom: 6,
              }}>
                {p.name}
              </div>
              <div style={{
                fontSize: "0.6875rem",
                color: "var(--text-tertiary)",
                lineHeight: 1.45,
              }}>
                {p.desc}
              </div>
              <div style={{
                fontSize: "0.625rem",
                color: "var(--text-muted)",
                marginTop: 8,
                fontFamily: "var(--font-mono)",
                letterSpacing: "0.03em",
              }}>
                {p.token}
              </div>
            </button>
          ))}
        </div>
      </section>

      {/* ── Wallet connect prompt (RTP Direct only) ── */}
      {platform === "rtp" && !connected && (phase === "form" || phase === "error") && (
        <section className="launch-form-section" style={{ textAlign: "center", padding: "48px 24px" }}>
          <p style={{ color: "var(--text-secondary)", marginBottom: "24px", fontSize: "1.1rem" }}>
            Connect your Phantom wallet to launch a token on devnet.
          </p>
          <button className="btn-launch" onClick={() => setVisible(true)}>
            Connect Phantom Wallet
          </button>
        </section>
      )}

      {/* ── Form section ── */}
      {(platform !== "rtp" || connected) && (phase === "form" || phase === "error") && (
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
              if (platform === "rtp") {
                setPhase("confirming");
              }
            }}
          >
            {/* ── Shared fields ── */}
            <div className="form-group">
              <label className="form-label" htmlFor="projectName">Project Name</label>
              <input
                id="projectName"
                className="form-input"
                type="text"
                placeholder="e.g. My Launchpad Token"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                required
              />
            </div>

            <div className="form-group">
              <label className="form-label" htmlFor="tokenSymbol">Token Symbol</label>
              <input
                id="tokenSymbol"
                className="form-input"
                type="text"
                placeholder="e.g. MLT"
                maxLength={8}
                value={tokenSymbol}
                onChange={(e) => setTokenSymbol(e.target.value.toUpperCase())}
                required
              />
            </div>

            {/* ── RTP Direct fields ── */}
            {platform === "rtp" && (
              <>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="totalSupply">Total Supply (tokens)</label>
                    <input
                      id="totalSupply"
                      className="form-input"
                      type="number"
                      min="1"
                      value={totalSupply}
                      onChange={(e) => setTotalSupply(e.target.value)}
                      required
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="feeBps">
                      Transfer Fee (bps)
                      <span className="form-hint">{(parseInt(feeBps || "0") / 100).toFixed(1)}%</span>
                    </label>
                    <input
                      id="feeBps"
                      className="form-input"
                      type="number"
                      min="0"
                      max="500"
                      step="10"
                      value={feeBps}
                      onChange={(e) => setFeeBps(e.target.value)}
                      required
                    />
                  </div>
                </div>
                <div className="form-note">
                  The transfer fee destination is a per-mint vault PDA derived from the program ID
                  (<code>{PROGRAM_ID_SHORT.slice(0, 8)}...{PROGRAM_ID_SHORT.slice(-4)}</code>).
                  Each token gets its own treasury — no shared vault, no single point of failure.
                </div>
              </>
            )}

            {/* ── Metaplex fields ── */}
            {platform === "metaplex" && (
              <>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="metaSupply">Token Allocation (tokens to sell)</label>
                    <input
                      id="metaSupply"
                      className="form-input"
                      type="number"
                      min="1"
                      value={metaSupply}
                      onChange={(e) => setMetaSupply(e.target.value)}
                      required
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="launchType">Launch Type</label>
                    <input
                      id="launchType"
                      className="form-input"
                      type="text"
                      value="Launchpool (fair launch)"
                      disabled
                      style={{ opacity: 0.6, cursor: "not-allowed" }}
                    />
                  </div>
                </div>
                <div className="form-group">
                  <label className="form-label" htmlFor="raiseGoal">Raise Goal (SOL)
                    <span className="form-hint">min 250 SOL</span>
                  </label>
                  <input
                    id="raiseGoal"
                    className="form-input"
                    type="number"
                    min="250"
                    step="1"
                    value={raiseGoal}
                    onChange={(e) => setRaiseGoal(e.target.value)}
                    required
                  />
                </div>
                <div className="form-note">
                  Uses <code>@metaplex-foundation/genesis</code> SDK with Umi. Supply is fixed at 1B tokens —
                  <code>tokenAllocation</code> is how many you sell. After Genesis creates the mint, the generated
                  script calls <code>createRTPToken()</code> to initialize a treasury. Metaplex is a Colosseum sponsor.
                </div>
              </>
            )}

            {/* ── Pump.fun fields ── */}
            {platform === "pumpfun" && (
              <>
                <div className="form-group">
                  <label className="form-label" htmlFor="pfDesc">Description</label>
                  <input
                    id="pfDesc"
                    className="form-input"
                    type="text"
                    placeholder="Token description shown on Pump.fun"
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                  />
                </div>
                <div className="form-group">
                  <label className="form-label" htmlFor="pfImg">Image URL</label>
                  <input
                    id="pfImg"
                    className="form-input"
                    type="url"
                    placeholder="https://example.com/token.png"
                    value={imageUrl}
                    onChange={(e) => setImageUrl(e.target.value)}
                  />
                </div>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfWeb">Website</label>
                    <input
                      id="pfWeb"
                      className="form-input"
                      type="url"
                      placeholder="https://mytoken.com"
                      value={website}
                      onChange={(e) => setWebsite(e.target.value)}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfTw">Twitter</label>
                    <input
                      id="pfTw"
                      className="form-input"
                      type="text"
                      placeholder="@mytoken"
                      value={twitter}
                      onChange={(e) => setTwitter(e.target.value)}
                    />
                  </div>
                </div>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfTg">Telegram</label>
                    <input
                      id="pfTg"
                      className="form-input"
                      type="text"
                      placeholder="t.me/mytoken"
                      value={telegram}
                      onChange={(e) => setTelegram(e.target.value)}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfBuy">Dev Buy Amount (SOL)</label>
                    <input
                      id="pfBuy"
                      className="form-input"
                      type="number"
                      min="0"
                      step="0.01"
                      value={devBuyAmount}
                      onChange={(e) => setDevBuyAmount(e.target.value)}
                    />
                  </div>
                </div>
                <div className="form-note">
                  PumpPortal returns a local <code>VersionedTransaction</code> — you sign and send it.
                  Metadata must be uploaded to IPFS first (e.g. <a href="https://pinata.cloud" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>Pinata</a>).
                  Token-2022 with TransferFeeConfig is <strong>not compatible</strong> with Pump.fun&apos;s bonding curve.
                  The generated script creates the SPL token via PumpPortal, then initializes RTP treasury separately.
                </div>
              </>
            )}

            {/* ── Bags.fm fields ── */}
            {platform === "bags" && (
              <>
                <div className="form-group">
                  <label className="form-label" htmlFor="bagsDesc">Description</label>
                  <input
                    id="bagsDesc"
                    className="form-input"
                    type="text"
                    placeholder="Token description"
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                  />
                </div>
                <div className="form-group">
                  <label className="form-label" htmlFor="bagsImg">Image URL</label>
                  <input
                    id="bagsImg"
                    className="form-input"
                    type="url"
                    placeholder="https://example.com/token.png"
                    value={imageUrl}
                    onChange={(e) => setImageUrl(e.target.value)}
                  />
                </div>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsWeb">Website</label>
                    <input
                      id="bagsWeb"
                      className="form-input"
                      type="url"
                      placeholder="https://mytoken.com"
                      value={website}
                      onChange={(e) => setWebsite(e.target.value)}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsTw">Twitter</label>
                    <input
                      id="bagsTw"
                      className="form-input"
                      type="text"
                      placeholder="@mytoken"
                      value={twitter}
                      onChange={(e) => setTwitter(e.target.value)}
                    />
                  </div>
                </div>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsBuy">Initial Buy Amount (SOL)</label>
                    <input
                      id="bagsBuy"
                      className="form-input"
                      type="number"
                      min="0"
                      step="0.01"
                      value={bagsBuyAmount}
                      onChange={(e) => setBagsBuyAmount(e.target.value)}
                    />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsClaimers">Fee Claimers (optional)</label>
                    <input
                      id="bagsClaimers"
                      className="form-input"
                      type="text"
                      placeholder="Treasury PDA address"
                      value={feeClaimers}
                      onChange={(e) => setFeeClaimers(e.target.value)}
                    />
                  </div>
                </div>
                <div className="form-note">
                  Bags.fm&apos;s fee sharing model aligns perfectly with RTP. Set the fee claimer to your
                  RTP treasury PDA — creator fees route to the treasury automatically. The generated script
                  uses the <code>@bagsfm/bags-sdk</code> and requires an API key from{" "}
                  <a href="https://dev.bags.fm/" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>dev.bags.fm</a>.
                </div>
              </>
            )}

            {/* ── Submit button (RTP Direct only) ── */}
            {platform === "rtp" && (
              <button type="submit" className="btn-launch">
                Launch Token on Devnet
              </button>
            )}
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

          {/* ── Code output ── */}
          <div style={{ marginTop: "32px", paddingTop: "24px", borderTop: "1px solid var(--border)" }}>
            <h3 style={{ fontSize: "0.9375rem", color: "var(--text-secondary)", marginBottom: "12px" }}>
              {platform === "rtp" ? "Generated SDK call:" : "Generated script:"}
            </h3>
            <div className="code-block" style={{ position: "relative" }}>
              <CopyButton text={snippet} />
              <pre><code>{snippet}</code></pre>
            </div>
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
              {platformNote(platform)}
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

      {/* ── Confirmation dialog (RTP Direct only) ── */}
      {phase === "confirming" && platform === "rtp" && (
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
            <div style={{ marginBottom: "8px" }}><strong>Name:</strong> {projectName}</div>
            <div style={{ marginBottom: "8px" }}><strong>Symbol:</strong> {tokenSymbol}</div>
            <div style={{ marginBottom: "8px" }}><strong>Supply:</strong> {parseInt(totalSupply).toLocaleString()}</div>
            <div style={{ marginBottom: "8px" }}><strong>Fee:</strong> {(parseInt(feeBps) / 100).toFixed(1)}%</div>
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
