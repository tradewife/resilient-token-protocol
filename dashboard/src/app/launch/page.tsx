"use client";

import React, { useState, useCallback } from "react";
import Link from "next/link";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import {
  Connection,
  VersionedTransaction,
  Transaction,
  Keypair,
  PublicKey,
} from "@solana/web3.js";
import {
  createRTPToken,
  fetchTreasuryState,
  registerAdopterBeta,
  fetchAdopterState,
  RTP_PROGRAM_ID,
  type RTPTokenResult,
  type TreasuryState,
  type AdopterState,
} from "../../lib/sdk";

const PROGRAM_ID_SHORT = RTP_PROGRAM_ID.toBase58();
const CLUSTER = "devnet";

/** Beta expiry: May 18 2026 23:59 UTC — one week after Colosseum hackathon deadline. */
const BETA_EXPIRES_AT = Math.floor(new Date("2026-05-18T23:59:59Z").getTime() / 1000);

// Platform types

type Platform = "rtp" | "metaplex" | "pumpfun" | "bags";

interface PlatformDef {
  id: Platform;
  name: string;
  color: string;
  desc: string;
  token: string;
}

const PLATFORMS: PlatformDef[] = [
  { id: "rtp", name: "RTP Direct", color: "var(--coral)", desc: "Token-2022 with TransferFeeConfig", token: "Token-2022" },
  { id: "metaplex", name: "Metaplex", color: "#4169E1", desc: "Fair launch via Genesis Launch Pool", token: "SPL (standard)" },
  { id: "pumpfun", name: "Pump.fun", color: "#00d18c", desc: "Bonding curve memecoin launch", token: "SPL (bonding curve)" },
  { id: "bags", name: "Bags.fm", color: "#B8A9E8", desc: "Fee sharing on Meteora DLMM", token: "SPL (Meteora DLMM)" },
];

// Types

type LaunchPhase = "form" | "launching" | "rtp_init" | "success" | "error";

interface LaunchResult {
  mint: string;
  signature: string;
  explorerUrl: string;
  platform: Platform;
  treasuryPDA?: string;
  vaultPDA?: string;
}

// Helpers

async function sendAndConfirm(
  connection: Connection,
  signedTx: VersionedTransaction | Transaction,
): Promise<string> {
  const raw = signedTx.serialize();
  const sig = await connection.sendRawTransaction(raw, { skipPreflight: false, maxRetries: 3 });
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  await connection.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, "confirmed");
  return sig;
}

function getStored(key: string): string {
  if (typeof window === "undefined") return "";
  return localStorage.getItem(key) || "";
}

function setStored(key: string, val: string) {
  if (typeof window !== "undefined") localStorage.setItem(key, val);
}

// Upload metadata JSON to Pinata (optional)

async function uploadMetadataToPinata(jwt: string, metadata: object): Promise<string | null> {
  try {
    const res = await fetch("https://api.pinata.cloud/pinning/pinJSONToIPFS", {
      method: "POST",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${jwt}` },
      body: JSON.stringify({ pinataContent: metadata }),
    });
    if (!res.ok) return null;
    const data = await res.json();
    return `https://ipfs.io/ipfs/${data.IpfsHash}`;
  } catch (e: unknown) {
    // Pinata IPFS upload failed — metadata will be missing from token URI
    console.warn("[Launch] Pinata IPFS upload failed:", e instanceof Error ? e.message : String(e));
    return null;
  }
}

async function uploadImageToPinata(file: File): Promise<string | null> {
  const jwt = getStored("rtp_pinata_jwt") || PINATA_JWT_FALLBACK;
  if (!jwt) return null;
  try {
    const formData = new FormData();
    formData.append("file", file);
    const res = await fetch("https://api.pinata.cloud/pinning/pinFileToIPFS", {
      method: "POST",
      headers: { Authorization: `Bearer ${jwt}` },
      body: formData,
    });
    if (!res.ok) return null;
    const data = await res.json();
    return `https://ipfs.io/ipfs/${data.IpfsHash}`;
  } catch (e: unknown) {
    console.warn("[Launch] Pinata image upload failed:", e instanceof Error ? e.message : String(e));
    return null;
  }
}

const PINATA_JWT_FALLBACK = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VySW5mb3JtYXRpb24iOnsiaWQiOiJjZmU1NWZhNi0yYzcwLTQ0NzMtOGYwMS01MTM5ODAxNWVhMWMiLCJlbWFpbCI6ImthdGVqY29vcGVyLmF0ZWxpZXJAZ21haWwuY29tIiwiZW1haWxfdmVyaWZpZWQiOnRydWUsInBpbl9wb2xpY3kiOnsicmVnaW9ucyI6W3siZGVzaXJlZFJlcGxpY2F0aW9uQ291bnQiOjEsImlkIjoiRlJBMSJ9LHsiZGVzaXJlZFJlcGxpY2F0aW9uQ291bnQiOjEsImlkIjoiTllDMSJ9XSwidmVyc2lvbiI6MX0sIm1mYV9lbmFibGVkIjpmYWxzZSwic3RhdHVzIjoiQUNUSVZFIn0sImF1dGhlbnRpY2F0aW9uVHlwZSI6InNjb3BlZEtleSIsInNjb3BlZEtleUtleSI6IjcwZGQxZThiYzBhYzljMTg1ZGE5Iiwic2NvcGVkS2V5U2VjcmV0IjoiZTg2MmU1MmMyMTgyMzI3MTZkZTgwYjkyMjI2ZmQ4YjVkOTIxOTAxZWQ2ZDM4MTk5YjVkYmNhZTMyNWYxYzc3NyIsImV4cCI6MTgwODAwMzY0Mn0.-hv7_nN6PvDh_YmK6M7j7AnFvblYg0ZhuXQT4C-h7VY";

// Pump.fun launch

async function launchPumpFun({
  connection, wallet, name, symbol, imageUrl, description, website, twitter, telegram, devBuyAmount,
}: {
  connection: Connection;
  wallet: { publicKey: PublicKey; signTransaction: <T extends VersionedTransaction | Transaction>(tx: T) => Promise<T> };
  name: string; symbol: string; imageUrl: string; description: string;
  website: string; twitter: string; telegram: string; devBuyAmount: number;
}): Promise<LaunchResult> {
  const mintKeypair = Keypair.generate();

  // Build metadata
  const metadata: Record<string, any> = { name, symbol, description: description || "", image: imageUrl || "", showName: true };
  const extensions: Record<string, string> = {};
  if (website) extensions.website = website;
  if (twitter) extensions.twitter = twitter;
  if (telegram) extensions.telegram = telegram;
  if (Object.keys(extensions).length > 0) metadata.extensions = extensions;

  // Try Pinata IPFS upload
  let metadataUri = imageUrl || "";
  const pinataJwt = getStored("rtp_pinata_jwt");
  if (pinataJwt) {
    const uri = await uploadMetadataToPinata(pinataJwt, metadata);
    if (uri) metadataUri = uri;
  }

  // Get create transaction from PumpPortal
  const response = await fetch("https://pumpportal.fun/api/trade-local", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      publicKey: wallet.publicKey.toBase58(),
      action: "create",
      tokenMetadata: { name, symbol, uri: metadataUri },
      mint: mintKeypair.publicKey.toBase58(),
      denominatedInSol: "true",
      amount: devBuyAmount,
      slippage: 10,
      priorityFee: 0.00005,
      pool: "pump",
    }),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`PumpPortal error (${response.status}): ${text}`);
  }

  const txData = await response.arrayBuffer();
  const tx = VersionedTransaction.deserialize(new Uint8Array(txData));
  tx.sign([mintKeypair]);
  const signedTx = await wallet.signTransaction(tx);
  const sig = await sendAndConfirm(connection, signedTx);

  return {
    mint: mintKeypair.publicKey.toBase58(),
    signature: sig,
    explorerUrl: `https://explorer.solana.com/tx/${sig}?cluster=mainnet-beta`,
    platform: "pumpfun",
  };
}

// Metaplex Genesis launch (REST)

async function launchMetaplex({
  connection, wallet, name, symbol, imageUrl, description, supply, raiseGoal,
}: {
  connection: Connection;
  wallet: { publicKey: PublicKey; signTransaction: <T extends VersionedTransaction | Transaction>(tx: T) => Promise<T> };
  name: string; symbol: string; imageUrl: string; description: string;
  supply: number; raiseGoal: number;
}): Promise<LaunchResult> {
  const createRes = await fetch("https://api.metaplex.com/v1/launches/create", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      wallet: wallet.publicKey.toBase58(),
      token: { name, symbol, image: imageUrl || "", description: description || "Launched via RTP + Metaplex Genesis" },
      launchType: "launchpool",
      launch: {
        launchpool: {
          tokenAllocation: supply,
          depositStartTime: new Date(Date.now() + 48 * 60 * 60 * 1000).toISOString(),
          raiseGoal,
          raydiumLiquidityBps: 5000,
          fundsRecipient: wallet.publicKey.toBase58(),
        },
      },
    }),
  });

  if (!createRes.ok) {
    const text = await createRes.text();
    throw new Error(`Metaplex API error (${createRes.status}): ${text}`);
  }

  const createData = await createRes.json();
  let lastSig = "";
  const mintAddress = createData.mintAddress || "";

  for (const txBase64 of createData.transactions || []) {
    const tx = Transaction.from(Buffer.from(txBase64, "base64"));
    const signed = await wallet.signTransaction(tx);
    lastSig = await sendAndConfirm(connection, signed);
  }

  // Register launch (best-effort — external API may be unreachable)
  try {
    await fetch("https://api.metaplex.com/v1/launches/register", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ launchId: createData.launchId, wallet: wallet.publicKey.toBase58() }),
    });
  } catch (e: unknown) {
    // External Metaplex registry unreachable — non-critical, token already created
    console.warn("[Launch] Metaplex registry failed:", e instanceof Error ? e.message : String(e));
  }

  return {
    mint: mintAddress,
    signature: lastSig,
    explorerUrl: `https://explorer.solana.com/tx/${lastSig}?cluster=mainnet-beta`,
    platform: "metaplex",
  };
}

// Bags.fm launch (REST + API key)

async function launchBags({
  connection, wallet, apiKey, name, symbol, imageUrl, description,
  website, twitter, telegram, buyAmount, feeClaimers,
}: {
  connection: Connection;
  wallet: { publicKey: PublicKey; signTransaction: <T extends VersionedTransaction | Transaction>(tx: T) => Promise<T> };
  apiKey: string; name: string; symbol: string; imageUrl: string; description: string;
  website: string; twitter: string; telegram: string; buyAmount: number; feeClaimers: string;
}): Promise<LaunchResult> {
  const headers: Record<string, string> = { "Content-Type": "application/json", "x-api-key": apiKey };

  // Step 1: Create token info
  const tokenRes = await fetch("https://api.bags.fm/v1/token/create-info", {
    method: "POST",
    headers,
    body: JSON.stringify({ imageUrl, name, symbol: symbol.toUpperCase().replace("$", ""), description, website, twitter, telegram }),
  });
  if (!tokenRes.ok) throw new Error(`Bags.fm create-info error (${tokenRes.status}): ${await tokenRes.text()}`);
  const tokenData = await tokenRes.json();
  const mintAddress = tokenData.tokenMint;

  // Step 2: Create fee share config
  const claimers = feeClaimers
    ? [{ user: wallet.publicKey.toBase58(), userBps: 7000 }, { user: feeClaimers, userBps: 3000 }]
    : [{ user: wallet.publicKey.toBase58(), userBps: 10000 }];

  const configRes = await fetch("https://api.bags.fm/v1/config/create-fee-share", {
    method: "POST",
    headers,
    body: JSON.stringify({ payer: wallet.publicKey.toBase58(), baseMint: mintAddress, feeClaimers: claimers }),
  });
  if (!configRes.ok) throw new Error(`Bags.fm fee-config error (${configRes.status}): ${await configRes.text()}`);
  const configData = await configRes.json();

  for (const txBase64 of configData.transactions || []) {
    const tx = Transaction.from(Buffer.from(txBase64, "base64"));
    const signed = await wallet.signTransaction(tx);
    await sendAndConfirm(connection, signed);
  }

  // Step 3: Create launch transaction
  const launchRes = await fetch("https://api.bags.fm/v1/token/create-launch-tx", {
    method: "POST",
    headers,
    body: JSON.stringify({
      metadataUrl: tokenData.tokenMetadata,
      tokenMint: mintAddress,
      launchWallet: wallet.publicKey.toBase58(),
      initialBuyLamports: Math.round(buyAmount * 1_000_000_000),
      configKey: configData.meteoraConfigKey,
    }),
  });
  if (!launchRes.ok) throw new Error(`Bags.fm launch error (${launchRes.status}): ${await launchRes.text()}`);
  const launchData = await launchRes.json();

  const launchTx = Transaction.from(Buffer.from(launchData.transaction, "base64"));
  const signedLaunch = await wallet.signTransaction(launchTx);
  const sig = await sendAndConfirm(connection, signedLaunch);

  return {
    mint: mintAddress,
    signature: sig,
    explorerUrl: `https://explorer.solana.com/tx/${sig}?cluster=mainnet-beta`,
    platform: "bags",
  };
}

// Copy button

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      onClick={() => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 2000); }}
      style={{
        position: "absolute", top: 8, right: 8,
        background: copied ? "var(--emerald)" : "var(--surface-2)",
        color: copied ? "#fff" : "var(--text-tertiary)",
        border: "none", borderRadius: 4, padding: "4px 10px",
        fontSize: "0.6875rem", cursor: "pointer", fontFamily: "var(--font-body)",
        transition: "background 0.15s",
      }}
    >
      {copied ? "Copied" : "Copy"}
    </button>
  );
}

// Main page

export default function LaunchPage() {
  const { publicKey, connected, signTransaction } = useWallet();
  const { connection } = useConnection();
  const { setVisible } = useWalletModal();

  // State
  const [platform, setPlatform] = useState<Platform>("rtp");
  const [phase, setPhase] = useState<LaunchPhase>("form");
  const [result, setResult] = useState<LaunchResult | null>(null);
  const [rtpResult, setRtpResult] = useState<RTPTokenResult | null>(null);
  const [treasuryState, setTreasuryState] = useState<TreasuryState | null>(null);
  const [adopterState, setAdopterState] = useState<AdopterState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [statusMsg, setStatusMsg] = useState("");

  // Shared fields
  const [projectName, setProjectName] = useState("");
  const [tokenSymbol, setTokenSymbol] = useState("");

  // RTP-specific
  const [totalSupply, setTotalSupply] = useState("1000000000");
  const [feeBps, setFeeBps] = useState("200");
  const [betaMode, setBetaMode] = useState(true);

  // Metaplex-specific
  const [metaSupply, setMetaSupply] = useState("500000000");
  const [raiseGoal, setRaiseGoal] = useState("250");

  // Pump.fun / Bags shared
  const [description, setDescription] = useState("");
  const [imageUrl, setImageUrl] = useState("");
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState("");
  const [dragOver, setDragOver] = useState(false);
  const [website, setWebsite] = useState("");
  const [twitter, setTwitter] = useState("");
  const [telegram, setTelegram] = useState("");

  // Pump.fun-specific
  const [devBuyAmount, setDevBuyAmount] = useState("0.1");

  // Bags-specific
  const [bagsBuyAmount, setBagsBuyAmount] = useState("0.1");
  const [feeClaimers, setFeeClaimers] = useState("");
  const [bagsApiKey, setBagsApiKey] = useState(() => getStored("rtp_bags_api_key"));

  const addr = publicKey
    ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}`
    : null;

  const wallet = publicKey && signTransaction
    ? { publicKey, signTransaction: signTransaction as <T extends VersionedTransaction | Transaction>(tx: T) => Promise<T> }
    : null;

  const reset = () => {
    setPhase("form");
    setResult(null);
    setRtpResult(null);
    setTreasuryState(null);
    setAdopterState(null);
    setError(null);
    setStatusMsg("");
  };

  const handleImageUpload = async (file: File) => {
    if (!file.type.startsWith("image/")) { setUploadError("Please select an image file"); return; }
    if (file.size > 10 * 1024 * 1024) { setUploadError("Max 10MB"); return; }
    setUploading(true);
    setUploadError("");
    try {
      const url = await uploadImageToPinata(file);
      if (url) { setImageUrl(url); } else { setUploadError("Upload failed — try pasting a URL instead"); }
    } catch { setUploadError("Upload failed"); }
    setUploading(false);
  };

  const onDrop = (e: React.DragEvent) => { e.preventDefault(); setDragOver(false); const f = e.dataTransfer.files[0]; if (f) handleImageUpload(f); };
  const onFileChange = (e: React.ChangeEvent<HTMLInputElement>) => { const f = e.target.files?.[0]; if (f) handleImageUpload(f); };

  const imageUploadWidget = (id: string) => (
    <div className="form-group">
      <label className="form-label">Token Image</label>
      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
        {imageUrl ? (
          <img src={imageUrl} alt="" style={{ width: 40, height: 40, borderRadius: "0.375rem", objectFit: "cover", flexShrink: 0 }} />
        ) : (
          <div style={{ width: 40, height: 40, borderRadius: "0.375rem", background: "rgba(255,255,255,0.05)", flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center", fontSize: "1.25rem", opacity: 0.3 }}>+</div>
        )}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: "0.25rem" }}>
          <label htmlFor={id} style={{ fontSize: "0.8125rem", cursor: "pointer", color: "var(--coral)", textDecoration: "underline", textUnderlineOffset: "2px" }}>
            {uploading ? "Uploading..." : imageUrl ? "Change image" : "Choose image"}
          </label>
          <input id={id} type="file" accept="image/*" style={{ display: "none" }} onChange={onFileChange} />
          {imageUrl && <span style={{ fontSize: "0.7rem", opacity: 0.4, wordBreak: "break-all" }}>ipfs://{imageUrl.split("/").pop()}</span>}
        </div>
      </div>
      {uploadError && <div style={{ color: "var(--coral)", fontSize: "0.75rem", marginTop: "0.25rem" }}>{uploadError}</div>}
      <input className="form-input" type="url" placeholder="Or paste image URL" value={imageUrl && !imageUrl.startsWith("https://ipfs.io") ? imageUrl : ""} onChange={(e) => e.target.value && setImageUrl(e.target.value)} style={{ marginTop: "0.5rem", fontSize: "0.8125rem" }} />
    </div>
  );

  // Main launch handler

  const handleLaunch = useCallback(async () => {
    if (!wallet || !publicKey) return;
    setPhase("launching");
    setError(null);

    try {
      let launchResult: LaunchResult;

      switch (platform) {
        case "rtp": {
          setStatusMsg("Creating Token-2022 mint + TransferFeeConfig...");
          const rtp = await createRTPToken(connection, wallet, {
            name: projectName || "My Token",
            symbol: tokenSymbol || "TKN",
            supply: parseInt(totalSupply) || 1_000_000_000,
            feeBps: parseInt(feeBps) || 200,
          });
          setRtpResult(rtp);
          launchResult = {
            mint: rtp.mint, signature: rtp.signature, explorerUrl: rtp.explorerUrl,
            platform: "rtp", treasuryPDA: rtp.treasuryPDA, vaultPDA: rtp.vaultPDA,
          };
          if (betaMode) {
            setStatusMsg("Registering beta adopter (expires May 18)...");
            try {
              await registerAdopterBeta(connection, wallet, rtp.mint, BETA_EXPIRES_AT);
            } catch (e: unknown) {
              console.warn("[Launch] Beta adopter registration failed:", e instanceof Error ? e.message : String(e));
            }
          }
          break;
        }

        case "pumpfun": {
          setStatusMsg("Calling PumpPortal API...");
          launchResult = await launchPumpFun({
            connection, wallet, name: projectName || "My Token", symbol: tokenSymbol || "TKN",
            imageUrl, description, website, twitter, telegram,
            devBuyAmount: parseFloat(devBuyAmount) || 0.1,
          });
          break;
        }

        case "metaplex": {
          setStatusMsg("Calling Metaplex Genesis API...");
          launchResult = await launchMetaplex({
            connection, wallet, name: projectName || "My Token", symbol: tokenSymbol || "TKN",
            imageUrl, description,
            supply: parseInt(metaSupply) || 500_000_000,
            raiseGoal: parseFloat(raiseGoal) || 250,
          });
          break;
        }

        case "bags": {
          if (!bagsApiKey) throw new Error("Bags.fm API key required. Get one at dev.bags.fm");
          setStatusMsg("Calling Bags.fm API...");
          launchResult = await launchBags({
            connection, wallet, apiKey: bagsApiKey,
            name: projectName || "My Token", symbol: tokenSymbol || "TKN",
            imageUrl, description, website, twitter, telegram,
            buyAmount: parseFloat(bagsBuyAmount) || 0.1, feeClaimers,
          });
          break;
        }
      }

      setResult(launchResult);

      // RTP treasury init for non-RTP platforms
      if (platform !== "rtp" && launchResult.mint) {
        setPhase("rtp_init");
        setStatusMsg("Initializing RTP treasury for new mint...");
        try {
          const rtp = await createRTPToken(connection, wallet, {
            name: projectName || "My Token", symbol: tokenSymbol || "TKN",
            supply: 1_000_000_000, feeBps: 200,
          });
          setRtpResult(rtp);
          launchResult.treasuryPDA = rtp.treasuryPDA;
          launchResult.vaultPDA = rtp.vaultPDA;
        } catch (e: unknown) {
          // Non-RTP platforms don't always support Token-2022 wrapping — token created, treasury skipped
          console.warn("[Launch] RTP treasury init skipped:", e instanceof Error ? e.message : String(e));
        }
      }

      setPhase("success");

      // Fetch treasury state (best-effort)
      setTimeout(async () => {
        try {
          const mint = launchResult.treasuryPDA ? launchResult.mint : rtpResult?.mint || launchResult.mint;
          const state = await fetchTreasuryState(connection, mint);
          setTreasuryState(state);
        } catch (e: unknown) {
          console.warn("[Launch] Treasury state fetch failed:", e instanceof Error ? e.message : String(e));
        }
        try {
          const mint = launchResult.treasuryPDA ? launchResult.mint : rtpResult?.mint || launchResult.mint;
          const adopter = await fetchAdopterState(connection, mint);
          setAdopterState(adopter);
        } catch (e: unknown) {
          console.warn("[Launch] Adopter state fetch failed:", e instanceof Error ? e.message : String(e));
        }
      }, 3000);

    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase("error");
    }
  }, [wallet, publicKey, signTransaction, connection, platform,
    projectName, tokenSymbol, totalSupply, feeBps, betaMode, imageUrl, description,
    website, twitter, telegram, devBuyAmount, metaSupply, raiseGoal,
    bagsApiKey, bagsBuyAmount, feeClaimers, rtpResult]);

  const canLaunch = connected && !!publicKey && !!projectName && !!tokenSymbol && (platform !== "bags" || !!bagsApiKey);

  // Render

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
          <Link href="/docs" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>Docs</Link>
          <Link href="/launch" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px", borderColor: "var(--coral-dim)", color: "var(--coral)" }}>Platform Demo</Link>
          <Link href="/research" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>Research</Link>
          <Link href="/" className="btn-connect" style={{ textDecoration: "none", fontSize: "0.8125rem", padding: "6px 14px" }}>Dashboard</Link>
          {connected && publicKey ? (
            <div className="wallet-pill">
              <span className="wallet-indicator" />
              <span className="wallet-addr">{addr}</span>
            </div>
          ) : (
            <button className="btn-connect" onClick={() => setVisible(true)}>Connect Wallet</button>
          )}
        </div>
      </header>

      {/* Hero */}
      <section className="launch-hero">
        <h1 className="launch-title">Instant Token Deploy</h1>
        <p className="launch-subtitle">
          Choose your launch platform. Click Launch, sign with Phantom, token is live on-chain.
          RTP treasury is initialized automatically for every token.
        </p>
      </section>

      {/* Platform selector */}
      <section className="platform-selector-wrap">
        <div className="platform-grid">
          {PLATFORMS.map((p) => (
            <button
              key={p.id}
              onClick={() => { setPlatform(p.id); reset(); }}
              style={{
                background: platform === p.id ? "rgba(255,255,255,0.06)" : "rgba(255,255,255,0.02)",
                border: `1px solid ${platform === p.id ? p.color : "var(--border)"}`,
                borderRadius: 8, padding: "var(--space-md)", cursor: "pointer",
                textAlign: "left" as const, transition: "border-color 0.15s, background 0.15s",
                color: "inherit", fontFamily: "var(--font-body)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
                <div style={{ fontSize: "0.875rem", fontWeight: 500, color: p.color }}>{p.name}</div>
                <span style={{
                  fontSize: "0.5625rem", fontWeight: 600, letterSpacing: "0.08em",
                  color: p.color, background: `${p.color}18`,
                  padding: "2px 5px", borderRadius: 3, textTransform: "uppercase",
                }}>INSTANT</span>
              </div>
              <div style={{ fontSize: "0.6875rem", color: "var(--text-tertiary)", lineHeight: 1.45 }}>{p.desc}</div>
              <div style={{ fontSize: "0.625rem", color: "var(--text-muted)", marginTop: 8, fontFamily: "var(--font-mono)", letterSpacing: "0.03em" }}>{p.token}</div>
            </button>
          ))}
        </div>
      </section>

      {/* Wallet connect prompt */}
      {!connected && (phase === "form" || phase === "error") && (
        <section className="launch-form-section" style={{ textAlign: "center", padding: "48px 24px" }}>
          <p style={{ color: "var(--text-secondary)", marginBottom: "24px", fontSize: "1.1rem" }}>
            Connect your Phantom wallet to launch a token instantly.
          </p>
          <button className="btn-launch" onClick={() => setVisible(true)}>Connect Phantom Wallet</button>
        </section>
      )}

      {/* Phase: FORM */}
      {connected && (phase === "form" || phase === "error") && (
        <section className="launch-form-section">
          <h3 style={{
            fontSize: "0.8125rem", fontWeight: 500, letterSpacing: "0.08em",
            textTransform: "uppercase", color: "var(--text-tertiary)",
            marginBottom: "var(--space-lg)", textAlign: "center",
          }}>
            Configure your token &rarr;
          </h3>

          <form className="launch-form" onSubmit={(e) => { e.preventDefault(); handleLaunch(); }}>
            {/* ── Shared: name + symbol ── */}
            <div className="form-group">
              <label className="form-label" htmlFor="projectName">Project Name</label>
              <input id="projectName" className="form-input" type="text" placeholder="e.g. My Launchpad Token"
                value={projectName} onChange={(e) => setProjectName(e.target.value)} required />
            </div>
            <div className="form-group">
              <label className="form-label" htmlFor="tokenSymbol">Token Symbol</label>
              <input id="tokenSymbol" className="form-input" type="text" placeholder="e.g. MLT" maxLength={8}
                value={tokenSymbol} onChange={(e) => setTokenSymbol(e.target.value.toUpperCase())} required />
            </div>

            {/* ── RTP Direct fields ── */}
            {platform === "rtp" && (
              <>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="totalSupply">Total Supply (tokens)</label>
                    <input id="totalSupply" className="form-input" type="number" min="1"
                      value={totalSupply} onChange={(e) => setTotalSupply(e.target.value)} required />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="feeBps">
                      Transfer Fee (bps) <span className="form-hint">{(parseInt(feeBps || "0") / 100).toFixed(1)}%</span>
                    </label>
                    <input id="feeBps" className="form-input" type="number" min="0" max="500" step="10"
                      value={feeBps} onChange={(e) => setFeeBps(e.target.value)} required />
                  </div>
                </div>
                <div className="form-note">
                  The transfer fee destination is a per-mint vault PDA derived from the program ID
                  (<code>{PROGRAM_ID_SHORT.slice(0, 8)}...{PROGRAM_ID_SHORT.slice(-4)}</code>).
                  Each token gets its own treasury. Executed entirely in-browser on devnet.
                </div>
                <div className="form-group" style={{ marginTop: "0.75rem" }}>
                  <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", cursor: "pointer" }}>
                    <input type="checkbox" checked={betaMode} onChange={(e) => setBetaMode(e.target.checked)}
                      style={{ width: "1rem", height: "1rem", accentColor: "var(--coral)" }} />
                    <span style={{ fontSize: "0.875rem" }}>
                      <strong>Colosseum Beta</strong> — free until May 18
                    </span>
                  </label>
                  {betaMode && (
                    <div style={{
                      marginTop: "0.4rem", padding: "0.5rem 0.75rem", borderRadius: "0.5rem",
                      background: "var(--surface, rgba(255,255,255,0.05))", fontSize: "0.8rem",
                      border: "1px solid var(--coral, #ff6b6b)", borderLeft: "3px solid var(--coral, #ff6b6b)",
                    }}>
                      Trading fees work for you until May 18 23:59 UTC. After that, the swarm stops
                      managing your treasury and fees return to normal. Yield generated during beta stays with you.
                    </div>
                  )}
                </div>
              </>
            )}

            {/* ── Metaplex fields ── */}
            {platform === "metaplex" && (
              <>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="metaSupply">Token Allocation (tokens to sell)</label>
                    <input id="metaSupply" className="form-input" type="number" min="1"
                      value={metaSupply} onChange={(e) => setMetaSupply(e.target.value)} required />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="launchType">Launch Type</label>
                    <input id="launchType" className="form-input" type="text" value="Launchpool (fair launch)"
                      disabled style={{ opacity: 0.6, cursor: "not-allowed" }} />
                  </div>
                </div>
                <div className="form-group">
                  <label className="form-label" htmlFor="raiseGoal">Raise Goal (SOL) <span className="form-hint">min 250</span></label>
                  <input id="raiseGoal" className="form-input" type="number" min="250" step="1"
                    value={raiseGoal} onChange={(e) => setRaiseGoal(e.target.value)} required />
                </div>
                {imageUploadWidget("imgUpload-metaplex")}
                <div className="form-group">
                  <label className="form-label" htmlFor="metaDesc">Description</label>
                  <input id="metaDesc" className="form-input" type="text" placeholder="Token description"
                    value={description} onChange={(e) => setDescription(e.target.value)} />
                </div>
                <div className="form-note">
                  Calls the Metaplex Genesis REST API to create a launch pool. Phantom signs the transaction
                  in-browser. No SDK install needed. Metaplex is a Colosseum sponsor.
                </div>
              </>
            )}

            {/* ── Pump.fun fields ── */}
            {platform === "pumpfun" && (
              <>
                <div className="form-group">
                  <label className="form-label" htmlFor="pfDesc">Description</label>
                  <input id="pfDesc" className="form-input" type="text" placeholder="Token description for Pump.fun"
                    value={description} onChange={(e) => setDescription(e.target.value)} />
                </div>
                {imageUploadWidget("imgUpload-pumpfun")}
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfWeb">Website</label>
                    <input id="pfWeb" className="form-input" type="url" placeholder="https://mytoken.com"
                      value={website} onChange={(e) => setWebsite(e.target.value)} />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfTw">Twitter</label>
                    <input id="pfTw" className="form-input" type="text" placeholder="@mytoken"
                      value={twitter} onChange={(e) => setTwitter(e.target.value)} />
                  </div>
                </div>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfTg">Telegram</label>
                    <input id="pfTg" className="form-input" type="text" placeholder="t.me/mytoken"
                      value={telegram} onChange={(e) => setTelegram(e.target.value)} />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="pfBuy">Dev Buy Amount (SOL)</label>
                    <input id="pfBuy" className="form-input" type="number" min="0" step="0.01"
                      value={devBuyAmount} onChange={(e) => setDevBuyAmount(e.target.value)} />
                  </div>
                </div>
                <div className="form-note">
                  Calls PumpPortal API to build a local <code>VersionedTransaction</code> — signed
                  in-browser by Phantom. No API key needed. Pure client-side. Token-2022 with TransferFeeConfig
                  is not compatible with Pump.fun&apos;s bonding curve; RTP treasury initialized separately.
                </div>
              </>
            )}

            {/* ── Bags.fm fields ── */}
            {platform === "bags" && (
              <>
                <div className="form-group">
                  <label className="form-label" htmlFor="bagsKey">
                    Bags.fm API Key <span className="form-hint">from dev.bags.fm</span>
                  </label>
                  <input id="bagsKey" className="form-input" type="password" placeholder="Your Bags.fm API key"
                    value={bagsApiKey}
                    onChange={(e) => { setBagsApiKey(e.target.value); setStored("rtp_bags_api_key", e.target.value); }} />
                </div>
                <div className="form-group">
                  <label className="form-label" htmlFor="bagsDesc">Description</label>
                  <input id="bagsDesc" className="form-input" type="text" placeholder="Token description"
                    value={description} onChange={(e) => setDescription(e.target.value)} />
                </div>
                {imageUploadWidget("imgUpload-bags")}
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsWeb">Website</label>
                    <input id="bagsWeb" className="form-input" type="url" placeholder="https://mytoken.com"
                      value={website} onChange={(e) => setWebsite(e.target.value)} />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsTw">Twitter</label>
                    <input id="bagsTw" className="form-input" type="text" placeholder="@mytoken"
                      value={twitter} onChange={(e) => setTwitter(e.target.value)} />
                  </div>
                </div>
                <div className="form-row">
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsBuy">Initial Buy Amount (SOL)</label>
                    <input id="bagsBuy" className="form-input" type="number" min="0" step="0.01"
                      value={bagsBuyAmount} onChange={(e) => setBagsBuyAmount(e.target.value)} />
                  </div>
                  <div className="form-group">
                    <label className="form-label" htmlFor="bagsClaimers">Fee Claimer (optional treasury PDA)</label>
                    <input id="bagsClaimers" className="form-input" type="text" placeholder="Treasury PDA address"
                      value={feeClaimers} onChange={(e) => setFeeClaimers(e.target.value)} />
                  </div>
                </div>
                <div className="form-note">
                  Bags.fm fee sharing routes creator fees to the treasury automatically.
                  Enter your API key from <a href="https://dev.bags.fm/" target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)" }}>dev.bags.fm</a>.
                  Key is stored in localStorage for convenience. The fee claimer can be set to your RTP treasury PDA.
                </div>
              </>
            )}

            {/* ── How it works (mini flow) ── */}
            <div style={{
              display: "flex", alignItems: "center", justifyContent: "center", gap: "var(--space-md)",
              padding: "var(--space-md) 0", borderTop: "1px solid var(--border)", borderBottom: "1px solid var(--border)",
            }}>
              {[
                { icon: "1", label: "Click Launch" },
                { icon: "→", label: "" },
                { icon: "2", label: "Phantom signs" },
                { icon: "→", label: "" },
                { icon: "3", label: "Token live" },
              ].map((step, i) => step.label ? (
                <div key={i} style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 4 }}>
                  <div style={{
                    width: 24, height: 24, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center",
                    background: "var(--surface-2)", color: "var(--text-secondary)", fontSize: "0.6875rem", fontWeight: 600,
                  }}>{step.icon}</div>
                  <span style={{ fontSize: "0.6875rem", color: "var(--text-tertiary)" }}>{step.label}</span>
                </div>
              ) : (
                <span key={i} style={{ color: "var(--text-muted)", fontSize: "0.75rem" }}>→</span>
              ))}
            </div>

            {/* ── Submit ── */}
            <button type="submit" className="btn-launch" disabled={!canLaunch}
              style={{
                opacity: canLaunch ? 1 : 0.5, cursor: canLaunch ? "pointer" : "not-allowed",
                background: PLATFORMS.find(p => p.id === platform)?.color,
                fontSize: "1rem", padding: "14px 32px", fontWeight: 600,
                letterSpacing: "0.02em",
              }}>
              Launch on {PLATFORMS.find(p => p.id === platform)?.name}
            </button>
          </form>

          {error && (
            <div style={{
              marginTop: "16px", padding: "12px 16px",
              background: "rgba(220, 38, 38, 0.1)", border: "1px solid rgba(220, 38, 38, 0.3)",
              borderRadius: "8px", color: "#f87171", fontSize: "0.875rem",
            }}>
              <strong>Error:</strong> {error}
            </div>
          )}
        </section>
      )}

      {/* Phase: LAUNCHING / RTP_INIT */}
      {(phase === "launching" || phase === "rtp_init") && (
        <section className="launch-form-section" style={{ textAlign: "center", padding: "64px 24px" }}>
          <div style={{ fontSize: "2rem", marginBottom: "16px" }}>{phase === "rtp_init" ? "🏗" : "⏳"}</div>
          <h2 style={{ fontSize: "1.25rem", marginBottom: "8px" }}>
            {phase === "rtp_init" ? "Initializing RTP treasury..." : "Launching your token..."}
          </h2>
          <p style={{ color: "var(--text-secondary)", fontSize: "0.875rem" }}>
            {statusMsg || "Signing transactions via Phantom. Check your wallet for approval prompts."}
          </p>
          <div style={{ marginTop: "24px" }}>
            <div style={{
              height: 3, background: "var(--surface-2)", borderRadius: 2, maxWidth: 300, margin: "0 auto",
              overflow: "hidden",
            }}>
              <div style={{
                height: "100%", background: "var(--coral)", borderRadius: 2,
                animation: "pulse 1.5s ease-in-out infinite", width: "60%",
              }} />
            </div>
          </div>
        </section>
      )}

      {/* Phase: SUCCESS */}
      {phase === "success" && result && (
        <section className="launch-result-section">
          <div className="launch-success">
            <span className="success-check">✓</span>
            <h2>Token Launched!</h2>
            <p className="success-subtitle">
              {platform === "rtp"
                ? "Your Token-2022 mint is live on devnet with fees routing to a per-mint treasury vault PDA."
                : `Your token is live on ${PLATFORMS.find(p => p.id === platform)?.name}. ${result.treasuryPDA ? "RTP treasury initialized." : "RTP treasury integration pending."}`}
            </p>
          </div>

          {/* Key info cards */}
          <div className="result-info" style={{ marginBottom: "24px" }}>
            <div className="info-card">
              <span className="info-label">Mint Address</span>
              <span className="info-value" style={{ fontSize: "0.75rem", wordBreak: "break-all" }}>{result.mint}</span>
              <a href={`https://explorer.solana.com/address/${result.mint}?cluster=${platform === "rtp" ? CLUSTER : "mainnet-beta"}`}
                target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)", fontSize: "0.75rem" }}>
                View on Explorer ↗
              </a>
            </div>
            {(result.treasuryPDA || rtpResult?.treasuryPDA) && (
              <div className="info-card">
                <span className="info-label">Treasury PDA</span>
                <span className="info-value" style={{ fontSize: "0.75rem", wordBreak: "break-all" }}>
                  {result.treasuryPDA || rtpResult?.treasuryPDA}
                </span>
                <a href={`https://explorer.solana.com/address/${result.treasuryPDA || rtpResult?.treasuryPDA}?cluster=${platform === "rtp" ? CLUSTER : "mainnet-beta"}`}
                  target="_blank" rel="noopener noreferrer" style={{ color: "var(--coral)", fontSize: "0.75rem" }}>
                  View on Explorer ↗
                </a>
              </div>
            )}
            {(result.vaultPDA || rtpResult?.vaultPDA) && (
              <div className="info-card">
                <span className="info-label">Vault PDA</span>
                <span className="info-value" style={{ fontSize: "0.75rem", wordBreak: "break-all" }}>
                  {result.vaultPDA || rtpResult?.vaultPDA}
                </span>
              </div>
            )}
            <div className="info-card">
              <span className="info-label">Transaction</span>
              <a href={result.explorerUrl} target="_blank" rel="noopener noreferrer"
                style={{ color: "var(--coral)", fontSize: "0.75rem", wordBreak: "break-all" }}>
                {result.signature.slice(0, 20)}...↗
              </a>
            </div>
          </div>

          {/* Platform-specific extras */}
          {platform === "pumpfun" && result.mint && (
            <div style={{ marginBottom: "24px", padding: "var(--space-md)", background: "rgba(0,210,140,0.06)", border: "1px solid rgba(0,210,140,0.2)", borderRadius: 6 }}>
              <a href={`https://pump.fun/${result.mint}`} target="_blank" rel="noopener noreferrer"
                style={{ color: "#00d18c", fontSize: "0.9375rem", fontWeight: 500, textDecoration: "none" }}>
                View on Pump.fun ↗
              </a>
            </div>
          )}
          {platform === "bags" && result.mint && (
            <div style={{ marginBottom: "24px", padding: "var(--space-md)", background: "rgba(184,169,232,0.06)", border: "1px solid rgba(184,169,232,0.2)", borderRadius: 6 }}>
              <a href={`https://bags.fm/${result.mint}`} target="_blank" rel="noopener noreferrer"
                style={{ color: "#B8A9E8", fontSize: "0.9375rem", fontWeight: 500, textDecoration: "none" }}>
                View on Bags.fm ↗
              </a>
            </div>
          )}

          {/* Treasury state (if loaded) */}
          {treasuryState && (
            <div style={{ background: "rgba(0,0,0,0.3)", border: "1px solid var(--border)", borderRadius: "8px", padding: "16px", marginBottom: "24px" }}>
              <h3 style={{ fontSize: "0.875rem", color: "var(--coral)", marginBottom: "12px" }}>Treasury State (on-chain)</h3>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px", fontSize: "0.8125rem" }}>
                <div><span style={{ color: "var(--text-secondary)" }}>Phase:</span> {treasuryState.phase}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Vault Balance:</span> {treasuryState.vaultBalance}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Fees Withdrawn:</span> {treasuryState.totalFeesWithdrawn}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Runway Floor:</span> {treasuryState.minRunwayBalance}</div>
              </div>
            </div>
          )}

          {/* Adopter state (if loaded) */}
          {adopterState && (
            <div style={{
              background: adopterState.isBeta ? "rgba(255,107,107,0.08)" : "rgba(0,0,0,0.3)",
              border: `1px solid ${adopterState.isBeta ? "var(--coral, #ff6b6b)" : "var(--border)"}`,
              borderLeft: adopterState.isBeta ? "3px solid var(--coral, #ff6b6b)" : undefined,
              borderRadius: "8px", padding: "16px", marginBottom: "24px",
            }}>
              <h3 style={{ fontSize: "0.875rem", color: "var(--coral)", marginBottom: "12px" }}>
                {adopterState.isBeta ? "Beta Adopter" : "Permanent Adopter"}
              </h3>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px", fontSize: "0.8125rem" }}>
                <div><span style={{ color: "var(--text-secondary)" }}>Status:</span>{" "}
                  {adopterState.betaEnded ? "Ended" : adopterState.isBeta ? "Active beta" : "Permanent"}
                </div>
                <div><span style={{ color: "var(--text-secondary)" }}>Deposits:</span> {adopterState.depositCount}</div>
                {adopterState.isBeta && !adopterState.betaEnded && (
                  <div style={{ gridColumn: "1 / -1" }}>
                    <span style={{ color: "var(--text-secondary)" }}>Expires:</span>{" "}
                    {new Date(adopterState.betaExpiresAt * 1000).toISOString().slice(0, 10)}
                  </div>
                )}
              </div>
            </div>
          )}

          <div className="result-actions">
            <button className="btn-secondary" onClick={reset}>Launch Another Token</button>
          </div>

          {/* Info cards */}
          <div className="result-info">
            <div className="info-card">
              <span className="info-label">Platform</span>
              <span className="info-value">{PLATFORMS.find(p => p.id === platform)?.name}</span>
              <span className="info-note">{PLATFORMS.find(p => p.id === platform)?.token}</span>
            </div>
            <div className="info-card">
              <span className="info-label">Fee Destination</span>
              <span className="info-value">{platform === "rtp" ? "Per-mint vault PDA" : "Platform + RTP"}</span>
              <span className="info-note">{platform === "rtp" ? "Program-owned, immutable" : "RTP treasury tracks yield"}</span>
            </div>
            <div className="info-card">
              <span className="info-label">Redistribution</span>
              <span className="info-value">70% holders / 20% dev / 10% ecosystem</span>
              <span className="info-note">Enforced on-chain by Anchor program</span>
            </div>
          </div>
        </section>
      )}

      {/* Footer */}
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
        <Link href="/" className="vital-link">Back to Dashboard ↗</Link>
      </footer>
    </div>
  );
}
