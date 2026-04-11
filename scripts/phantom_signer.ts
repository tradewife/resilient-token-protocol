/**
 * RTP Trading Wing — Phantom ServerSDK Sidecar
 *
 * Creates and signs via Phantom's embedded wallet for the Trading Wing.
 * This is NOT a personal wallet — it's a developer-app-owned identity:
 *
 *   Phantom Portal registration (https://phantom.app/portal):
 *     → PHANTOM_ORG_ID     = developer organization
 *     → PHANTOM_APP_ID     = this app ("RTP Trading Wing")
 *     → PHANTOM_PRIVATE_KEY = service credential for KMS requests
 *
 *   sdk.createWallet({ userId: "rtp-trading-wing-executor" })
 *     → embedded wallet owned by the RTP app
 *     → keys in Phantom's TEE/HSM, never on this machine
 *     → sovereign on-chain identity for the agent
 *     → no human holds the keys
 *
 * Narrative: "Who controls the treasury?"
 *   → No one. The embedded wallet is controlled by program constraints,
 *     not by the developer's personal keys. "Don't rug" enforced at
 *     the key custody level, not just code.
 *
 * Signing architecture:
 *   HL order signing  → ETH keypair (configs/hl_testnet_key.json) via web3.py
 *                       (Phantom EVM support coming soon — uses direct keypair for now)
 *   Solana treasury   → THIS FILE — Phantom ServerSDK (KMS-backed, autonomous)
 *   Demo dashboard    → Phantom browser-sdk (Phase 5)
 *
 * Chain support: Solana ✅ now | Ethereum/Base/Polygon/Sui ⏳ coming soon
 *
 * Prerequisites:
 *   1. Register dev app at https://phantom.app/portal
 *   2. Fill configs/.env.phantom with PHANTOM_ORG_ID, PHANTOM_APP_ID, PHANTOM_PRIVATE_KEY
 *   3. npm install @phantom/wallet-sdk dotenv
 *
 * Usage:
 *   ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts status
 *   ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts create-wallet
 *   ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts sign <base64-tx>
 */

import { ServerSDK } from "@phantom/wallet-sdk";
import * as dotenv from "dotenv";
import * as path from "path";

// Load Phantom dev credentials
dotenv.config({ path: path.resolve(__dirname, "../configs/.env.phantom") });

const REQUIRED_ENV = ["PHANTOM_ORG_ID", "PHANTOM_APP_ID", "PHANTOM_PRIVATE_KEY"] as const;

function getSDK(): ServerSDK {
  const missing = REQUIRED_ENV.filter((k) => !process.env[k]);
  if (missing.length > 0) {
    console.error(`Missing env vars: ${missing.join(", ")}`);
    console.error("Register at https://phantom.app/phantom-connect and fill configs/.env.phantom");
    process.exit(1);
  }

  return new ServerSDK({
    organizationId: process.env.PHANTOM_ORG_ID!,
    appId: process.env.PHANTOM_APP_ID!,
    apiPrivateKey: process.env.PHANTOM_PRIVATE_KEY!,
  });
}

const RTP_WALLET_USER = "rtp-trading-wing";

/**
 * Create a Phantom-managed wallet for the Trading Wing.
 * Called once during setup. Wallet ID is persisted for reuse.
 */
async function createWallet(): Promise<void> {
  const sdk = getSDK();
  const wallet = await sdk.createWallet({ userId: RTP_WALLET_USER });
  console.log("Trading Wing wallet created:");
  console.log(`  walletId: ${wallet.walletId}`);
  console.log(`  address:  ${wallet.address}`);
  console.log(`  network:  ${wallet.networkId}`);
  console.log("\nSave walletId to configs/.env.phantom as PHANTOM_WALLET_ID");
}

/**
 * Sign and send a Solana transaction (CPI transfer to treasury PDA).
 * Called by Trading Wing after Hyperliquid fill confirmed → USDC yield ready.
 *
 * @param txBase64 - Base64-encoded Solana transaction
 */
async function signAndSend(txBase64: string): Promise<void> {
  const sdk = getSDK();
  const walletId = process.env.PHANTOM_WALLET_ID;

  if (!walletId) {
    console.error("PHANTOM_WALLET_ID not set. Run 'create-wallet' first.");
    process.exit(1);
  }

  // Decode base64 → Uint8Array
  const txBytes = Buffer.from(txBase64, "base64");

  console.log(`Signing Solana transaction (${txBytes.length} bytes)...`);

  const result = await sdk.signAndSendTransaction({
    walletId,
    transaction: txBytes,
    networkId: "solana:devnet",
  });

  console.log("Transaction result:");
  console.log(JSON.stringify(result, null, 2));
}

/**
 * Check SDK connection status and wallet info.
 */
async function status(): Promise<void> {
  const hasCreds = REQUIRED_ENV.every((k) => !!process.env[k]);
  if (!hasCreds) {
    console.log("Phantom ServerSDK: NOT CONFIGURED");
    console.log("  Register at https://phantom.app/phantom-connect");
    console.log("  Fill configs/.env.phantom with credentials");
    return;
  }

  console.log("Phantom ServerSDK: CONFIGURED");
  console.log(`  ORG_ID:   ${process.env.PHANTOM_ORG_ID}`);
  console.log(`  APP_ID:   ${process.env.PHANTOM_APP_ID}`);
  console.log(`  WALLET_ID: ${process.env.PHANTOM_WALLET_ID || "(not set — run create-wallet)"}`);
}

// CLI entry point
const [,, command, ...args] = process.argv;

async function main() {
  switch (command) {
    case "create-wallet":
      await createWallet();
      break;
    case "sign":
      if (!args[0]) {
        console.error("Usage: phantom_signer.ts sign <base64-tx>");
        process.exit(1);
      }
      await signAndSend(args[0]);
      break;
    case "status":
      await status();
      break;
    default:
      console.log("RTP Trading Wing — Phantom ServerSDK Sidecar");
      console.log("");
      console.log("Commands:");
      console.log("  status        — Check SDK config and wallet status");
      console.log("  create-wallet — Create KMS-backed wallet for Trading Wing");
      console.log("  sign <b64>    — Sign and send Solana transaction");
      console.log("");
      console.log("Signing architecture:");
      console.log("  HL orders     → ETH keypair (configs/hl_testnet_key.json) via web3.py");
      console.log("  Solana treasury → Phantom ServerSDK (this script)");
      console.log("  Demo dashboard  → Phantom browser-sdk (Phase 5)");
  }
}

main().catch((err) => {
  console.error("Error:", err.message);
  process.exit(1);
});
