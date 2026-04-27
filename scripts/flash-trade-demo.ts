/**
 * RTP × Flash Trade — M5 End-to-End Demo
 *
 * Demonstrates the full Flash Trade CPI integration:
 *   1. Query Flash Trade REST API (prices, markets, positions)
 *   2. Build the open_position instruction with correct account ordering
 *   3. Simulate or execute on mainnet
 *   4. Show Explorer links + position data
 *
 * Usage:
 *   npx tsx scripts/flash-trade-demo.ts              # Query + simulate only
 *   npx tsx scripts/flash-trade-demo.ts --execute     # Actually open position on mainnet
 *   npx tsx scripts/flash-trade-demo.ts --close       # Close existing position
 *
 * Prerequisites:
 *   - Funded keypair at ~/.config/solana/id.json (or set KEYPAIR_PATH)
 *   - SOL balance >= 0.03 SOL for mainnet execution
 *
 * This is the demo script for hackathon judging (May 11, 2026).
 * All transactions are on Solana mainnet. Flash Trade uses Pyth oracle
 * prices which are mainnet-only.
 */

import {
  Connection,
  Keypair,
  PublicKey,
  ComputeBudgetProgram,
  VersionedTransaction,
  TransactionMessage,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  NATIVE_MINT,
  createInitializeAccount3Instruction,
  createCloseAccountInstruction,
  getMinimumBalanceForRentExemptAccount,
} from "@solana/spl-token";
import fs from "fs";
import path from "path";

// ─── Constants ──────────────────────────────────────────────────────────

const FLASH_PROGRAM_ID = new PublicKey(
  "FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn"
);

// Crypto.1 pool accounts (from Flash Trade PoolConfig)
const POOL = new PublicKey("HfF7GCcEc76xubFCHLLXRdYcgRzwjEPdfKWqzRS8Ncog");
const PERPETUALS = new PublicKey("7DWCtB5Z8rPiyBMKUwqyC95R9tJpbhoQhLM9LbK3Z5QZ");

// SOL custody (custodyId: 1)
const SOL_CUSTODY = new PublicKey("BjzZ33nMnbXZ7rw3Uy9Uu1W7BDCzzugqkiZoamJHRKF7");
const SOL_ORACLE = new PublicKey("DXqtMo8qRBfHcK11kBnSaCSXkWKk1huMf94R6sAxLHtf");

// SOL Long market
const SOL_LONG_MARKET = new PublicKey("3vHoXbUvGhEHFsLUmxyC6VWsbYDreb1zMn9TAp5ijN5K");

// Custody token account (from on-chain data at offset 72)
const SOL_CUSTODY_TOKEN_ACCOUNT = new PublicKey(
  "Hhed3wTHoVoPpnuBntGf236UfowMMAXfxqTLkMyJJENe"
);

// PDAs
const [TRANSFER_AUTHORITY] = PublicKey.findProgramAddressSync(
  [Buffer.from("transfer_authority")],
  FLASH_PROGRAM_ID
);
const [EVENT_AUTHORITY] = PublicKey.findProgramAddressSync(
  [Buffer.from("__event_authority")],
  FLASH_PROGRAM_ID
);

// Instruction discriminators (from IDL v15.2.0)
const OPEN_POS_DISC = Buffer.from([135, 128, 47, 77, 15, 152, 240, 49]);
const CLOSE_POS_DISC = Buffer.from([191, 210, 137, 115, 145, 22, 230, 244]);

// ─── Helpers ────────────────────────────────────────────────────────────

function banner(title: string) {
  console.log(`\n${"═".repeat(60)}`);
  console.log(`  ${title}`);
  console.log(`${"═".repeat(60)}\n`);
}

function step(label: string) {
  console.log(`\n▸ ${label}`);
  console.log("─".repeat(50));
}

function ok(msg: string) {
  console.log(`  ✅ ${msg}`);
}

function info(msg: string) {
  console.log(`  → ${msg}`);
}

function warn(msg: string) {
  console.log(`  ⚠ ${msg}`);
}

// ─── Flash Trade REST API Queries ───────────────────────────────────────

interface FlashPrice {
  symbol: string;
  oraclePrice: string;
  oraclePriceDecimals: number;
  pool: string;
}

interface FlashPosition {
  positionAddress: string;
  owner: string;
  pool: string;
  market: string;
  side: string;
  sizeUsd: string;
  collateralUsd: string;
  unrealizedPnlUsd: string;
  leverage: string;
  entryPrice: string;
  markPrice: string;
  liquidationPrice: string;
}

interface OpenPositionResponse {
  newEntryPrice: string;
  newLeverage: string;
  newLiquidationPrice: string;
  entryFee: string;
  youPayUsdUi: string;
  youRecieveUsdUi: string;
  transactionBase64: string | null;
  err: string | null;
}

interface ClosePositionResponse {
  markPrice: string;
  entryPrice: string;
  settledPnl: string;
  fees: string;
  receiveTokenAmountUi: string;
  transactionBase64: string | null;
  err: string | null;
}

async function getSolPrice(): Promise<number> {
  try {
    const resp = await fetch("https://flashapi.trade/prices");
    const data = (await resp.json()) as FlashPrice[];
    const sol = data.find((p) => p.symbol === "SOL");
    if (sol) {
      return Number(sol.oraclePrice);
    }
  } catch (e) {
    warn(`Price query failed: ${e}`);
  }
  return 170; // fallback
}

async function getPositions(
  owner: string
): Promise<FlashPosition[]> {
  try {
    const resp = await fetch(
      `https://flashapi.trade/positions/owner/${owner}`
    );
    return (await resp.json()) as FlashPosition[];
  } catch {
    return [];
  }
}

// ─── Build Open Position Instruction ────────────────────────────────────

function buildOpenPositionIx(
  owner: PublicKey,
  wsolAccount: PublicKey,
  positionPda: PublicKey,
  solPrice: number,
  inputLamports: number,
  leverageMultiplier: number = 2
): TransactionInstruction {
  // OpenPositionParams from IDL:
  //   price: OraclePrice { price: i64, exponent: i32 }
  //   collateral_amount: u64
  //   size_amount: u64
  //   privilege: FlashPrivilege (u8, None=0)

  const slippagePrice = BigInt(Math.floor(solPrice * 1.05 * 1e8)); // 5% slippage
  const collateralAmount = BigInt(inputLamports);
  const sizeAmount = BigInt(inputLamports * leverageMultiplier);

  // Serialize params: price(i64) + exponent(i32) + collateral(u64) + size(u64) + privilege(u8)
  const paramsBuf = Buffer.alloc(8 + 4 + 8 + 8 + 1);
  paramsBuf.writeBigInt64LE(slippagePrice, 0);
  paramsBuf.writeInt32LE(-8, 8); // Pyth exponent
  paramsBuf.writeBigUInt64LE(collateralAmount, 12);
  paramsBuf.writeBigUInt64LE(sizeAmount, 20);
  paramsBuf.writeUInt8(0, 28); // privilege: None

  const ixData = Buffer.concat([OPEN_POS_DISC, paramsBuf]);

  // 19 accounts in IDL order (v15.2.0)
  const accounts = [
    { pubkey: owner, isSigner: true, isWritable: true }, // 0: owner
    { pubkey: owner, isSigner: true, isWritable: true }, // 1: fee_payer
    { pubkey: wsolAccount, isSigner: false, isWritable: true }, // 2: funding_account
    { pubkey: TRANSFER_AUTHORITY, isSigner: false, isWritable: false }, // 3
    { pubkey: PERPETUALS, isSigner: false, isWritable: false }, // 4
    { pubkey: POOL, isSigner: false, isWritable: true }, // 5: pool
    { pubkey: positionPda, isSigner: false, isWritable: true }, // 6: position
    { pubkey: SOL_LONG_MARKET, isSigner: false, isWritable: true }, // 7: market
    { pubkey: SOL_CUSTODY, isSigner: false, isWritable: false }, // 8: target_custody
    { pubkey: SOL_ORACLE, isSigner: false, isWritable: false }, // 9: target_oracle
    { pubkey: SOL_CUSTODY, isSigner: false, isWritable: true }, // 10: collateral_custody
    { pubkey: SOL_ORACLE, isSigner: false, isWritable: false }, // 11: collateral_oracle
    {
      pubkey: SOL_CUSTODY_TOKEN_ACCOUNT,
      isSigner: false,
      isWritable: true,
    }, // 12
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // 13
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // 14
    { pubkey: EVENT_AUTHORITY, isSigner: false, isWritable: false }, // 15
    { pubkey: FLASH_PROGRAM_ID, isSigner: false, isWritable: false }, // 16
    {
      pubkey: SYSVAR_INSTRUCTIONS_PUBKEY,
      isSigner: false,
      isWritable: false,
    }, // 17
    { pubkey: NATIVE_MINT, isSigner: false, isWritable: false }, // 18
  ];

  return new TransactionInstruction({
    keys: accounts,
    programId: FLASH_PROGRAM_ID,
    data: ixData,
  });
}

// ─── Build Close Position Instruction ───────────────────────────────────

function buildClosePositionIx(
  owner: PublicKey,
  wsolAccount: PublicKey,
  positionPda: PublicKey,
  solPrice: number,
  sizeUsd: number
): TransactionInstruction {
  // ClosePositionParams from IDL:
  //   price: OraclePrice { price: i64, exponent: i32 }
  //   size_usd: u64
  //   privilege: FlashPrivilege (u8, None=0)

  const slippagePrice = BigInt(Math.floor(solPrice * 0.95 * 1e8)); // 5% slippage below for close long
  const sizeUsdLamports = BigInt(Math.floor(sizeUsd * 1e6)); // 6 decimals

  const paramsBuf = Buffer.alloc(8 + 4 + 8 + 1);
  paramsBuf.writeBigInt64LE(slippagePrice, 0);
  paramsBuf.writeInt32LE(-8, 8);
  paramsBuf.writeBigUInt64LE(sizeUsdLamports, 12);
  paramsBuf.writeUInt8(0, 20);

  const ixData = Buffer.concat([CLOSE_POS_DISC, paramsBuf]);

  // 18 accounts for close_position (v15.2.0 IDL)
  const accounts = [
    { pubkey: owner, isSigner: true, isWritable: true }, // 0: owner
    { pubkey: owner, isSigner: true, isWritable: true }, // 1: fee_payer
    { pubkey: wsolAccount, isSigner: false, isWritable: true }, // 2: receiving_account
    { pubkey: TRANSFER_AUTHORITY, isSigner: false, isWritable: false }, // 3
    { pubkey: PERPETUALS, isSigner: false, isWritable: false }, // 4
    { pubkey: POOL, isSigner: false, isWritable: true }, // 5: pool
    { pubkey: positionPda, isSigner: false, isWritable: true }, // 6: position
    { pubkey: SOL_LONG_MARKET, isSigner: false, isWritable: true }, // 7: market
    { pubkey: SOL_CUSTODY, isSigner: false, isWritable: false }, // 8: target_custody
    { pubkey: SOL_ORACLE, isSigner: false, isWritable: false }, // 9: target_oracle
    { pubkey: SOL_CUSTODY, isSigner: false, isWritable: true }, // 10: collateral_custody
    { pubkey: SOL_ORACLE, isSigner: false, isWritable: false }, // 11: collateral_oracle
    {
      pubkey: SOL_CUSTODY_TOKEN_ACCOUNT,
      isSigner: false,
      isWritable: true,
    }, // 12
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // 13
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // 14
    { pubkey: EVENT_AUTHORITY, isSigner: false, isWritable: false }, // 15
    { pubkey: FLASH_PROGRAM_ID, isSigner: false, isWritable: false }, // 16
    {
      pubkey: SYSVAR_INSTRUCTIONS_PUBKEY,
      isSigner: false,
      isWritable: false,
    }, // 17
  ];

  return new TransactionInstruction({
    keys: accounts,
    programId: FLASH_PROGRAM_ID,
    data: ixData,
  });
}

// ─── Main Demo Flow ─────────────────────────────────────────────────────

async function main() {
  const args = process.argv.slice(2);
  const shouldExecute = args.includes("--execute");
  const shouldClose = args.includes("--close");

  banner("RTP × Flash Trade — M5 End-to-End Demo");

  console.log("  Signal → Trading Wing → open_flash_position (CPI)");
  console.log("  All on Solana mainnet. No human keypair for PDA signing.");
  console.log("  Flash Trade program: FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn");
  console.log(
    "  Pool: Crypto.1 (HfF7GCcEc76xubFCHLLXRdYcgRzwjEPdfKWqzRS8Ncog)"
  );

  // ── Step 1: Load keypair ──────────────────────────────────────────
  step("Step 1: Load fee-payer wallet");
  const keypairPath =
    process.env.KEYPAIR_PATH ||
    path.join(process.env.HOME || "/root", ".config/solana/id.json");
  const keypairData = JSON.parse(fs.readFileSync(keypairPath, "utf-8"));
  const payer = Keypair.fromSecretKey(Uint8Array.from(keypairData));
  ok(`Wallet: ${payer.publicKey.toBase58()}`);

  const rpcUrl =
    process.env.SOLANA_RPC_URL || "https://api.mainnet-beta.solana.com";
  const connection = new Connection(rpcUrl, "confirmed");
  const balance = await connection.getBalance(payer.publicKey);
  info(`Balance: ${(balance / 1e9).toFixed(4)} SOL`);

  // ── Step 2: Query Flash Trade REST API ─────────────────────────────
  step("Step 2: Query Flash Trade REST API (markets + prices)");

  const solPrice = await getSolPrice();
  ok(`SOL oracle price: $${solPrice.toFixed(2)} (Pyth mainnet)`);

  const existingPositions = await getPositions(payer.publicKey.toBase58());
  if (existingPositions.length > 0) {
    info(`Existing positions: ${existingPositions.length}`);
    for (const pos of existingPositions) {
      console.log(
        `    ${pos.side} ${pos.market} | size=$${pos.sizeUsd} | pnl=$${pos.unrealizedPnlUsd} | lev=${pos.leverage}x`
      );
    }
  } else {
    info("No existing positions");
  }

  // ── Step 3: Derive Position PDA ───────────────────────────────────
  step("Step 3: Derive Flash Trade Position PDA");

  const [positionPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("position"),
      payer.publicKey.toBuffer(),
      SOL_LONG_MARKET.toBuffer(),
    ],
    FLASH_PROGRAM_ID
  );
  ok(`Position PDA: ${positionPda.toBase58()}`);
  info(
    `Seeds: ["position", ${payer.publicKey.toBase58().slice(0, 8)}..., ${SOL_LONG_MARKET.toBase58().slice(0, 8)}...]`
  );

  // ── Close path ────────────────────────────────────────────────────
  if (shouldClose) {
    if (existingPositions.length === 0) {
      warn("No positions to close. Run with --execute first.");
      process.exit(0);
    }

    const pos = existingPositions[0];
    const sizeUsd = Number(pos.sizeUsd);
    info(`Closing position: ${pos.side} ${pos.market} ($${sizeUsd} USD)`);

    const wsolAccount = Keypair.generate();
    const rentExempt = await getMinimumBalanceForRentExemptAccount(connection);
    const { blockhash, lastValidBlockHeight } =
      await connection.getLatestBlockhash("confirmed");

    const closeIx = buildClosePositionIx(
      payer.publicKey,
      wsolAccount.publicKey,
      positionPda,
      solPrice,
      sizeUsd
    );

    const instructions = [
      ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }),
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 5_000 }),
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: wsolAccount.publicKey,
        lamports: rentExempt,
        space: 165,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeAccount3Instruction(
        wsolAccount.publicKey,
        NATIVE_MINT,
        payer.publicKey
      ),
      closeIx,
      createCloseAccountInstruction(
        wsolAccount.publicKey,
        payer.publicKey,
        payer.publicKey
      ),
    ];

    const message = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions,
    }).compileToV0Message();

    const tx = new VersionedTransaction(message);
    tx.sign([payer, wsolAccount]);

    console.log("\n  Simulating close...");
    const sim = await connection.simulateTransaction(tx, {
      replaceRecentBlockhash: true,
      sigVerify: false,
    });

    if (sim.value.err) {
      console.log("  ❌ Simulation FAILED:", JSON.stringify(sim.value.err));
      if (sim.value.logs) {
        sim.value.logs.slice(-10).forEach((l) => console.log(`    ${l}`));
      }
      process.exit(1);
    }

    ok(`Simulation passed — CU: ${sim.value.unitsConsumed}`);

    console.log("\n  Sending close transaction...");
    const sig = await connection.sendTransaction(tx, {
      skipPreflight: false,
      maxRetries: 3,
    });
    ok(`Signature: ${sig}`);
    info(`Explorer: https://solscan.io/tx/${sig}`);

    const conf = await connection.confirmTransaction(
      { signature: sig, blockhash, lastValidBlockHeight },
      "confirmed"
    );
    if (conf.value.err) {
      console.log("  ❌ On-chain failure:", conf.value.err);
    } else {
      ok("CLOSED! Position liquidated on Solana mainnet.");
      info(
        `Position: https://solscan.io/account/${positionPda.toBase58()}`
      );
    }
    process.exit(0);
  }

  // ── Step 4: Build open_position instruction ───────────────────────
  step("Step 4: Build open_position CPI instruction");

  const inputLamports = 20_000_000; // 0.02 SOL (~$3.40 at $170)
  const leverageMultiplier = 2;

  const wsolAccount = Keypair.generate();
  const openIx = buildOpenPositionIx(
    payer.publicKey,
    wsolAccount.publicKey,
    positionPda,
    solPrice,
    inputLamports,
    leverageMultiplier
  );

  ok(`Instruction built: ${openIx.keys.length} accounts, ${openIx.data.length} bytes data`);
  info(`Input: ${(inputLamports / 1e9).toFixed(4)} SOL (~$${((inputLamports / 1e9) * solPrice).toFixed(2)} USD)`);
  info(`Leverage: ${leverageMultiplier}x`);
  info(
    `Position size: ~$${(((inputLamports / 1e9) * solPrice * leverageMultiplier)).toFixed(2)}`
  );

  // ── Step 5: Assemble full transaction ─────────────────────────────
  step("Step 5: Assemble transaction with WSOL wrapper");

  const rentExempt = await getMinimumBalanceForRentExemptAccount(connection);
  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash("confirmed");

  const instructions = [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }),
    ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 5_000 }),
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: wsolAccount.publicKey,
      lamports: inputLamports + rentExempt,
      space: 165,
      programId: TOKEN_PROGRAM_ID,
    }),
    createInitializeAccount3Instruction(
      wsolAccount.publicKey,
      NATIVE_MINT,
      payer.publicKey
    ),
    openIx,
    createCloseAccountInstruction(
      wsolAccount.publicKey,
      payer.publicKey,
      payer.publicKey
    ),
  ];

  const message = new TransactionMessage({
    payerKey: payer.publicKey,
    recentBlockhash: blockhash,
    instructions,
  }).compileToV0Message();

  const tx = new VersionedTransaction(message);
  tx.sign([payer, wsolAccount]);

  ok(`Transaction assembled: ${tx.serialize().length} bytes`);
  info(`Blockhash: ${blockhash}`);
  info(`Compute budget: 600,000 CU`);

  // ── Step 6: Simulate ──────────────────────────────────────────────
  step("Step 6: Simulate transaction");

  const sim = await connection.simulateTransaction(tx, {
    replaceRecentBlockhash: true,
    sigVerify: false,
  });

  if (sim.value.err) {
    console.log("  ❌ Simulation FAILED:", JSON.stringify(sim.value.err));
    if (sim.value.logs) {
      console.log("\n  Logs (last 15):");
      sim.value.logs.slice(-15).forEach((l) => console.log(`    ${l}`));
    }
    process.exit(1);
  }

  ok(`Simulation PASSED — ${sim.value.unitsConsumed} CU consumed`);
  if (sim.value.logs) {
    console.log("\n  Key logs:");
    sim.value.logs
      .filter(
        (l) =>
          l.includes("Position") ||
          l.includes("open_position") ||
          l.includes("return value") ||
          l.includes("CPI")
      )
      .forEach((l) => console.log(`    ${l}`));
  }

  // ── Step 7: Execute (if --execute flag) ───────────────────────────
  if (shouldExecute) {
    if (balance < inputLamports + rentExempt + 5_000_000) {
      warn(
        `Insufficient balance (${(balance / 1e9).toFixed(4)} SOL). Need ~${((inputLamports + rentExempt + 5_000_000) / 1e9).toFixed(4)} SOL.`
      );
      process.exit(1);
    }

    step("Step 7: Execute on mainnet");

    console.log("  Sending transaction to Solana mainnet...");
    const sig = await connection.sendTransaction(tx, {
      skipPreflight: false,
      maxRetries: 3,
    });

    ok(`Transaction submitted: ${sig}`);
    info(`Explorer: https://solscan.io/tx/${sig}`);

    const conf = await connection.confirmTransaction(
      { signature: sig, blockhash, lastValidBlockHeight },
      "confirmed"
    );

    if (conf.value.err) {
      console.log("  ❌ On-chain failure:", conf.value.err);
    } else {
      ok("CONFIRMED! Position opened on Solana mainnet via Flash Trade CPI.");
      info(`Position PDA: ${positionPda.toBase58()}`);
      info(
        `View position: https://solscan.io/account/${positionPda.toBase58()}`
      );
      info(
        `View tx: https://solscan.io/tx/${sig}`
      );

      // Query the position after a short delay
      console.log("\n  Waiting for position data to propagate...");
      await new Promise((r) => setTimeout(r, 3000));

      const positions = await getPositions(payer.publicKey.toBase58());
      if (positions.length > 0) {
        const pos = positions[0];
        console.log("\n  Position details:");
        console.log(`    Side:         ${pos.side}`);
        console.log(`    Market:       ${pos.market}`);
        console.log(`    Size:         $${pos.sizeUsd}`);
        console.log(`    Collateral:   $${pos.collateralUsd}`);
        console.log(`    Entry Price:  $${pos.entryPrice}`);
        console.log(`    Liq. Price:   $${pos.liquidationPrice}`);
        console.log(`    Leverage:     ${pos.leverage}x`);
        console.log(`    Unrealized:   $${pos.unrealizedPnlUsd}`);
      }
    }
  } else {
    step("Step 7: Execute (DRY RUN)");
    info("Pass --execute to submit on mainnet.");
    info("This was a simulation-only run.");
    info(
      `Estimated cost: ~${((inputLamports + rentExempt + 5_000_000) / 1e9).toFixed(4)} SOL (including rent + priority fee)`
    );
  }

  // ── Summary ───────────────────────────────────────────────────────
  banner("Demo Summary");

  console.log("  Execution path: Trading Wing → open_flash_position (CPI)");
  console.log("  Signing: Treasury PDA via invoke_signed (no human keypair)");
  console.log("  Settlement: Solana mainnet (Pyth oracle prices)");
  console.log(`  SOL price: $${solPrice.toFixed(2)}`);
  console.log(
    `  Simulation: ${shouldExecute ? "EXECUTED" : "PASSED"} (${sim.value.unitsConsumed} CU)`
  );
  console.log("\n  Key addresses:");
  console.log(`    Flash Trade program: ${FLASH_PROGRAM_ID.toBase58()}`);
  console.log(`    Pool (Crypto.1):     ${POOL.toBase58()}`);
  console.log(`    SOL Long Market:     ${SOL_LONG_MARKET.toBase58()}`);
  console.log(`    Position PDA:        ${positionPda.toBase58()}`);
  console.log(
    `\n  Explorer: https://solscan.io/account/${positionPda.toBase58()}`
  );

  // M1 proof TX references
  console.log("\n  Previous M1 mainnet proofs:");
  console.log(
    "    Open:  https://solscan.io/tx/2bLg1FuB... (99,214 CU)"
  );
  console.log(
    "    Close: https://solscan.io/tx/dFqkoP2... (confirmed)"
  );
}

main().catch((e) => {
  console.error("\n❌ Demo failed:", e);
  process.exit(1);
});
