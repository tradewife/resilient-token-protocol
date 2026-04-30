/**
 * RTP Treasury — Devnet Beta Adopter Lifecycle Test
 *
 * Comprehensive integration test against the deployed program on devnet.
 * Tests every beta code path with real on-chain transactions.
 *
 * Usage:
 *   npx tsx scripts/devnet-beta-test.ts
 *
 * Tests:
 *   1. Create Token-2022 mint + treasury + adopter (permanent)
 *   2. Create Token-2022 mint + treasury + beta adopter (with expiry)
 *   3. register_adopter: verify beta_expires_at=0, beta_ended=false
 *   4. register_adopter_beta: verify beta_expires_at set correctly
 *   5. record_fee_deposit: increments fees on beta adopter
 *   6. register_strategy: Live strategy for hydration
 *   7. hydrate_swarm: succeeds for permanent adopter
 *   8. hydrate_swarm: succeeds for active beta adopter
 *   9. end_beta: authority sets beta_ended=true
 *  10. hydrate_swarm: FAILS for ended beta (BetaExpired)
 *  11. register_adopter_beta: FAILS with past expiry (BetaExpired)
 *  12. end_beta: FAILS for unauthorized caller (UnauthorizedBetaOp)
 *  13. Cross-check: permanent adopter still hydrates after beta tests
 */

import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  Keypair,
  SystemProgram,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  ExtensionType,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createInitializeMint2Instruction,
  mintTo,
  getAccount,
  createInitializeTransferFeeConfigInstruction,
  getMintLen,
  createTransferCheckedWithFeeInstruction,
  createHarvestWithheldTokensToMintInstruction,
} from "@solana/spl-token";
import * as fs from "fs";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const SEED_TREASURY = Buffer.from("treasury");
const SEED_VAULT = Buffer.from("vault");
const SEED_SWARM = Buffer.from("swarm-hydration");
const SEED_STRATEGY = Buffer.from("strategy");
const SEED_ADOPTER = Buffer.from("adopter");

const FEE_BPS = 1000;
const MAX_FEE = BigInt(1_000_000_000);
const DECIMALS = 6;
const MIN_RUNWAY = 10_000_000;
const MINT_SPACE = getMintLen([ExtensionType.TransferFeeConfig]);

let passed = 0;
let failed = 0;

function ok(msg: string) { console.log(`  \x1b[32mPASS\x1b[0m ${msg}`); passed++; }
function fail(msg: string, err?: string) { console.log(`  \x1b[31mFAIL\x1b[0m ${msg}${err ? ` — ${err}` : ""}`); failed++; }
function info(msg: string) { console.log(`  \x1b[36m>\x1b[0m ${msg}`); }
function section(msg: string) { console.log(`\n\x1b[1m${msg}\x1b[0m`); }

function deriveTreasuryPDA(mint: PublicKey, pid: PublicKey) {
  return PublicKey.findProgramAddressSync([SEED_TREASURY, mint.toBuffer()], pid);
}
function deriveVaultPDA(mint: PublicKey, pid: PublicKey) {
  return PublicKey.findProgramAddressSync([SEED_TREASURY, mint.toBuffer(), SEED_VAULT], pid);
}
function deriveSwarmVaultPDA(mint: PublicKey, pid: PublicKey) {
  return PublicKey.findProgramAddressSync([SEED_SWARM, mint.toBuffer()], pid);
}
function deriveAdopterPDA(tokenMint: PublicKey, pid: PublicKey) {
  return PublicKey.findProgramAddressSync([SEED_ADOPTER, tokenMint.toBuffer()], pid);
}
function deriveStrategyPDA(treasury: PublicKey, strategyId: string, pid: PublicKey) {
  return PublicKey.findProgramAddressSync([SEED_STRATEGY, treasury.toBuffer(), Buffer.from(strategyId)], pid);
}

// ---------------------------------------------------------------------------
// Mint + Treasury setup
// ---------------------------------------------------------------------------

interface SetupResult {
  mint: PublicKey;
  mintAuth: Keypair;
  treasuryPDA: PublicKey;
  vaultPDA: PublicKey;
  swarmVaultPDA: PublicKey;
  adopterPDA: PublicKey;
  holdersWallet: Keypair;
  devWallet: Keypair;
  ecosystemWallet: Keypair;
  feeRecipientKp: Keypair;
  feeRecipientATA: PublicKey;
}

async function setupMintTreasury(
  connection: anchor.web3.Connection,
  payer: Keypair,
  program: any,
  isBeta: boolean,
  betaExpiresAt?: number,
): Promise<SetupResult> {
  const mintAuth = Keypair.generate();
  const mintKp = Keypair.generate();
  const mint = mintKp.publicKey;
  const pid = program.programId;

  const [treasuryPDA] = deriveTreasuryPDA(mint, pid);
  const [vaultPDA] = deriveVaultPDA(mint, pid);
  const [swarmVaultPDA] = deriveSwarmVaultPDA(mint, pid);
  const [adopterPDA] = deriveAdopterPDA(mint, pid);

  // Create Token-2022 mint with TransferFeeConfig
  const space = MINT_SPACE;
  const lamports = await connection.getMinimumBalanceForRentExemption(space);
  const tx1 = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey, newAccountPubkey: mint, space, lamports,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
    createInitializeTransferFeeConfigInstruction(
      mint, mintAuth.publicKey, treasuryPDA, FEE_BPS, MAX_FEE, TOKEN_2022_PROGRAM_ID,
    ),
  );
  await sendAndConfirmTransaction(connection, tx1, [payer, mintKp]);
  const tx2 = new Transaction().add(
    createInitializeMint2Instruction(
      mint, DECIMALS, mintAuth.publicKey, null, TOKEN_2022_PROGRAM_ID,
    ),
  );
  await sendAndConfirmTransaction(connection, tx2, [payer]);

  // Create wallets
  const holdersWallet = Keypair.generate();
  const devWallet = Keypair.generate();
  const ecosystemWallet = Keypair.generate();

  // Initialize treasury
  await program.methods
    .initialize(new BN(MIN_RUNWAY))
    .accounts({
      mint, treasury: treasuryPDA, treasuryVault: vaultPDA,
      holdersWallet: holdersWallet.publicKey,
      projectDevWallet: devWallet.publicKey,
      ecosystemWallet: ecosystemWallet.publicKey,
      authority: payer.publicKey,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  // Create swarm vault
  await program.methods
    .createSwarmVault()
    .accounts({
      mint, treasury: treasuryPDA, swarmVault: swarmVaultPDA,
      authority: payer.publicKey,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  // Register adopter
  if (isBeta) {
    await program.methods
      .registerAdopterBeta(mint, new BN(betaExpiresAt!))
      .accounts({
        adopterRecord: adopterPDA, treasury: treasuryPDA,
        authority: payer.publicKey, systemProgram: SystemProgram.programId,
      })
      .rpc();
  } else {
    await program.methods
      .registerAdopter(mint)
      .accounts({
        adopterRecord: adopterPDA, treasury: treasuryPDA,
        authority: payer.publicKey, systemProgram: SystemProgram.programId,
      })
      .rpc();
  }

  // Create ATAs + mint tokens for fee generation
  const feeRecipientKp = Keypair.generate();
  const sourceATA = getAssociatedTokenAddressSync(mint, payer.publicKey, false, TOKEN_2022_PROGRAM_ID);
  const feeRecipientATA = getAssociatedTokenAddressSync(mint, feeRecipientKp.publicKey, false, TOKEN_2022_PROGRAM_ID);
  const tx3 = new Transaction().add(
    createAssociatedTokenAccountInstruction(payer.publicKey, sourceATA, payer.publicKey, mint, TOKEN_2022_PROGRAM_ID),
    createAssociatedTokenAccountInstruction(payer.publicKey, feeRecipientATA, feeRecipientKp.publicKey, mint, TOKEN_2022_PROGRAM_ID),
  );
  await sendAndConfirmTransaction(connection, tx3, [payer]);
  await mintTo(connection, payer, mint, sourceATA, mintAuth, BigInt(1_000_000_000_000), [], undefined, TOKEN_2022_PROGRAM_ID);

  return { mint, mintAuth, treasuryPDA, vaultPDA, swarmVaultPDA, adopterPDA, holdersWallet, devWallet, ecosystemWallet, feeRecipientKp, feeRecipientATA };
}

async function generateFees(
  connection: anchor.web3.Connection,
  payer: Keypair,
  program: any,
  setup: SetupResult,
  count: number = 5,
) {
  const sourceATA = getAssociatedTokenAddressSync(setup.mint, payer.publicKey, false, TOKEN_2022_PROGRAM_ID);

  for (let i = 0; i < count; i++) {
    const amount = BigInt(10_000_000_000);
    const fee = BigInt(Math.floor(Number(amount) * FEE_BPS / 10000));
    const ix = createTransferCheckedWithFeeInstruction(
      sourceATA, setup.mint, setup.feeRecipientATA, payer.publicKey,
      amount, DECIMALS, fee, [], TOKEN_2022_PROGRAM_ID,
    );
    await sendAndConfirmTransaction(connection, new Transaction().add(ix), [payer]);
  }

  // Harvest + withdraw
  const harvestIx = createHarvestWithheldTokensToMintInstruction(setup.mint, [setup.feeRecipientATA], TOKEN_2022_PROGRAM_ID);
  await sendAndConfirmTransaction(connection, new Transaction().add(harvestIx), [payer]);

  await program.methods
    .withdrawFees()
    .accounts({
      mint: setup.mint, treasury: setup.treasuryPDA, treasuryVault: setup.vaultPDA,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();
}

// ---------------------------------------------------------------------------
// Main test runner
// ---------------------------------------------------------------------------

async function main() {
  console.log("\n\x1b[1m\x1b[35mRTP Beta Adopter — Devnet Integration Test\x1b[0m\n");

  const connection = new anchor.web3.Connection("https://api.devnet.solana.com", "confirmed");
  info(`Cluster: ${connection.rpcEndpoint}`);

  const payer = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(process.env.HOME + "/.config/solana/id.json", "utf-8"))),
  );
  info(`Payer: ${payer.publicKey.toBase58()}`);

  const balance = await connection.getBalance(payer.publicKey);
  info(`SOL: ${(balance / LAMPORTS_PER_SOL).toFixed(2)}`);

  // Load program
  const rawIdl = require("../target/idl/rtp_treasury.json");
  const wallet = new anchor.Wallet(payer);
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
  const program = new anchor.Program(rawIdl, provider);
  info(`Program: ${program.programId.toBase58()}`);

  // Verify program is live on devnet
  const programInfo = await connection.getAccountInfo(program.programId);
  if (!programInfo) { fail("Program not found on devnet"); process.exit(1); }
  ok(`Program live on devnet (data: ${programInfo.data.length} bytes)`);

  // ========================================================================
  // SETUP: Permanent adopter
  // ========================================================================

  section("SETUP: Permanent adopter");
  const perm = await setupMintTreasury(connection, payer, program, false);
  ok("Permanent adopter: mint + treasury + swarm vault + adopter created");

  // ========================================================================
  // SETUP: Beta adopter (expires 7 days from now)
  // ========================================================================

  section("SETUP: Beta adopter");
  const betaExpiresAt = Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60; // 7 days
  const beta = await setupMintTreasury(connection, payer, program, true, betaExpiresAt);
  ok("Beta adopter: mint + treasury + swarm vault + beta adopter created");

  // ========================================================================
  // TEST 1: Verify permanent adopter fields
  // ========================================================================

  section("TEST 1: Permanent adopter — beta_expires_at=0, beta_ended=false");
  {
    const record = await program.account.adopterRecord.fetch(perm.adopterPDA);
    if (record.betaExpiresAt.toNumber() === 0) ok("beta_expires_at = 0 (permanent)");
    else fail(`beta_expires_at = ${record.betaExpiresAt}, expected 0`);
    if (record.betaEnded === false) ok("beta_ended = false");
    else fail("beta_ended should be false");
  }

  // ========================================================================
  // TEST 2: Verify beta adopter fields
  // ========================================================================

  section("TEST 2: Beta adopter — beta_expires_at set, beta_ended=false");
  {
    const record = await program.account.adopterRecord.fetch(beta.adopterPDA);
    if (record.betaExpiresAt.toNumber() === betaExpiresAt) ok(`beta_expires_at = ${betaExpiresAt}`);
    else fail(`beta_expires_at = ${record.betaExpiresAt}, expected ${betaExpiresAt}`);
    if (record.betaEnded === false) ok("beta_ended = false");
    else fail("beta_ended should be false");
  }

  // ========================================================================
  // TEST 3: record_fee_deposit on beta adopter
  // ========================================================================

  section("TEST 3: Fee deposit on beta adopter");
  {
    const depositAmt = new BN(500_000_000); // 0.5 SOL
    await program.methods
      .recordFeeDeposit(depositAmt)
      .accounts({
        adopterRecord: beta.adopterPDA, treasury: beta.treasuryPDA,
        authority: payer.publicKey,
      })
      .rpc();
    const record = await program.account.adopterRecord.fetch(beta.adopterPDA);
    if (record.feesContributedLamports.toNumber() === 500_000_000) ok("fees_contributed = 500M lamports");
    else fail(`fees_contributed = ${record.feesContributedLamports}, expected 500M`);
    if (record.depositCount.toNumber() === 1) ok("deposit_count = 1");
    else fail("deposit_count should be 1");
  }

  // ========================================================================
  // TEST 4: Register strategy on both treasuries
  // ========================================================================

  section("TEST 4: Register Live strategies");
  {
    const permStratId = "PERM_STRAT";
    const [permStratPDA] = deriveStrategyPDA(perm.treasuryPDA, permStratId, program.programId);
    await program.methods
      .registerStrategy(permStratId, 396)
      .accounts({
        treasury: perm.treasuryPDA, strategyRecord: permStratPDA,
        authority: payer.publicKey, systemProgram: SystemProgram.programId,
      })
      .rpc();
    ok("Permanent: strategy registered (Live)");

    const betaStratId = "BETA_STRAT";
    const [betaStratPDA] = deriveStrategyPDA(beta.treasuryPDA, betaStratId, program.programId);
    await program.methods
      .registerStrategy(betaStratId, 396)
      .accounts({
        treasury: beta.treasuryPDA, strategyRecord: betaStratPDA,
        authority: payer.publicKey, systemProgram: SystemProgram.programId,
      })
      .rpc();
    ok("Beta: strategy registered (Live)");
  }

  // ========================================================================
  // TEST 5: hydrate_swarm succeeds for permanent adopter
  // ========================================================================

  section("TEST 5: hydrate_swarm — permanent adopter succeeds");
  {
    await generateFees(connection, payer, program, perm, 3);
    const vault = await getAccount(connection, perm.vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID);
    const excess = Number(vault.amount) - MIN_RUNWAY;

    if (excess > 1_000_000) {
      const hydrateAmt = new BN(Math.min(excess - 500_000, 5_000_000));
      const [stratPDA] = deriveStrategyPDA(perm.treasuryPDA, "PERM_STRAT", program.programId);

      await program.methods
        .hydrateSwarm(hydrateAmt)
        .accounts({
          mint: perm.mint, treasury: perm.treasuryPDA, treasuryVault: perm.vaultPDA,
          swarmVault: perm.swarmVaultPDA, strategyRecord: stratPDA,
          adopterRecord: perm.adopterPDA,
          authority: payer.publicKey, tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      ok("Hydration succeeded for permanent adopter");
    } else {
      info(`Skipped — vault excess too low (${excess})`);
    }
  }

  // ========================================================================
  // TEST 6: hydrate_swarm succeeds for active beta adopter
  // ========================================================================

  section("TEST 6: hydrate_swarm — active beta adopter succeeds");
  {
    await generateFees(connection, payer, program, beta, 3);
    const vault = await getAccount(connection, beta.vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID);
    const excess = Number(vault.amount) - MIN_RUNWAY;

    if (excess > 1_000_000) {
      const hydrateAmt = new BN(Math.min(excess - 500_000, 5_000_000));
      const [stratPDA] = deriveStrategyPDA(beta.treasuryPDA, "BETA_STRAT", program.programId);

      await program.methods
        .hydrateSwarm(hydrateAmt)
        .accounts({
          mint: beta.mint, treasury: beta.treasuryPDA, treasuryVault: beta.vaultPDA,
          swarmVault: beta.swarmVaultPDA, strategyRecord: stratPDA,
          adopterRecord: beta.adopterPDA,
          authority: payer.publicKey, tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      ok("Hydration succeeded for active beta adopter");
    } else {
      info(`Skipped — vault excess too low (${excess})`);
    }
  }

  // ========================================================================
  // TEST 7: end_beta — authority sets beta_ended
  // ========================================================================

  section("TEST 7: end_beta — authority sunset");
  {
    await program.methods
      .endBeta()
      .accounts({
        adopterRecord: beta.adopterPDA, treasury: beta.treasuryPDA,
        authority: payer.publicKey,
      })
      .rpc();
    const record = await program.account.adopterRecord.fetch(beta.adopterPDA);
    if (record.betaEnded === true) ok("beta_ended = true after end_beta");
    else fail("beta_ended should be true");
  }

  // ========================================================================
  // TEST 8: hydrate_swarm FAILS for ended beta
  // ========================================================================

  section("TEST 8: hydrate_swarm — ended beta FAILS");
  {
    const [stratPDA] = deriveStrategyPDA(beta.treasuryPDA, "BETA_STRAT", program.programId);
    try {
      await program.methods
        .hydrateSwarm(new BN(1_000_000))
        .accounts({
          mint: beta.mint, treasury: beta.treasuryPDA, treasuryVault: beta.vaultPDA,
          swarmVault: beta.swarmVaultPDA, strategyRecord: stratPDA,
          adopterRecord: beta.adopterPDA,
          authority: payer.publicKey, tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.systemId,
        })
        .rpc();
      fail("Should have rejected ended beta");
    } catch (err: any) {
      if (err.toString().includes("BetaExpired")) ok("Rejected with BetaExpired");
      else fail("Wrong error", err.toString().substring(0, 200));
    }
  }

  // ========================================================================
  // TEST 9: register_adopter_beta FAILS with past expiry
  // ========================================================================

  section("TEST 9: register_adopter_beta — past expiry FAILS");
  {
    const fakeMint = Keypair.generate().publicKey;
    const [fakeAdopter] = deriveAdopterPDA(fakeMint, program.programId);
    const pastExpiry = Math.floor(Date.now() / 1000) - 3600;

    try {
      await program.methods
        .registerAdopterBeta(fakeMint, new BN(pastExpiry))
        .accounts({
          adopterRecord: fakeAdopter, treasury: beta.treasuryPDA,
          authority: payer.publicKey, systemProgram: SystemProgram.programId,
        })
        .rpc();
      fail("Should have rejected past expiry");
    } catch (err: any) {
      if (err.toString().includes("BetaExpired")) ok("Rejected past expiry with BetaExpired");
      else fail("Wrong error", err.toString().substring(0, 200));
    }
  }

  // ========================================================================
  // TEST 10: end_beta FAILS for unauthorized caller
  // ========================================================================

  section("TEST 10: end_beta — unauthorized caller FAILS");
  {
    // Create a fresh beta adopter to test unauthorized end_beta
    const freshSetup = await setupMintTreasury(connection, payer, program, true, Math.floor(Date.now() / 1000) + 86400);
    const attacker = Keypair.generate();

    try {
      await program.methods
        .endBeta()
        .accounts({
          adopterRecord: freshSetup.adopterPDA, treasury: freshSetup.treasuryPDA,
          authority: attacker.publicKey,
        })
        .signers([attacker])
        .rpc();
      fail("Should have rejected unauthorized end_beta");
    } catch (err: any) {
      if (err.toString().includes("UnauthorizedBetaOp")) ok("Rejected unauthorized with UnauthorizedBetaOp");
      else fail("Wrong error", err.toString().substring(0, 200));
    }
  }

  // ========================================================================
  // TEST 11: Permanent adopter still works after beta tests
  // ========================================================================

  section("TEST 11: Permanent adopter still hydrates after all beta tests");
  {
    await generateFees(connection, payer, program, perm, 3);
    const vault = await getAccount(connection, perm.vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID);
    const excess = Number(vault.amount) - MIN_RUNWAY;

    if (excess > 1_000_000) {
      const hydrateAmt = new BN(Math.min(excess - 500_000, 3_000_000));
      const [stratPDA] = deriveStrategyPDA(perm.treasuryPDA, "PERM_STRAT", program.programId);

      await program.methods
        .hydrateSwarm(hydrateAmt)
        .accounts({
          mint: perm.mint, treasury: perm.treasuryPDA, treasuryVault: perm.vaultPDA,
          swarmVault: perm.swarmVaultPDA, strategyRecord: stratPDA,
          adopterRecord: perm.adopterPDA,
          authority: payer.publicKey, tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      ok("Permanent adopter hydration still works — no cross-contamination");
    } else {
      info(`Skipped — vault excess too low (${excess})`);
    }
  }

  // ========================================================================
  // Summary
  // ========================================================================

  console.log(`\n\x1b[1m${"=".repeat(50)}\x1b[0m`);
  console.log(`\x1b[1mResults: ${passed} passed, ${failed} failed\x1b[0m`);
  if (failed > 0) {
    console.log("\x1b[31mFAILURES DETECTED — fix before mainnet deploy\x1b[0m");
    process.exit(1);
  } else {
    console.log("\x1b[32mAll devnet integration tests passed — ready for mainnet\x1b[0m");
  }
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
