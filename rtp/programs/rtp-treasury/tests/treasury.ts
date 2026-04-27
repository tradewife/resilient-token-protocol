/**
 * RTP Treasury — Anchor Integration Tests
 *
 * Tests every instruction against audit findings (C-1, C-2/C-3, H-1–H-5,
 * M-1–M-5). Runs via `anchor test --skip-build --validator legacy --provider.cluster localnet`.
 */

import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  Keypair,
  SystemProgram,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  ExtensionType,
  MINT_SIZE,
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
import { assert } from "chai";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SEED_TREASURY = Buffer.from("treasury");
const SEED_VAULT = Buffer.from("vault");
const SEED_SWARM = Buffer.from("swarm-hydration");
const SEED_STRATEGY = Buffer.from("strategy");

const PROGRAM_ID = new PublicKey(
  "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"
);

const FEE_BASIS_POINTS = 1000; // 10%
const MAX_FEE = BigInt(1_000_000_000);
const MINT_DECIMALS = 6;
const MIN_RUNWAY = 10_000_000; // 10 tokens
const DEFAULT_MIN_REDISTRIBUTE = 1_000_000; // 1 token
const MINT_SPACE_WITH_FEE = getMintLen([ExtensionType.TransferFeeConfig]);

// ---------------------------------------------------------------------------
// PDA helpers
// ---------------------------------------------------------------------------

function deriveTreasuryPDA(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_TREASURY, mint.toBuffer()],
    PROGRAM_ID
  );
}

function deriveVaultPDA(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_TREASURY, mint.toBuffer(), SEED_VAULT],
    PROGRAM_ID
  );
}

function deriveSwarmVaultPDA(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_SWARM, mint.toBuffer()],
    PROGRAM_ID
  );
}

function deriveStrategyPDA(
  treasury: PublicKey,
  strategyId: string
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_STRATEGY, treasury.toBuffer(), Buffer.from(strategyId)],
    PROGRAM_ID
  );
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("rtp-treasury", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const payer = (provider.wallet as anchor.Wallet).payer;

  let program: any;
  let idl: any;

  // Shared state (set by setupFresh)
  let mint: PublicKey;
  let mintAuthKp: Keypair;
  let treasuryPDA: PublicKey;
  let vaultPDA: PublicKey;
  let swarmVaultPDA: PublicKey;

  let holdersWallet: Keypair;
  let devWallet: Keypair;
  let ecosystemWallet: Keypair;
  let sourceWallet: Keypair;
  let feeRecipientWallet: Keypair;

  let holdersATA: PublicKey;
  let devATA: PublicKey;
  let ecosystemATA: PublicKey;
  let sourceATA: PublicKey;
  let feeRecipientATA: PublicKey;
  let treasuryAdopterPDA: PublicKey;

  // -----------------------------------------------------------------------
  // Helpers (must be inside describe to access provider/payer/program)
  // -----------------------------------------------------------------------

  async function createMintWithFee(
    mintAuth: Keypair,
    mintKp: Keypair,
    withdrawWithheldAuthority: PublicKey,
    decimals: number = MINT_DECIMALS,
    feeBps: number = FEE_BASIS_POINTS,
  ): Promise<void> {
    const mint = mintKp.publicKey;
    const space = MINT_SPACE_WITH_FEE;
    const lamports = await provider.connection.getMinimumBalanceForRentExemption(space);

    const tx1 = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: mint,
        space,
        lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeTransferFeeConfigInstruction(
        mint, mintAuth.publicKey, withdrawWithheldAuthority,
        feeBps, MAX_FEE, TOKEN_2022_PROGRAM_ID,
      ),
    );
    await provider.sendAndConfirm(tx1, [payer, mintKp]);

    const tx2 = new Transaction().add(
      createInitializeMint2Instruction(
        mint, decimals, mintAuth.publicKey, null, TOKEN_2022_PROGRAM_ID,
      ),
    );
    await provider.sendAndConfirm(tx2, [payer]);
  }

  async function createVanillaMint(
    mintAuth: Keypair,
    decimals: number = MINT_DECIMALS,
  ): Promise<Keypair> {
    const mintKp = Keypair.generate();
    const lamports = await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE);

    const tx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: mintKp.publicKey,
        space: MINT_SIZE,
        lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeMint2Instruction(
        mintKp.publicKey, decimals, mintAuth.publicKey, null, TOKEN_2022_PROGRAM_ID,
      ),
    );
    await provider.sendAndConfirm(tx, [payer, mintKp]);
    return mintKp;
  }

  async function createATA(
    mint: PublicKey,
    owner: PublicKey,
  ): Promise<PublicKey> {
    const ata = getAssociatedTokenAddressSync(mint, owner, false, TOKEN_2022_PROGRAM_ID);
    const tx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        payer.publicKey, ata, owner, mint, TOKEN_2022_PROGRAM_ID,
      ),
    );
    await provider.sendAndConfirm(tx, []);
    return ata;
  }

  before(async () => {
    idl = require("../target/idl/rtp_treasury.json");
    program = new anchor.Program(idl, provider);

    holdersWallet = Keypair.generate();
    devWallet = Keypair.generate();
    ecosystemWallet = Keypair.generate();
    sourceWallet = Keypair.generate();
    feeRecipientWallet = Keypair.generate();

    // Airdrop SOL for tx fees
    for (const w of [holdersWallet, devWallet, ecosystemWallet, sourceWallet, feeRecipientWallet]) {
      const sig = await provider.connection.requestAirdrop(
        w.publicKey, 5 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");
    }
  });

  /**
   * Full setup: mint with TransferFeeConfig → treasury init → ATAs → swarm vault.
   *
   * Important: the withdraw_withheld_authority on the mint must be set to the
   * Treasury PDA (derived from the mint address). Since PDAs are deterministic,
   * we can generate the mint keypair first, derive the PDA, then pass it to
   * the mint creation function.
   */
  async function setupFresh(): Promise<void> {
    mintAuthKp = Keypair.generate();
    const mintKp = Keypair.generate();
    mint = mintKp.publicKey;

    // Derive PDAs from the mint address BEFORE creating it (deterministic)
    [treasuryPDA] = deriveTreasuryPDA(mint);
    [vaultPDA] = deriveVaultPDA(mint);
    [swarmVaultPDA] = deriveSwarmVaultPDA(mint);

    // Create mint with withdraw_withheld_authority = treasury PDA
    await createMintWithFee(mintAuthKp, mintKp, treasuryPDA);

    // ATAs
    holdersATA = await createATA(mint, holdersWallet.publicKey);
    devATA = await createATA(mint, devWallet.publicKey);
    ecosystemATA = await createATA(mint, ecosystemWallet.publicKey);
    sourceATA = await createATA(mint, sourceWallet.publicKey);
    // Create a dedicated fee recipient ATA (reused across generateAndWithdrawFees calls
    // to avoid the first-transfer-after-ATA-creation NO-OP issue)
    feeRecipientATA = await createATA(mint, feeRecipientWallet.publicKey);

    // Mint 1M tokens to source (payer pays fee, mintAuthKp is authority)
    await mintTo(
      provider.connection, payer, mint, sourceATA, mintAuthKp,
      BigInt(1_000_000_000_000), [], undefined, TOKEN_2022_PROGRAM_ID
    );

    // Initialize treasury
    await program.methods
      .initialize(new BN(MIN_RUNWAY))
      .accounts({
        mint,
        treasury: treasuryPDA,
        treasuryVault: vaultPDA,
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
        mint,
        treasury: treasuryPDA,
        swarmVault: swarmVaultPDA,
        authority: payer.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Register the treasury's mint as an adopter (Phase 1: 1 mint = 1 adopter)
    [treasuryAdopterPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("adopter"), mint.toBuffer()],
      PROGRAM_ID
    );
    await program.methods
      .registerAdopter(mint)
      .accounts({
        adopterRecord: treasuryAdopterPDA,
        treasury: treasuryPDA,
        authority: payer.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  }

  /**
   * Generate fees by transferring tokens from source to a throwaway
   * recipient, then withdraw fees from the mint into the vault.
   * Returns vault balance after withdrawal.
   *
   * Uses a separate recipient to avoid polluting distribution ATAs.
   */
  async function generateAndWithdrawFees(
    transfers: number = 5,
    amountPerTransfer: bigint = BigInt(10_000_000_000), // 10k tokens
  ): Promise<number> {
    // Snapshot source balance before transfers
    const sourceBefore = await getAccount(provider.connection, sourceATA, "confirmed", TOKEN_2022_PROGRAM_ID);
    const srcBefore = Number(sourceBefore.amount);

    for (let i = 0; i < transfers; i++) {
      const fee = BigInt(Math.floor(Number(amountPerTransfer) * FEE_BASIS_POINTS / 10000));
      try {
        const ix = createTransferCheckedWithFeeInstruction(
          sourceATA, mint, feeRecipientATA, sourceWallet.publicKey,
          amountPerTransfer, MINT_DECIMALS, fee, [], TOKEN_2022_PROGRAM_ID,
        );
        const tx = new Transaction().add(ix);
        await sendAndConfirmTransaction(
          provider.connection, tx, [sourceWallet], { commitment: "confirmed" }
        );
      } catch (e: any) {
        console.error(`  transferCheckedWithFee failed on iteration ${i}:`, e.message?.substring(0, 300));
        throw e;
      }
    }

    // Harvest withheld fees from DESTINATION token account into the mint
    // (transferCheckedWithFee tracks incoming fees in the destination's
    // TransferFeeAmount extension, not the source's)
    const harvestIx = createHarvestWithheldTokensToMintInstruction(mint, [feeRecipientATA], TOKEN_2022_PROGRAM_ID);
    const harvestTx = new Transaction().add(harvestIx);
    await sendAndConfirmTransaction(provider.connection, harvestTx, [payer], { commitment: "confirmed" });

    // Check vault balance before withdraw
    const vaultBefore = await getAccount(provider.connection, vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID);

    // Withdraw fees from mint into treasury vault
    await program.methods
      .withdrawFees()
      .accounts({
        mint,
        treasury: treasuryPDA,
        treasuryVault: vaultPDA,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .rpc();

    // Allow state propagation after CPI
    await new Promise(r => setTimeout(r, 200));

    const vault = await getAccount(
      provider.connection, vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID
    );
    return Number(vault.amount);
  }

  // =========================================================================
  // initialize
  // =========================================================================

  describe("initialize", () => {
    it("successfully initializes a treasury for a configured mint", async () => {
      await setupFresh();

      const treasury: any = await (program.account as any).treasury.fetch(treasuryPDA);
      assert.equal(treasury.mint.toBase58(), mint.toBase58());
      assert.equal(treasury.authority.toBase58(), payer.publicKey.toBase58());
      assert.deepEqual(treasury.phase, { sustenance: {} });
      assert.equal(treasury.totalFeesWithdrawn.toString(), "0");
      assert.equal(treasury.holdersWallet.toBase58(), holdersWallet.publicKey.toBase58());
      assert.equal(treasury.projectDevWallet.toBase58(), devWallet.publicKey.toBase58());
      assert.equal(treasury.ecosystemWallet.toBase58(), ecosystemWallet.publicKey.toBase58());
      assert.equal(treasury.minRunwayBalance.toString(), MIN_RUNWAY.toString());
    });

    it("rejects min_runway_balance below DEFAULT_MIN_RUNWAY (H-3)", async () => {
      const auth = Keypair.generate();
      const mk = Keypair.generate();
      const [tp] = deriveTreasuryPDA(mk.publicKey);
      const [vp] = deriveVaultPDA(mk.publicKey);

      await createMintWithFee(auth, mk, tp);

      try {
        await program.methods
          .initialize(new BN(5_000_000)) // Below DEFAULT_MIN_RUNWAY (10_000_000)
          .accounts({
            mint: mk.publicKey,
            treasury: tp,
            treasuryVault: vp,
            holdersWallet: holdersWallet.publicKey,
            projectDevWallet: devWallet.publicKey,
            ecosystemWallet: ecosystemWallet.publicKey,
            authority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected");
      } catch (err: any) {
        assert.include(err.toString(), "InsufficientRunway",
          `Expected InsufficientRunway, got: ${err}`);
      }
    });

    it("rejects mint without TransferFeeConfig (M-1)", async () => {
      const auth = Keypair.generate();
      const vanillaKp = await createVanillaMint(auth);
      const vanillaMint = vanillaKp.publicKey;
      const [tp] = deriveTreasuryPDA(vanillaMint);
      const [vp] = deriveVaultPDA(vanillaMint);

      try {
        await program.methods
          .initialize(new BN(MIN_RUNWAY))
          .accounts({
            mint: vanillaMint,
            treasury: tp,
            treasuryVault: vp,
            holdersWallet: holdersWallet.publicKey,
            projectDevWallet: devWallet.publicKey,
            ecosystemWallet: ecosystemWallet.publicKey,
            authority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected mint without TransferFeeConfig");
      } catch (err: any) {
        assert.include(err.toString(), "MintNotConfigured",
          `Expected MintNotConfigured, got: ${err}`);
      }
    });
  });

  // =========================================================================
  // verify_adoption
  // =========================================================================

  describe("verify_adoption", () => {
    it("succeeds for a correctly configured mint", async () => {
      await program.methods
        .verifyAdoption()
        .accounts({
          mint,
          treasury: treasuryPDA,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .rpc();
    });

    it("rejects when called on a non-existent treasury PDA", async () => {
      const auth = Keypair.generate();
      const vk = await createVanillaMint(auth);
      const [tp] = deriveTreasuryPDA(vk.publicKey);

      try {
        await program.methods
          .verifyAdoption()
          .accounts({
            mint: vk.publicKey,
            treasury: tp,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();
        assert.fail("Should have rejected — treasury PDA does not exist");
      } catch (err: any) {
        // Anchor: AccountNotInitialized or account deserialization failure
        assert.isTrue(
          err.toString().includes("AccountNotInitialized") ||
          err.toString().includes("account") ||
          err.toString().includes("8iB") ||
          err.toString().includes("InvalidAccountData"),
          `Expected account error, got: ${err}`
        );
      }
    });
  });

  // =========================================================================
  // create_swarm_vault
  // =========================================================================

  describe("create_swarm_vault", () => {
    it("verifies the swarm vault exists from setupFresh", async () => {
      const vault = await getAccount(
        provider.connection, swarmVaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID
      );
      assert.equal(vault.mint.toBase58(), mint.toBase58());
      assert.equal(vault.amount.toString(), "0");
    });

    it("rejects duplicate swarm vault initialization", async () => {
      try {
        await program.methods
          .createSwarmVault()
          .accounts({
            mint,
            treasury: treasuryPDA,
            swarmVault: swarmVaultPDA,
            authority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected duplicate init");
      } catch (err: any) {
        assert.include(err.toString(), "already in use",
          `Expected account-already-in-use, got: ${err}`);
      }
    });
  });

  // =========================================================================
  // withdraw_fees
  // =========================================================================

  describe("withdraw_fees", () => {
    it("withdraws withheld fees and increments total_fees_withdrawn (H-2)", async () => {
      const treasuryBefore: any = await (program.account as any).treasury.fetch(treasuryPDA);
      const feesBefore = Number(treasuryBefore.totalFeesWithdrawn);

      const vaultBalance = await generateAndWithdrawFees(5, BigInt(10_000_000_000));

      const treasuryAfter: any = await (program.account as any).treasury.fetch(treasuryPDA);
      const feesAfter = Number(treasuryAfter.totalFeesWithdrawn);

      assert.isTrue(
        feesAfter > feesBefore,
        `total_fees_withdrawn should increase. Before: ${feesBefore}, After: ${feesAfter}`
      );
      assert.isTrue(vaultBalance > 0,
        `Vault should have tokens. Balance: ${vaultBalance}`);
    });
  });

  // =========================================================================
  // check_redistribute
  // =========================================================================

  describe("check_redistribute", () => {
    it("rejects when vault is below threshold", async () => {
      // Create an isolated mint+treasury with no fees
      const auth = Keypair.generate();
      const mk = Keypair.generate();
      const [tp] = deriveTreasuryPDA(mk.publicKey);
      const [vp] = deriveVaultPDA(mk.publicKey);

      await createMintWithFee(auth, mk, tp);

      await program.methods
        .initialize(new BN(MIN_RUNWAY))
        .accounts({
          mint: mk.publicKey, treasury: tp, treasuryVault: vp,
          holdersWallet: holdersWallet.publicKey,
          projectDevWallet: devWallet.publicKey,
          ecosystemWallet: ecosystemWallet.publicKey,
          authority: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const hATA = await createATA(mk.publicKey, holdersWallet.publicKey);
      const dATA = await createATA(mk.publicKey, devWallet.publicKey);
      const eATA = await createATA(mk.publicKey, ecosystemWallet.publicKey);

      try {
        await program.methods
          .checkRedistribute()
          .accounts({
            mint: mk.publicKey, treasury: tp, treasuryVault: vp,
            holdersRecipient: hATA, devRecipient: dATA, ecosystemRecipient: eATA,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();
        assert.fail("Should have rejected below-threshold");
      } catch (err: any) {
        assert.include(err.toString(), "BelowThreshold",
          `Expected BelowThreshold, got: ${err}`);
      }
    });

    it("rejects wrong holder recipient (C-2/C-3 fix)", async () => {
      // Add fees to existing vault
      await generateAndWithdrawFees(5, BigInt(10_000_000_000));

      const attacker = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(
        attacker.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");

      const attackerATA = await createATA(mint, attacker.publicKey);

      try {
        await program.methods
          .checkRedistribute()
          .accounts({
            mint, treasury: treasuryPDA, treasuryVault: vaultPDA,
            holdersRecipient: attackerATA, // wrong — not holdersWallet's ATA
            devRecipient: devATA,
            ecosystemRecipient: ecosystemATA,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();
        assert.fail("Should have rejected wrong recipient");
      } catch (err: any) {
        // Anchor constraint: token::authority = treasury.holders_wallet
        assert.isTrue(
          err.toString().includes("ConstraintToken") ||
          err.toString().includes("invalid") ||
          err.toString().includes("authority") ||
          err.toString().includes("seeds"),
          `Expected constraint error, got: ${err}`
        );
      }
    });

    it("distributes 70/20/10 split correctly (C-2/C-3 fix)", async () => {
      // Add more fees to ensure enough for redistribution
      await generateAndWithdrawFees(5, BigInt(10_000_000_000));

      // Snapshot balances BEFORE redistribution
      const hBefore = await getAccount(provider.connection, holdersATA, "confirmed", TOKEN_2022_PROGRAM_ID);
      const dBefore = await getAccount(provider.connection, devATA, "confirmed", TOKEN_2022_PROGRAM_ID);
      const eBefore = await getAccount(provider.connection, ecosystemATA, "confirmed", TOKEN_2022_PROGRAM_ID);

      await program.methods
        .checkRedistribute()
        .accounts({
          mint, treasury: treasuryPDA, treasuryVault: vaultPDA,
          holdersRecipient: holdersATA,
          devRecipient: devATA,
          ecosystemRecipient: ecosystemATA,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .rpc();

      // Allow state propagation after CPI transfers
      await new Promise(r => setTimeout(r, 300));

      const hAfter = await getAccount(provider.connection, holdersATA, "confirmed", TOKEN_2022_PROGRAM_ID);
      const dAfter = await getAccount(provider.connection, devATA, "confirmed", TOKEN_2022_PROGRAM_ID);
      const eAfter = await getAccount(provider.connection, ecosystemATA, "confirmed", TOKEN_2022_PROGRAM_ID);

      const hDelta = Number(hAfter.amount - hBefore.amount);
      const dDelta = Number(dAfter.amount - dBefore.amount);
      const eDelta = Number(eAfter.amount - eBefore.amount);
      const total = hDelta + dDelta + eDelta;

      assert.isAbove(total, 0, "Total distributed should be > 0");

      // Verify ~70/20/10 split (±2% tolerance for rounding)
      assert.approximately(hDelta / total, 0.70, 0.02,
        `Holders: ${(hDelta / total * 100).toFixed(1)}% (expected ~70%)`);
      assert.approximately(dDelta / total, 0.20, 0.02,
        `Dev: ${(dDelta / total * 100).toFixed(1)}% (expected ~20%)`);

      // Vault should be at exactly min_runway after redistribution
      const vault = await getAccount(provider.connection, vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID);
      assert.equal(Number(vault.amount), MIN_RUNWAY,
        `Vault should equal min_runway (${MIN_RUNWAY}), got ${vault.amount}`);

      // Verify treasury tracking
      const treasury: any = await (program.account as any).treasury.fetch(treasuryPDA);
      assert.isTrue(
        Number(treasury.totalDistributedHolders) > 0,
        "total_distributed_holders should be > 0"
      );
    });
  });

  // =========================================================================
  // hydrate_swarm
  // =========================================================================

  describe("hydrate_swarm", () => {
    it("successfully hydrates swarm vault", async () => {
      // Add fees to replenish vault
      await generateAndWithdrawFees(5, BigInt(10_000_000_000));

      // Register a Live strategy (required by hydrate_swarm)
      const stratId = "LEGACY_HYD";
      const [stratPDA] = deriveStrategyPDA(treasuryPDA, stratId);
      await program.methods
        .registerStrategy(stratId, 300)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: stratPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Allow state propagation after withdrawFees CPI
      await new Promise(r => setTimeout(r, 200));

      const vault = await getAccount(
        provider.connection, vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID
      );
      const excess = Number(vault.amount) - MIN_RUNWAY;
      if (excess <= Number(DEFAULT_MIN_REDISTRIBUTE)) {
        console.log(`  Skipping hydration test — excess ${excess} too low`);
        return;
      }

      // Hydrate a safe amount (stay above runway)
      const hydrateAmt = new BN(Math.min(excess - Number(DEFAULT_MIN_REDISTRIBUTE), 5_000_000));

      const svBefore = await getAccount(
        provider.connection, swarmVaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID
      );

      await program.methods
        .hydrateSwarm(hydrateAmt)
        .accounts({
          mint,
          treasury: treasuryPDA,
          treasuryVault: vaultPDA,
          swarmVault: swarmVaultPDA,
          strategyRecord: stratPDA,
          adopterRecord: treasuryAdopterPDA,
          authority: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Allow state propagation after CPI transfer
      await new Promise(r => setTimeout(r, 300));

      const svAfter = await getAccount(
        provider.connection, swarmVaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID
      );

      const expectedCredit = Math.floor(hydrateAmt.toNumber() * (10000 - FEE_BASIS_POINTS) / 10000);

      assert.equal(
        Number(svAfter.amount),
        Number(svBefore.amount) + expectedCredit,
        `Swarm vault should receive ${expectedCredit} (after ${FEE_BASIS_POINTS/100}% fee)`
      );

      // Verify runway invariant
      const vaultAfter = await getAccount(
        provider.connection, vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID
      );
      const treasury: any = await (program.account as any).treasury.fetch(treasuryPDA);
      assert.isTrue(
        Number(vaultAfter.amount) >= Number(treasury.minRunwayBalance),
        `Post-hydration vault (${vaultAfter.amount}) >= runway (${treasury.minRunwayBalance})`
      );
    });

    it("rejects hydration that would violate 90-day runway (invariant #9)", async () => {
      // Re-derive the strategy PDA (registered in previous test)
      const stratId = "LEGACY_HYD";
      const [stratPDA] = deriveStrategyPDA(treasuryPDA, stratId);

      const vault = await getAccount(
        provider.connection, vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID
      );
      // Try to hydrate everything except the minimum runway (just 1 token less)
      const greedy = new BN(Number(vault.amount) - MIN_RUNWAY + 1);
      if (Number(vault.amount) <= MIN_RUNWAY) {
        console.log("  Skipping greedy hydration test — vault too low");
        return;
      }

      try {
        await program.methods
          .hydrateSwarm(greedy)
          .accounts({
            mint,
            treasury: treasuryPDA,
            treasuryVault: vaultPDA,
            swarmVault: swarmVaultPDA,
            strategyRecord: stratPDA,
            adopterRecord: treasuryAdopterPDA,
            authority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected greedy hydration");
      } catch (err: any) {
        assert.include(err.toString(), "InsufficientRunway",
          `Expected InsufficientRunway, got: ${err}`);
      }
    });
  });

  // =========================================================================
  // evolve_phase
  // =========================================================================

  describe("evolve_phase", () => {
    it("rejects unauthorized caller (H-1 fix)", async () => {
      const attacker = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(
        attacker.publicKey, anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");

      try {
        await program.methods
          .evolvePhase()
          .accounts({
            mint,
            treasury: treasuryPDA,
            treasuryVault: vaultPDA,
            phaseAuthority: attacker.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .signers([attacker])
          .rpc();
        assert.fail("Should have rejected unauthorized caller");
      } catch (err: any) {
        assert.include(err.toString(), "UnauthorizedPhaseEvolution",
          `Expected UnauthorizedPhaseEvolution, got: ${err}`);
      }
    });

    it("rejects phase evolution when vault below SUSTENANCE_CAP (C-1 fix)", async () => {
      // Our test vault has far less than $50k (50B tokens at 6 decimals)
      try {
        await program.methods
          .evolvePhase()
          .accounts({
            mint,
            treasury: treasuryPDA,
            treasuryVault: vaultPDA,
            phaseAuthority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();
        assert.fail("Should have rejected phase evolution below threshold");
      } catch (err: any) {
        assert.include(err.toString(), "BelowThreshold",
          `Expected BelowThreshold, got: ${err}`);
      }
    });
  });

  // =========================================================================
  // Multi-token attribution
  // =========================================================================

  describe("Multi-token attribution", () => {
    // Derive fake token mint pubkeys for testing adopter records
    const tokenMintA = Keypair.generate().publicKey;
    const tokenMintB = Keypair.generate().publicKey;

    function deriveAdopterPDA(tokenMint: PublicKey): [PublicKey, number] {
      return PublicKey.findProgramAddressSync(
        [Buffer.from("adopter"), tokenMint.toBuffer()],
        PROGRAM_ID
      );
    }

    it("registers an adopter and initialises AdopterRecord", async () => {
      const [adopterPda] = deriveAdopterPDA(tokenMintA);

      await program.methods
        .registerAdopter(tokenMintA)
        .accounts({
          adopterRecord: adopterPda,
          treasury: treasuryPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const record = await program.account.adopterRecord.fetch(adopterPda);
      assert.ok(record.tokenMint.equals(tokenMintA));
      assert.equal(record.feesContributedLamports.toNumber(), 0);
      assert.equal(record.depositCount.toNumber(), 0);
      assert.equal(record.betaExpiresAt.toNumber(), 0);
      assert.equal(record.betaEnded, false);
      assert.equal(record.bump, deriveAdopterPDA(tokenMintA)[1]);
    });

    it("records a fee deposit and increments both adopter and treasury totals", async () => {
      const [adopterPda] = deriveAdopterPDA(tokenMintA);
      const depositAmount = new BN(1_000_000_000); // 1 SOL

      await program.methods
        .recordFeeDeposit(depositAmount)
        .accounts({
          adopterRecord: adopterPda,
          treasury: treasuryPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const record = await program.account.adopterRecord.fetch(adopterPda);
      assert.equal(record.feesContributedLamports.toNumber(), 1_000_000_000);
      assert.equal(record.depositCount.toNumber(), 1);

      const treasury: any = await (program.account as any).treasury.fetch(treasuryPDA);
      assert.isTrue(
        treasury.totalFeesReceivedLamports.toNumber() >= 1_000_000_000,
        "total_fees_received_lamports should be >= 1 SOL"
      );
    });

    it("computes correct pro-rata yield shares for two adopters", async () => {
      // Register second adopter
      const [adopterPdaB] = deriveAdopterPDA(tokenMintB);

      await program.methods
        .registerAdopter(tokenMintB)
        .accounts({
          adopterRecord: adopterPdaB,
          treasury: treasuryPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Deposit 3 SOL from tokenB (tokenA has 1 SOL from previous test)
      await program.methods
        .recordFeeDeposit(new BN(3_000_000_000))
        .accounts({
          adopterRecord: adopterPdaB,
          treasury: treasuryPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const [adopterPdaA] = deriveAdopterPDA(tokenMintA);
      const recordA = await program.account.adopterRecord.fetch(adopterPdaA);
      const recordB = await program.account.adopterRecord.fetch(adopterPdaB);
      const treasury: any = await (program.account as any).treasury.fetch(treasuryPDA);

      // tokenA: 1 SOL / 4 SOL total = 25%
      // tokenB: 3 SOL / 4 SOL total = 75%
      const totalFees = treasury.totalFeesReceivedLamports.toNumber();
      const shareA = recordA.feesContributedLamports.toNumber() / totalFees;
      const shareB = recordB.feesContributedLamports.toNumber() / totalFees;

      assert.approximately(shareA, 0.25, 0.01, "TokenA should have ~25% share");
      assert.approximately(shareB, 0.75, 0.01, "TokenB should have ~75% share");
    });

    it("rejects zero-amount fee deposit", async () => {
      const [adopterPda] = deriveAdopterPDA(tokenMintA);

      try {
        await program.methods
          .recordFeeDeposit(new BN(0))
          .accounts({
            adopterRecord: adopterPda,
            treasury: treasuryPDA,
            authority: payer.publicKey,
          })
          .rpc();
        assert.fail("Should have rejected zero amount");
      } catch (err: any) {
        assert.include(err.toString(), "ZeroAmount");
      }
    });
  });

  // =========================================================================
  // Beta adopter lifecycle
  // =========================================================================

  describe("Beta adopter lifecycle", () => {
    const betaTokenMint = Keypair.generate().publicKey;

    function deriveAdopterPDA(tokenMint: PublicKey): [PublicKey, number] {
      return PublicKey.findProgramAddressSync(
        [Buffer.from("adopter"), tokenMint.toBuffer()],
        PROGRAM_ID
      );
    }

    it("registers a beta adopter with expiry timestamp", async () => {
      const [adopterPda] = deriveAdopterPDA(betaTokenMint);
      // Expires 7 days from now
      const expiresAt = Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60;

      await program.methods
        .registerAdopterBeta(betaTokenMint, new BN(expiresAt))
        .accounts({
          adopterRecord: adopterPda,
          treasury: treasuryPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const record = await program.account.adopterRecord.fetch(adopterPda);
      assert.ok(record.tokenMint.equals(betaTokenMint));
      assert.equal(record.betaExpiresAt.toNumber(), expiresAt);
      assert.equal(record.betaEnded, false);
    });

    it("rejects beta registration with past expiry", async () => {
      const pastMint = Keypair.generate().publicKey;
      const [adopterPda] = deriveAdopterPDA(pastMint);
      const pastExpiry = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago

      try {
        await program.methods
          .registerAdopterBeta(pastMint, new BN(pastExpiry))
          .accounts({
            adopterRecord: adopterPda,
            treasury: treasuryPDA,
            authority: payer.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected past expiry");
      } catch (err: any) {
        assert.include(err.toString(), "BetaExpired");
      }
    });

    it("end_beta sets beta_ended flag and emits event", async () => {
      const [adopterPda] = deriveAdopterPDA(betaTokenMint);

      await program.methods
        .endBeta()
        .accounts({
          adopterRecord: adopterPda,
          treasury: treasuryPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const record = await program.account.adopterRecord.fetch(adopterPda);
      assert.equal(record.betaEnded, true);
    });

    it("rejects end_beta from unauthorized caller", async () => {
      // Re-register a fresh beta adopter to test unauthorized end_beta
      const freshMint = Keypair.generate().publicKey;
      const [adopterPda] = deriveAdopterPDA(freshMint);
      const expiresAt = Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60;

      await program.methods
        .registerAdopterBeta(freshMint, new BN(expiresAt))
        .accounts({
          adopterRecord: adopterPda,
          treasury: treasuryPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const attacker = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(
        attacker.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");

      try {
        await program.methods
          .endBeta()
          .accounts({
            adopterRecord: adopterPda,
            treasury: treasuryPDA,
            authority: attacker.publicKey,
          })
          .signers([attacker])
          .rpc();
        assert.fail("Should have rejected unauthorized caller");
      } catch (err: any) {
        assert.include(err.toString(), "UnauthorizedBetaOp");
      }
    });

    it("hydrate_swarm refuses funding for ended beta adopters", async () => {
      // The betaTokenMint adopter has beta_ended=true from previous test.
      // But hydrate_swarm uses treasury's own adopter record, not this one.
      // We need a setup where the treasury's mint adopter record has beta_ended=true.
      // Instead, test with a fresh isolated setup.

      const auth = Keypair.generate();
      const mk = Keypair.generate();
      const [tp] = deriveTreasuryPDA(mk.publicKey);
      const [vp] = deriveVaultPDA(mk.publicKey);
      const [svp] = deriveSwarmVaultPDA(mk.publicKey);

      await createMintWithFee(auth, mk, tp);

      await program.methods
        .initialize(new BN(MIN_RUNWAY))
        .accounts({
          mint: mk.publicKey, treasury: tp, treasuryVault: vp,
          holdersWallet: holdersWallet.publicKey,
          projectDevWallet: devWallet.publicKey,
          ecosystemWallet: ecosystemWallet.publicKey,
          authority: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Register beta adopter for this mint with future expiry
      const [ap] = PublicKey.findProgramAddressSync(
        [Buffer.from("adopter"), mk.publicKey.toBuffer()],
        PROGRAM_ID
      );
      const expiresAt = Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60;
      await program.methods
        .registerAdopterBeta(mk.publicKey, new BN(expiresAt))
        .accounts({
          adopterRecord: ap,
          treasury: tp,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // End the beta
      await program.methods
        .endBeta()
        .accounts({
          adopterRecord: ap,
          treasury: tp,
          authority: payer.publicKey,
        })
        .rpc();

      // Create swarm vault + register strategy
      await program.methods
        .createSwarmVault()
        .accounts({
          mint: mk.publicKey, treasury: tp, swarmVault: svp,
          authority: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const stratId = "BETA_TEST";
      const [sp] = deriveStrategyPDA(tp, stratId);
      await program.methods
        .registerStrategy(stratId, 300)
        .accounts({
          treasury: tp,
          strategyRecord: sp,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Mint tokens to vault to have balance
      const vaultATA = await createATA(mk.publicKey, payer.publicKey);
      // Actually need to fund the treasury vault. Let's transfer to vault directly.
      const sourceATA2 = await createATA(mk.publicKey, sourceWallet.publicKey);
      await mintTo(
        provider.connection, payer, mk.publicKey, sourceATA2, auth,
        BigInt(100_000_000_000), [], undefined, TOKEN_2022_PROGRAM_ID
      );

      // Transfer with fee to generate vault balance (simplified: just send to fee recipient, harvest, withdraw)
      // For simplicity, let's just attempt hydration with 0 balance — it should still hit the BetaExpired check
      // before the balance check if beta_ended is true.

      try {
        await program.methods
          .hydrateSwarm(new BN(1_000_000))
          .accounts({
            mint: mk.publicKey,
            treasury: tp,
            treasuryVault: vp,
            swarmVault: svp,
            strategyRecord: sp,
            adopterRecord: ap,
            authority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected beta-ended adopter");
      } catch (err: any) {
        assert.include(err.toString(), "BetaExpired",
          `Expected BetaExpired, got: ${err}`);
      }
    });
  });

  // =========================================================================
  // freeze_treasury / unfreeze_treasury
  // =========================================================================

  describe("freeze_treasury / unfreeze_treasury", () => {
    let fMint: PublicKey, fMintKp: Keypair, fAuth: Keypair;
    let fTreasury: PublicKey, fVault: PublicKey, fSwarmVault: PublicKey;

    beforeEach(async () => {
      fAuth = Keypair.generate();
      fMintKp = Keypair.generate();
      fMint = fMintKp.publicKey;

      await createMintWithFee(fAuth, fMintKp, payer.publicKey);

      [fTreasury] = deriveTreasuryPDA(fMint);
      [fVault] = deriveVaultPDA(fMint);
      [fSwarmVault] = deriveSwarmVaultPDA(fMint);

      // Initialize treasury
      await program.methods
        .initialize(new BN(MIN_RUNWAY))
        .accounts({
          mint: fMint,
          treasury: fTreasury,
          treasuryVault: fVault,
          authority: payer.publicKey,
          holdersWallet: Keypair.generate().publicKey,
          projectDevWallet: Keypair.generate().publicKey,
          ecosystemWallet: Keypair.generate().publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });

    it("authority can freeze treasury", async () => {
      const stateBefore = await program.account.treasury.fetch(fTreasury);
      assert.isFalse(stateBefore.frozen);

      await program.methods
        .freezeTreasury()
        .accounts({
          treasury: fTreasury,
          authority: payer.publicKey,
        })
        .rpc();

      const stateAfter = await program.account.treasury.fetch(fTreasury);
      assert.isTrue(stateAfter.frozen);
    });

    it("authority can unfreeze treasury", async () => {
      // Freeze first
      await program.methods
        .freezeTreasury()
        .accounts({ treasury: fTreasury, authority: payer.publicKey })
        .rpc();

      // Unfreeze
      await program.methods
        .unfreezeTreasury()
        .accounts({ treasury: fTreasury, authority: payer.publicKey })
        .rpc();

      const state = await program.account.treasury.fetch(fTreasury);
      assert.isFalse(state.frozen);
    });

    it("non-authority cannot freeze", async () => {
      const imposter = Keypair.generate();
      // Airdrop SOL for tx fee
      const sig = await provider.connection.requestAirdrop(imposter.publicKey, 1_000_000_000);
      await provider.connection.confirmTransaction(sig);

      try {
        await program.methods
          .freezeTreasury()
          .accounts({
            treasury: fTreasury,
            authority: imposter.publicKey,
          })
          .signers([imposter])
          .rpc();
        assert.fail("Non-authority should not be able to freeze");
      } catch (err: any) {
        assert.include(err.toString(), "UnauthorizedPhaseEvolution",
          `Expected UnauthorizedPhaseEvolution, got: ${err}`);
      }
    });

    it("non-authority cannot unfreeze", async () => {
      // Freeze first
      await program.methods
        .freezeTreasury()
        .accounts({ treasury: fTreasury, authority: payer.publicKey })
        .rpc();

      const imposter = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(imposter.publicKey, 1_000_000_000);
      await provider.connection.confirmTransaction(sig);

      try {
        await program.methods
          .unfreezeTreasury()
          .accounts({
            treasury: fTreasury,
            authority: imposter.publicKey,
          })
          .signers([imposter])
          .rpc();
        assert.fail("Non-authority should not be able to unfreeze");
      } catch (err: any) {
        assert.include(err.toString(), "UnauthorizedPhaseEvolution",
          `Expected UnauthorizedPhaseEvolution, got: ${err}`);
      }
    });

    it("double-freeze rejected (AlreadyFrozen)", async () => {
      await program.methods
        .freezeTreasury()
        .accounts({ treasury: fTreasury, authority: payer.publicKey })
        .rpc();

      try {
        await program.methods
          .freezeTreasury()
          .accounts({ treasury: fTreasury, authority: payer.publicKey })
          .rpc();
        assert.fail("Double freeze should be rejected");
      } catch (err: any) {
        assert.include(err.toString(), "AlreadyFrozen",
          `Expected AlreadyFrozen, got: ${err}`);
      }
    });

    it("unfreeze on non-frozen rejected (NotFrozen)", async () => {
      try {
        await program.methods
          .unfreezeTreasury()
          .accounts({ treasury: fTreasury, authority: payer.publicKey })
          .rpc();
        assert.fail("Unfreeze on non-frozen should be rejected");
      } catch (err: any) {
        assert.include(err.toString(), "NotFrozen",
          `Expected NotFrozen, got: ${err}`);
      }
    });

    it("frozen treasury rejects withdraw_fees", async () => {
      await program.methods
        .freezeTreasury()
        .accounts({ treasury: fTreasury, authority: payer.publicKey })
        .rpc();

      try {
        await program.methods
          .withdrawFees()
          .accounts({
            mint: fMint,
            treasury: fTreasury,
            treasuryVault: fVault,
            feeWithheldAccount: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();
        assert.fail("withdraw_fees on frozen treasury should be rejected");
      } catch (err: any) {
        assert.include(err.toString(), "TreasuryFrozen",
          `Expected TreasuryFrozen, got: ${err}`);
      }
    });

    it("frozen treasury rejects check_redistribute", async () => {
      await program.methods
        .freezeTreasury()
        .accounts({ treasury: fTreasury, authority: payer.publicKey })
        .rpc();

      try {
        await program.methods
          .checkRedistribute(new BN(DEFAULT_MIN_REDISTRIBUTE))
          .accounts({
            mint: fMint,
            treasury: fTreasury,
            treasuryVault: fVault,
            holdersWallet: Keypair.generate().publicKey,
            projectDevWallet: Keypair.generate().publicKey,
            ecosystemWallet: Keypair.generate().publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
          })
          .rpc();
        assert.fail("check_redistribute on frozen treasury should be rejected");
      } catch (err: any) {
        assert.include(err.toString(), "TreasuryFrozen",
          `Expected TreasuryFrozen, got: ${err}`);
      }
    });
  });
});
