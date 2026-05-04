/**
 * Flash Trade Direct Loop — Self-Funded Proof-of-Yield
 *
 * Opens and closes a SOL long position on Flash Trade mainnet
 * using your own SOL. No RTP program deployment needed.
 *
 * Uses the Flash Trade REST API to build transactions:
 *   1. Query SOL price
 *   2. Build open-position tx (API returns unsigned VersionedTransaction)
 *   3. Sign with your keypair
 *   4. Submit to Solana mainnet
 *   5. Wait, then close position
 *   6. Record PnL
 *
 * Usage:
 *   npx tsx scripts/flash-trade-loop.ts --keypair ~/.config/solana/id.json
 *   npx tsx scripts/flash-trade-loop.ts --keypair ~/.config/solana/id.json --amount 0.02
 *   npx tsx scripts/flash-trade-loop.ts --keypair ~/.config/solani/id.json --dry-run
 */

import {
  Connection,
  Keypair,
  VersionedTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { readFileSync } from "fs";
import { resolve } from "path";

const FLASH_API = "https://flashapi.trade";
const MAINNET_RPC = "https://api.mainnet-beta.solana.com";

function loadKeypair(path: string): Keypair {
  const data = JSON.parse(readFileSync(resolve(path), "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(data));
}

async function flashApi(path: string, body?: object) {
  const url = `${FLASH_API}${path}`;
  const opts: RequestInit = body
    ? { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
    : {};
  const res = await fetch(url, opts);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Flash API ${res.status}: ${text}`);
  }
  return res.json();
}

async function main() {
  const args = process.argv.slice(2);
  const kpIdx = args.indexOf("--keypair");
  const amountIdx = args.indexOf("--amount");
  const dryRun = args.includes("--dry-run");

  if (kpIdx === -1) {
    console.error("Usage: npx tsx scripts/flash-trade-loop.ts --keypair <path> [--amount <SOL>] [--dry-run]");
    process.exit(1);
  }

  const keypair = loadKeypair(args[kpIdx + 1]);
  const wallet = keypair.publicKey.toBase58();
  const inputAmount = amountIdx !== -1 ? parseFloat(args[amountIdx + 1]) : 0.02; // 0.02 SOL default (~$3)
  const leverage = 1.0; // 1x — conservative for proof

  const connection = new Connection(MAINNET_RPC, "confirmed");

  console.log("=== Flash Trade Direct Loop — Self-Funded Proof-of-Yield ===");
  console.log(`Wallet:   ${wallet}`);
  console.log(`Amount:   ${inputAmount} SOL`);
  console.log(`Leverage: ${leverage}x`);
  console.log(`Mode:     ${dryRun ? "DRY RUN (preview only)" : "LIVE"}`);
  console.log("");

  // 1. Check wallet SOL balance
  const balance = await connection.getBalance(keypair.publicKey);
  console.log(`[1/7] Wallet balance: ${(balance / LAMPORTS_PER_SOL).toFixed(6)} SOL`);
  if (balance < inputAmount * LAMPORTS_PER_SOL + 5000) {
    console.error("Insufficient SOL for this trade + gas.");
    process.exit(1);
  }

  // 2. Get SOL price
  const prices = await flashApi("/prices");
  const solPrice = parseFloat(prices.SOL.priceUi);
  console.log(`[2/7] SOL price: $${solPrice.toFixed(2)}`);
  const positionUsd = inputAmount * solPrice * leverage;
  console.log(`      Position size: ~$${positionUsd.toFixed(2)} (${inputAmount} SOL × ${leverage}x)`);

  // 3. Check existing positions
  const positions = await flashApi(`/positions/owner/${wallet}?includePnlInLeverageDisplay=true`);
  if (positions.length > 0) {
    console.log(`[3/7] Existing positions: ${positions.length}`);
    for (const pos of positions) {
      console.log(`      ${pos.marketSymbol} ${pos.sideUi}: $${pos.sizeUsdUi} @ $${pos.entryPriceUi}, PnL: $${pos.pnlWithFeeUsdUi}`);
    }
  } else {
    console.log("[3/7] No existing positions — clean slate.");
  }

  // 4. Build open-position transaction
  console.log(`[4/7] Building open-position: ${inputAmount} SOL LONG @ ${leverage}x...`);
  const openResp = await flashApi("/transaction-builder/open-position", {
    inputTokenSymbol: "SOL",
    outputTokenSymbol: "SOL",
    inputAmountUi: inputAmount.toString(),
    leverage: leverage,
    tradeType: "LONG",
    owner: wallet,
    slippagePercentage: "1.0",
  });

  if (openResp.err) {
    console.error(`      API error: ${openResp.err}`);
    process.exit(1);
  }

  console.log(`      Entry price:  $${openResp.newEntryPrice}`);
  console.log(`      Entry fee:    $${openResp.entryFee}`);
  console.log(`      Liquidation:  $${openResp.newLiquidationPrice}`);
  console.log(`      You pay:      $${openResp.youPayUsdUi}`);
  console.log(`      Position:     $${openResp.youRecieveUsdUi}`);

  if (dryRun) {
    console.log("");
    console.log("[DRY RUN] Would submit open-position transaction.");
    console.log("[DRY RUN] Re-run without --dry-run to execute.");
    return;
  }

  if (!openResp.transactionBase64) {
    console.error("No transaction returned from API.");
    process.exit(1);
  }

  // 5. Sign and submit open
  console.log("[5/7] Signing and submitting open-position...");
  const openTxBuf = Buffer.from(openResp.transactionBase64, "base64");
  const openTx = VersionedTransaction.deserialize(openTxBuf);
  openTx.sign([keypair]);

  const openSig = await connection.sendRawTransaction(openTx.serialize(), {
    skipPreflight: false,
    maxRetries: 3,
  });
  console.log(`      TX: https://explorer.solana.com/tx/${openSig}?cluster=mainnet-beta`);

  // Wait for confirmation
  console.log("      Waiting for confirmation...");
  const latestBlockhash = await connection.getLatestBlockhash("confirmed");
  await connection.confirmTransaction(
    { signature: openSig, blockhash: latestBlockhash.blockhash, lastValidBlockHeight: latestBlockhash.lastValidBlockHeight },
    "confirmed",
  );
  console.log("      CONFIRMED.");

  // 6. Get position key from Flash Trade positions API
  const holdSeconds = 10;
  console.log(`[6/7] Holding for ${holdSeconds}s, then closing...`);
  await new Promise((r) => setTimeout(r, holdSeconds * 1000));

  // Fetch open positions to get the positionKey
  const openPositions = await flashApi(`/positions/owner/${wallet}?includePnlInLeverageDisplay=true`);
  const solLong = openPositions.find(
    (p: any) => p.marketSymbol === "SOL" && p.sideUi === "Long",
  );
  if (!solLong) {
    console.error("      No SOL Long position found after open — may have failed.");
    return;
  }
  const positionKey = solLong.key;
  const sizeUsd = solLong.sizeUsdUi;
  console.log(`      Position key: ${positionKey}`);
  console.log(`      Current size: $${sizeUsd}`);

  console.log("      Building close-position...");
  const closeResp = await flashApi("/transaction-builder/close-position", {
    positionKey,
    inputUsdUi: sizeUsd, // close full size
    withdrawTokenSymbol: "SOL",
    slippagePercentage: "1.0",
  });

  if (closeResp.err) {
    console.error(`      Close API error: ${closeResp.err}`);
    console.log("      Position is still open. Close manually or wait for the next loop cycle.");
    return;
  }

  console.log(`      Settled PnL:  $${closeResp.settledPnl}`);
  console.log(`      Fees:         $${closeResp.fees}`);
  console.log(`      Receive:      ${closeResp.receiveTokenAmountUi} ${closeResp.receiveTokenSymbol}`);

  // 7. Sign and submit close
  console.log("[7/7] Signing and submitting close-position...");
  const closeTxBuf = Buffer.from(closeResp.transactionBase64, "base64");
  const closeTx = VersionedTransaction.deserialize(closeTxBuf);
  closeTx.sign([keypair]);

  const closeSig = await connection.sendRawTransaction(closeTx.serialize(), {
    skipPreflight: false,
    maxRetries: 3,
  });
  console.log(`      TX: https://explorer.solana.com/tx/${closeSig}?cluster=mainnet-beta`);

  const closeBlockhash = await connection.getLatestBlockhash("confirmed");
  await connection.confirmTransaction(
    { signature: closeSig, blockhash: closeBlockhash.blockhash, lastValidBlockHeight: closeBlockhash.lastValidBlockHeight },
    "confirmed",
  );
  console.log("      CONFIRMED.");

  // Summary
  const newBalance = await connection.getBalance(keypair.publicKey);
  const solChange = (newBalance - balance) / LAMPORTS_PER_SOL;
  console.log("");
  console.log("=== Trade Complete ===");
  console.log(`PnL (on-paper): $${closeResp.settledPnl}`);
  console.log(`SOL balance change: ${solChange >= 0 ? "+" : ""}${solChange.toFixed(6)} SOL`);
  console.log(`Fees paid: $${closeResp.fees}`);
  console.log(`Open TX:  https://explorer.solana.com/tx/${openSig}?cluster=mainnet-beta`);
  console.log(`Close TX: https://explorer.solana.com/tx/${closeSig}?cluster=mainnet-beta`);
}

main().catch((e) => {
  console.error("Fatal:", e);
  process.exit(1);
});
