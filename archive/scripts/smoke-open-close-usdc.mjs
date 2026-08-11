#!/usr/bin/env node
/**
 * One-shot smoke test for the Flash v2 USDC-input open path.
 * Builds via REST /transaction-builder/open-position (USDC input),
 * signs ONLY the owner slot, submits to ER RPC, waits for the
 * position to be readable, then closes it immediately.
 *
 * Usage: node scripts/smoke-open-close-usdc.mjs [side=LONG|SHORT]
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { Connection, Keypair, VersionedTransaction } from "@solana/web3.js";

const API = process.env.FLASH_API_URL || "https://flashapi.trade";
const ER_RPC = process.env.RTP_TRADER_ER_RPC || "https://flash.magicblock.xyz";
const MAINNET_RPC = process.env.RTP_SOLANA_RPC_URL || "https://api.mainnet-beta.solana.com";
const WALLET = "HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5";

const KEYPAIR_PATH = process.env.RTP_TRADER_KEYPAIR_PATH || path.join(os.homedir(), ".config/solana/rtp-trader.json");
const secret = JSON.parse(fs.readFileSync(KEYPAIR_PATH, "utf8"));
const keypair = Keypair.fromSecretKey(new Uint8Array(secret));
if (keypair.publicKey.toBase58() !== WALLET) {
  throw new Error(`keypair mismatch: ${keypair.publicKey.toBase58()} != ${WALLET}`);
}

const SIDE = (process.argv[2] || "LONG").toUpperCase();
const COLLATERAL_SOL = 0.18;
const LEVERAGE = 9;

async function post(url, body) {
  const r = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return r.json();
}

// Sign only this keypair's slot; never touch the blockhash or API pre-sigs.
// v1 web3.js VersionedTransaction.sign(signers) fills only the given signer's
// slot and leaves any pre-filled signatures intact.
function partialSign(tx) {
  tx.sign([keypair]);
}

async function sendEr(txBytes) {
  const b64 = Buffer.from(txBytes).toString("base64");
  const r = await post(ER_RPC, {
    jsonrpc: "2.0",
    id: 1,
    method: "sendTransaction",
    params: [b64, { encoding: "base64", skipPreflight: false, maxRetries: 3 }],
  });
  if (r.error) throw new Error(`ER sendTransaction: ${JSON.stringify(r.error)}`);
  return r.result;
}

async function waitSig(sig, attempts = 24) {
  const conn = new Connection(ER_RPC, "confirmed");
  for (let i = 0; i < attempts; i++) {
    const s = await conn.getSignatureStatuses([sig]);
    const st = s.value[0];
    if (st?.confirmationStatus === "confirmed" || st?.confirmationStatus === "finalized") {
      if (st.err) throw new Error(`tx ${sig} confirmed with error: ${JSON.stringify(st.err)}`);
      return;
    }
    await new Promise((res) => setTimeout(res, 1500));
  }
  throw new Error(`tx ${sig} not confirmed after ${attempts} polls`);
}

async function getPositions() {
  const r = await fetch(`${API}/positions/owner/${WALLET}?includePnlInLeverageDisplay=true`);
  const j = await r.json();
  if (j.positions) return j.positions;
  if (Array.isArray(j)) return j;
  if (j && typeof j === "object" && Object.keys(j).length > 0) return Object.values(j).filter((v) => v && (v.sideUi || v.side_ui));
  return [];
}

const priceR = await (await fetch(`${API}/prices/SOL`)).json();
const solPrice = Number(priceR.priceUi);
const inputUsd = COLLATERAL_SOL * solPrice;

console.log(`[smoke] side=${SIDE} collateral=${COLLATERAL_SOL} SOL (~$${inputUsd.toFixed(2)}) lev=${LEVERAGE} SOL=$${solPrice.toFixed(2)}`);

// 1. OPEN
const openBody = {
  inputTokenSymbol: "USDC",
  outputTokenSymbol: "SOL",
  inputAmountUi: inputUsd.toFixed(4),
  leverage: LEVERAGE,
  tradeType: SIDE,
  owner: WALLET,
  slippagePercentage: "1.0",
};
console.log("[smoke] open body:", JSON.stringify(openBody));
const oj = await post(`${API}/transaction-builder/open-position`, openBody);
if (oj.err || !oj.transactionBase64) {
  console.error("[smoke] OPEN BUILD FAILED:", JSON.stringify(oj).slice(0, 600));
  process.exit(1);
}
console.log(`[smoke] open built: entry=${oj.newEntryPrice} size=$${oj.youRecieveUsdUi}`);

const otx = VersionedTransaction.deserialize(Buffer.from(oj.transactionBase64, "base64"));
partialSign(otx);
const osig = await sendEr(otx.serialize());
console.log(`[smoke] OPEN TX: https://explorer.solana.com/tx/${osig}?cluster=mainnet-beta`);
await waitSig(osig);
console.log("[smoke] open confirmed");

// 2. Verify readability
let pos = null;
for (let i = 0; i < 10; i++) {
  const positions = await getPositions();
  pos = positions.find((p) => {
    const sideUi = p.sideUi ?? p.side_ui ?? "";
    return String(sideUi).toUpperCase() === SIDE && (p.marketSymbol === "SOL" || !p.marketSymbol);
  });
  if (pos) break;
  await new Promise((r) => setTimeout(r, 2000));
}
if (!pos) {
  console.error("[smoke] open confirmed but position NOT readable — investigate before continuing");
  process.exit(1);
}
const sizeUsd = pos.sizeUsdUi ?? pos.size_usd_ui;
console.log(`[smoke] position readable: side=${pos.sideUi ?? pos.side_ui} size=$${sizeUsd} entry=${pos.entryPriceUi ?? pos.entry_price_ui}`);

// 3. CLOSE immediately
const closeBody = {
  marketSymbol: "SOL",
  side: SIDE,
  inputUsdUi: sizeUsd,
  withdrawTokenSymbol: "SOL",
  owner: WALLET,
  closeAll: true,
  slippagePercentage: "1.0",
};
console.log("[smoke] close body:", JSON.stringify(closeBody));
const cj = await post(`${API}/transaction-builder/close-position`, closeBody);
if (cj.err || !cj.transactionBase64) {
  console.error("[smoke] CLOSE BUILD FAILED:", JSON.stringify(cj).slice(0, 600));
  console.error("[smoke] POSITION LEFT OPEN — close manually!");
  process.exit(1);
}
const ctx = VersionedTransaction.deserialize(Buffer.from(cj.transactionBase64, "base64"));
partialSign(ctx);
const csig = await sendEr(ctx.serialize());
console.log(`[smoke] CLOSE TX: https://explorer.solana.com/tx/${csig}?cluster=mainnet-beta`);
await waitSig(csig);
console.log(`[smoke] close confirmed (settledPnl=${cj.settledPnl ?? "n/a"})`);

// 4. Verify flat
for (let i = 0; i < 8; i++) {
  const positions = await getPositions();
  if (positions.length === 0) {
    console.log("[smoke] SMOKE TEST PASSED — flat after full open/close cycle");
    process.exit(0);
  }
  await new Promise((r) => setTimeout(r, 2000));
}
console.error("[smoke] close confirmed but position still visible — check manually");
process.exit(1);
