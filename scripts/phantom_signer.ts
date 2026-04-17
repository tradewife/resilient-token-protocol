/**
 * RTP Trading Wing — Phantom ServerSDK Sidecar
 *
 * Signs via Phantom's embedded wallet (not a personal wallet — app-owned identity).
 * Keys live in Phantom's TEE/HSM, never on this machine.
 *
 * Unified signing: HL orders (EVM), Solana treasury CPI (Solana), dashboard (browser-sdk).
 *
 * Prerequisites:
 *   1. Register at https://phantom.app/portal
 *   2. Fill configs/.env.phantom with PHANTOM_ORG_ID, PHANTOM_APP_ID, PHANTOM_PRIVATE_KEY
 *   3. npm install @phantom/server-sdk dotenv
 *
 * Usage:
 *   ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts status
 *   ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts create-wallet
 *   ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts sign-sol <base64-tx>
 *   ts-node --project scripts/tsconfig.json scripts/phantom_signer.ts sign-evm <hex-tx>
 */

import { ServerSDK, NetworkId } from "@phantom/server-sdk";
import * as dotenv from "dotenv";
import * as path from "path";

// Load Phantom dev credentials
dotenv.config({ path: path.resolve(__dirname, "../configs/.env.phantom") });

const REQUIRED_ENV = ["PHANTOM_ORG_ID", "PHANTOM_APP_ID", "PHANTOM_PRIVATE_KEY"] as const;

function getSDK(): ServerSDK {
  const missing = REQUIRED_ENV.filter((k) => !process.env[k]);
  if (missing.length > 0) {
    console.error(`Missing env vars: ${missing.join(", ")}`);
    console.error("Register at https://phantom.app/portal and fill configs/.env.phantom");
    process.exit(1);
  }

  return new ServerSDK({
    organizationId: process.env.PHANTOM_ORG_ID!,
    appId: process.env.PHANTOM_APP_ID!,
    apiPrivateKey: process.env.PHANTOM_PRIVATE_KEY!,
  });
}

function getWalletId(): string {
  const walletId = process.env.PHANTOM_WALLET_ID;
  if (!walletId) {
    console.error("PHANTOM_WALLET_ID not set. Run 'create-wallet' first.");
    process.exit(1);
  }
  return walletId;
}

/**
 * Create a Phantom-managed wallet for the Trading Wing.
 * Called once during setup. Wallet ID is persisted for reuse.
 */
async function createWallet(): Promise<void> {
  const sdk = getSDK();
  const result = await sdk.createWallet("rtp-trading-wing-executor");
  console.log("Trading Wing embedded wallet created:");
  console.log(`  walletId: ${result.walletId}`);
  for (const addr of result.addresses) {
    console.log(`  ${addr.addressType}: ${addr.address}`);
  }
  console.log("\nSave walletId to configs/.env.phantom as PHANTOM_WALLET_ID");
  console.log("Then run 'addresses' to see chain-specific addresses.");
}

/**
 * Get wallet addresses across all supported chains.
 */
async function showAddresses(): Promise<void> {
  const sdk = getSDK();
  const walletId = getWalletId();

  const addresses = await sdk.getWalletAddresses(walletId);
  console.log("Trading Wing wallet addresses:");
  for (const addr of addresses) {
    console.log(`  ${addr.addressType}: ${addr.address}`);
  }
}

/**
 * Sign and send a Solana transaction (CPI transfer to treasury PDA).
 * Called by Trading Wing after Hyperliquid fill confirmed → USDC yield ready.
 */
async function signSolana(txBase64: string): Promise<void> {
  const sdk = getSDK();
  const walletId = getWalletId();

  console.log(`Signing Solana transaction...`);

  const result = await sdk.signAndSendTransaction({
    walletId,
    transaction: Buffer.from(txBase64, "base64"),
    networkId: NetworkId.SOLANA_DEVNET,
  });

  console.log("Solana transaction result:");
  console.log(JSON.stringify(result, null, 2));
}

/**
 * Sign an EVM transaction (Hyperliquid order or other EVM tx).
 * For HL: the action hash is signed as a message, not a full tx.
 * The signed result is included in the HL API payload.
 */
async function signEvm(txHex: string): Promise<void> {
  const sdk = getSDK();
  const walletId = getWalletId();

  console.log(`Signing EVM transaction...`);

  const result = await sdk.signTransaction({
    walletId,
    transaction: txHex,
    networkId: NetworkId.ETHEREUM_MAINNET,
  });

  console.log("EVM transaction result:");
  console.log(JSON.stringify(result, null, 2));
}

/**
 * Sign a raw message (used for HL EIP-712 order signing).
 * The Trading Wing constructs the HL action, hashes it, and signs via this method.
 */
async function signMessage(message: string): Promise<void> {
  const sdk = getSDK();
  const walletId = getWalletId();

  console.log(`Signing message on Ethereum mainnet...`);

  const result = await sdk.signMessage({
    walletId,
    message,
    networkId: NetworkId.ETHEREUM_MAINNET,
  });

  console.log("Signed message result:");
  console.log(JSON.stringify(result, null, 2));
}

/**
 * Check SDK connection status and wallet info.
 */
async function status(): Promise<void> {
  const hasCreds = REQUIRED_ENV.every((k) => !!process.env[k]);
  if (!hasCreds) {
    console.log("Phantom ServerSDK: NOT CONFIGURED");
    console.log("  Register at https://phantom.app/portal");
    console.log("  Fill configs/.env.phantom with credentials");
    return;
  }

  console.log("Phantom ServerSDK: CONFIGURED");
  console.log(`  ORG_ID:    ${process.env.PHANTOM_ORG_ID}`);
  console.log(`  APP_ID:    ${process.env.PHANTOM_APP_ID}`);
  console.log(`  WALLET_ID: ${process.env.PHANTOM_WALLET_ID || "(not set — run create-wallet)"}`);
  console.log("");
  console.log("Supported chains:");
  console.log("  Solana ✅  | Ethereum ✅ | Base ✅    | Polygon ✅");
  console.log("  Arbitrum ✅ | Bitcoin ✅  | Sui ✅     | Monad ✅");
}

// CLI entry point
const [,, command, ...args] = process.argv;

async function main() {
  switch (command) {
    case "create-wallet":
      await createWallet();
      break;
    case "addresses":
      await showAddresses();
      break;
    case "sign-sol":
      if (!args[0]) {
        console.error("Usage: phantom_signer.ts sign-sol <base64-tx>");
        process.exit(1);
      }
      await signSolana(args[0]);
      break;
    case "sign-evm":
      if (!args[0]) {
        console.error("Usage: phantom_signer.ts sign-evm <hex-tx>");
        process.exit(1);
      }
      await signEvm(args[0]);
      break;
    case "sign-message":
      if (!args[0]) {
        console.error("Usage: phantom_signer.ts sign-message <message>");
        process.exit(1);
      }
      await signMessage(args[0]);
      break;
    case "status":
      await status();
      break;
    default:
      console.log("RTP Trading Wing — Phantom ServerSDK Sidecar");
      console.log("");
      console.log("Commands:");
      console.log("  status                    — Check SDK config and wallet status");
      console.log("  create-wallet             — Create KMS-backed embedded wallet");
      console.log("  addresses                 — Show wallet addresses across chains");
      console.log("  sign-sol <b64>            — Sign and send Solana transaction");
      console.log("  sign-evm <hex>            — Sign EVM transaction (HL orders)");
      console.log("  sign-message <msg>        — Sign raw message (HL EIP-712)");
      console.log("");
      console.log("Unified signing architecture (all through Phantom):");
      console.log("  Hyperliquid orders → signMessage/signTransaction (Ethereum mainnet)");
      console.log("  Solana treasury CPI → signAndSendTransaction (Solana devnet)");
      console.log("  Demo dashboard      → Phantom browser-sdk (Phase 5)");
  }
}

main().catch((err) => {
  console.error("Error:", err.message);
  process.exit(1);
});
