/**
 * Mainnet Proof-of-Yield Setup Script
 *
 * Self-funded: uses your own SOL in lieu of creator fees.
 * Proves the full RTP loop on mainnet before any token project adopts it.
 *
 * Steps:
 *   1. Deploy program to mainnet (manual: `solana program deploy`)
 *   2. Initialize treasury PDA (authority-seeded)
 *   3. Deposit SOL (self-funded, stands in for creator fees)
 *   4. Register adopter record
 *   5. Register strategy (SOL_FT_V1) with promotion Sharpe from Night Shift
 *   6. Open micro Flash Trade position via CPI
 *   7. Close position — SOL returns to treasury
 *   8. Check redistribution (70/20/10 split)
 *
 * Usage:
 *   # After deploying program and funding wallet:
 *   npx tsx scripts/mainnet-proof.ts --keypair ~/.config/solana/id.json --fund 0.5
 *   npx tsx scripts/mainnet-proof.ts --keypair ~/.config/solana/id.json --step init
 *   npx tsx scripts/mainnet-proof.ts --keypair ~/.config/solana/id.json --step deposit --amount 0.1
 *   npx tsx scripts/mainnet-proof.ts --keypair ~/.config/solana/id.json --step strategy
 *   npx tsx scripts/mainnet-proof.ts --keypair ~/.config/solani/id.json --step status
 */

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { readFileSync } from "fs";
import { resolve, basename } from "path";
import { AnchorProvider, BorshCoder, Program, BN } from "@coral-xyz/anchor";
import {
  registerWithRTP,
  depositSol,
  registerStrategy,
  fetchTreasuryState,
  deriveTreasuryPDA,
  RTP_PROGRAM_ID,
} from "../sdk/index";
import { IDL } from "../sdk/idl";

const MAINNET_RPC = "https://api.mainnet-beta.solana.com";

function loadKeypair(path: string): Keypair {
  const resolved = resolve(path);
  const data = JSON.parse(readFileSync(resolved, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(data));
}

function kpWallet(kp: Keypair) {
  return {
    publicKey: kp.publicKey,
    signTransaction: async (tx: Transaction) => {
      tx.partialSign(kp);
      return tx;
    },
    signAllTransactions: async (txs: Transaction[]) => {
      return txs.map((tx) => { tx.partialSign(kp); return tx; });
    },
  };
}

async function main() {
  const args = process.argv.slice(2);
  const keypairIdx = args.indexOf("--keypair");
  const stepIdx = args.indexOf("--step");
  const fundIdx = args.indexOf("--fund");
  const amountIdx = args.indexOf("--amount");

  if (keypairIdx === -1) {
    console.error("Usage: npx tsx scripts/mainnet-proof.ts --keypair <path> [--step <init|deposit|strategy|status>] [--amount <SOL>] [--fund <SOL>]");
    console.error("");
    console.error("Steps:");
    console.error("  init     — Initialize treasury + register adopter (one-time)");
    console.error("  deposit  — Deposit SOL into treasury (self-funded fees)");
    console.error("  strategy — Register SOL_FT_V1 strategy as Live");
    console.error("  status   — Show treasury state on mainnet");
    console.error("");
    console.error("--fund <SOL>  — Shortcut: init + deposit in one command");
    process.exit(1);
  }

  const keypairPath = args[keypairIdx + 1];
  const step = stepIdx !== -1 ? args[stepIdx + 1] : null;
  const fundAmount = fundIdx !== -1 ? parseFloat(args[fundIdx + 1]) : null;
  const depositAmount = amountIdx !== -1 ? parseFloat(args[amountIdx + 1]) : 0.1;

  const authority = loadKeypair(keypairPath);
  const connection = new Connection(MAINNET_RPC, "confirmed");
  const [treasuryPDA] = deriveTreasuryPDA(authority.publicKey);

  console.log("=== RTP Mainnet Proof-of-Yield ===");
  console.log(`Authority:    ${authority.publicKey.toBase58()}`);
  console.log(`Treasury PDA: ${treasuryPDA.toBase58()}`);
  console.log(`Program:      ${RTP_PROGRAM_ID.toBase58()}`);
  console.log(`Network:      mainnet-beta`);
  console.log("");

  // --fund shortcut: init + deposit
  if (fundAmount !== null) {
    console.log(`--- Fund mode: initializing treasury + depositing ${fundAmount} SOL ---`);

    // Step 1: Initialize
    console.log("[1/3] Initializing treasury...");
    try {
      const result = await registerWithRTP(connection, authority, {
        authority: authority.publicKey,
        holdersWallet: authority.publicKey,
        projectDevWallet: authority.publicKey,
        ecosystemWallet: authority.publicKey,
        minRunwayBalance: 50_000_000, // 0.05 SOL
      });
      console.log(`  TX: ${result.explorerUrl}`);
      console.log(`  Treasury PDA: ${result.treasuryPDA}`);
    } catch (e: any) {
      if (e.message?.includes("already in use") || e.message?.includes("0x1")) {
        console.log("  Treasury already initialized — skipping.");
      } else {
        console.error("  Init failed:", e.message);
        process.exit(1);
      }
    }

    // Step 2: Deposit SOL
    console.log(`[2/3] Depositing ${fundAmount} SOL into treasury...`);
    const depositResult = await depositSol(
      connection,
      authority,
      { authority: authority.publicKey },
      Math.round(fundAmount * LAMPORTS_PER_SOL),
    );
    console.log(`  TX: https://explorer.solana.com/tx/${depositResult}?cluster=mainnet-beta`);

    // Step 3: Register strategy
    console.log("[3/3] Registering strategy SOL_FT_V1...");
    try {
      const stratResult = await registerStrategy(
        connection,
        authority,
        { authority: authority.publicKey },
        "SOL_FT_V1",
        396, // OOS Sharpe 3.96 from Night Shift (x100)
      );
      console.log(`  TX: https://explorer.solana.com/tx/${stratResult}?cluster=mainnet-beta`);
    } catch (e: any) {
      if (e.message?.includes("already in use") || e.message?.includes("0x1")) {
        console.log("  Strategy already registered — skipping.");
      } else {
        console.error("  Strategy registration failed:", e.message);
      }
    }

    console.log("");
    console.log("--- Fund complete. Treasury is ready for Flash Trade CPI ---");
    return;
  }

  // Individual steps
  switch (step) {
    case "init": {
      console.log("Initializing treasury + adopter...");
      const result = await registerWithRTP(connection, authority, {
        authority: authority.publicKey,
        holdersWallet: authority.publicKey,
        projectDevWallet: authority.publicKey,
        ecosystemWallet: authority.publicKey,
        minRunwayBalance: 50_000_000, // 0.05 SOL
      });
      console.log(`TX: ${result.explorerUrl}`);
      console.log(`Treasury PDA: ${result.treasuryPDA}`);
      console.log(`Adopter PDA:  ${result.adopterPDA}`);
      break;
    }

    case "deposit": {
      console.log(`Depositing ${depositAmount} SOL into treasury...`);
      const result = await depositSol(
        connection,
        authority,
        { authority: authority.publicKey },
        Math.round(depositAmount * LAMPORTS_PER_SOL),
      );
      console.log(`TX: https://explorer.solana.com/tx/${result}?cluster=mainnet-beta`);
      break;
    }

    case "strategy": {
      console.log("Registering strategy SOL_FT_V1 (Sharpe 3.96, 9/9 folds profitable)...");
      const result = await registerStrategy(
        connection,
        authority,
        { authority: authority.publicKey },
        "SOL_FT_V1",
        396,
      );
      console.log(`TX: https://explorer.solana.com/tx/${result}?cluster=mainnet-beta`);
      break;
    }

    case "status": {
      console.log("Fetching treasury state from mainnet...");
      const state = await fetchTreasuryState(connection, authority.publicKey);
      console.log(`Phase:              ${state.phase}`);
      console.log(`SOL Balance:        ${(state.solBalance / LAMPORTS_PER_SOL).toFixed(6)} SOL`);
      console.log(`Available SOL:      ${(state.availableSolLamports / LAMPORTS_PER_SOL).toFixed(6)} SOL`);
      console.log(`Committed SOL:      ${(state.committedSolLamports / LAMPORTS_PER_SOL).toFixed(6)} SOL`);
      console.log(`Total Fees In:      ${state.totalFeesWithdrawn} lamports`);
      console.log(`Distributed (70%):  ${state.totalDistributedHolders} lamports`);
      console.log(`Distributed (20%):  ${state.totalDistributedDev} lamports`);
      console.log(`Distributed (10%):  ${state.totalDistributedEcosystem} lamports`);
      console.log(`Frozen:             ${state.frozen}`);
      console.log("");
      console.log(`Explorer: https://explorer.solana.com/address/${treasuryPDA.toBase58()}`);
      break;
    }

    default:
      console.error(`Unknown step: ${step}. Use: init, deposit, strategy, status`);
      process.exit(1);
  }
}

main().catch((e) => {
  console.error("Fatal:", e);
  process.exit(1);
});
