/**
 * Strategy Lifecycle — Anchor Integration Tests
 *
 * Tests on-chain strategy lifecycle enforcement:
 *   - register_strategy → update_strategy_performance → hydrate_swarm
 *   - Hard stop transitions (drawdown, consecutive losses, rolling Sharpe)
 *   - Soft decay retirement (3 strikes)
 *   - force_retire_strategy (authority override)
 *   - hydrate_swarm rejection for Suspended/Retired strategies
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

const FEE_BASIS_POINTS = 1000;
const MAX_FEE = BigInt(1_000_000_000);
const MINT_DECIMALS = 6;
const MIN_RUNWAY = 10_000_000;
const DEFAULT_MIN_REDISTRIBUTE = 1_000_000;
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

describe("strategy-lifecycle", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const payer = (provider.wallet as anchor.Wallet).payer;

  let program: any;
  let idl: any;

  // Shared state
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

  // -----------------------------------------------------------------------
  // Helpers
  // -----------------------------------------------------------------------

  async function createMintWithFee(
    mintAuth: Keypair,
    mintKp: Keypair,
    withdrawWithheldAuthority: PublicKey,
    decimals: number = MINT_DECIMALS,
    feeBps: number = FEE_BASIS_POINTS
  ): Promise<void> {
    const mintPub = mintKp.publicKey;
    const space = MINT_SPACE_WITH_FEE;
    const lamports = await provider.connection.getMinimumBalanceForRentExemption(
      space
    );

    const tx1 = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: mintPub,
        space,
        lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeTransferFeeConfigInstruction(
        mintPub,
        mintAuth.publicKey,
        withdrawWithheldAuthority,
        feeBps,
        MAX_FEE,
        TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(tx1, [payer, mintKp]);

    const tx2 = new Transaction().add(
      createInitializeMint2Instruction(
        mintPub,
        decimals,
        mintAuth.publicKey,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(tx2, [payer]);
  }

  async function createATA(mint: PublicKey, owner: PublicKey): Promise<PublicKey> {
    const ata = getAssociatedTokenAddressSync(
      mint,
      owner,
      false,
      TOKEN_2022_PROGRAM_ID
    );
    const tx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        payer.publicKey,
        ata,
        owner,
        mint,
        TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(tx, []);
    return ata;
  }

  async function setupFresh(): Promise<void> {
    mintAuthKp = Keypair.generate();
    const mintKp = Keypair.generate();
    mint = mintKp.publicKey;

    [treasuryPDA] = deriveTreasuryPDA(mint);
    [vaultPDA] = deriveVaultPDA(mint);
    [swarmVaultPDA] = deriveSwarmVaultPDA(mint);

    await createMintWithFee(mintAuthKp, mintKp, treasuryPDA);

    holdersATA = await createATA(mint, holdersWallet.publicKey);
    devATA = await createATA(mint, devWallet.publicKey);
    ecosystemATA = await createATA(mint, ecosystemWallet.publicKey);
    sourceATA = await createATA(mint, sourceWallet.publicKey);
    feeRecipientATA = await createATA(mint, feeRecipientWallet.publicKey);

    await mintTo(
      provider.connection,
      payer,
      mint,
      sourceATA,
      mintAuthKp,
      BigInt(1_000_000_000_000),
      [],
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

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
  }

  async function generateAndWithdrawFees(
    transfers: number = 5,
    amountPerTransfer: bigint = BigInt(10_000_000_000)
  ): Promise<number> {
    for (let i = 0; i < transfers; i++) {
      const fee = BigInt(
        Math.floor(Number(amountPerTransfer) * (FEE_BASIS_POINTS / 10000))
      );
      const ix = createTransferCheckedWithFeeInstruction(
        sourceATA,
        mint,
        feeRecipientATA,
        sourceWallet.publicKey,
        amountPerTransfer,
        MINT_DECIMALS,
        fee,
        [],
        TOKEN_2022_PROGRAM_ID
      );
      const tx = new Transaction().add(ix);
      await sendAndConfirmTransaction(provider.connection, tx, [sourceWallet], {
        commitment: "confirmed",
      });
    }

    const harvestIx = createHarvestWithheldTokensToMintInstruction(
      mint,
      [feeRecipientATA],
      TOKEN_2022_PROGRAM_ID
    );
    const harvestTx = new Transaction().add(harvestIx);
    await sendAndConfirmTransaction(provider.connection, harvestTx, [payer], {
      commitment: "confirmed",
    });

    await program.methods
      .withdrawFees()
      .accounts({
        mint,
        treasury: treasuryPDA,
        treasuryVault: vaultPDA,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .rpc();

    await new Promise((r) => setTimeout(r, 200));

    const vault = await getAccount(
      provider.connection,
      vaultPDA,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );
    return Number(vault.amount);
  }

  before(async () => {
    idl = require("../target/idl/rtp_treasury.json");
    program = new anchor.Program(idl, provider);

    holdersWallet = Keypair.generate();
    devWallet = Keypair.generate();
    ecosystemWallet = Keypair.generate();
    sourceWallet = Keypair.generate();
    feeRecipientWallet = Keypair.generate();

    for (const w of [
      holdersWallet,
      devWallet,
      ecosystemWallet,
      sourceWallet,
      feeRecipientWallet,
    ]) {
      const sig = await provider.connection.requestAirdrop(
        w.publicKey,
        5 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");
    }
  });

  // =========================================================================
  // register_strategy
  // =========================================================================

  describe("register_strategy", () => {
    it("registers a strategy with Live status", async () => {
      await setupFresh();

      const strategyId = "SOL_SV2";
      const promoSharpe = 396; // 3.96
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, promoSharpe)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.ok(record.treasury.equals(treasuryPDA));
      assert.equal(record.strategyId, strategyId);
      assert.deepEqual(record.status, { live: {} });
      assert.equal(record.promotionSharpeX100, promoSharpe);
      assert.equal(record.rollingSharpeX100, promoSharpe);
      assert.equal(record.rollingPnlBps, 0);
      assert.equal(record.consecutiveLosses, 0);
      assert.equal(record.softDecayStrikes, 0);
      assert.equal(record.drawdown24HBps, 0);
      assert.equal(record.totalTrades, 0);
    });

    it("rejects strategy ID longer than 16 characters", async () => {
      const longId = "A".repeat(17);
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, longId);

      try {
        await program.methods
          .registerStrategy(longId, 396)
          .accounts({
            treasury: treasuryPDA,
            strategyRecord: strategyPDA,
            authority: payer.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected long strategy ID");
      } catch (err: any) {
        assert.include(
          err.toString(),
          "InvalidStrategyId",
          `Expected InvalidStrategyId, got: ${err}`
        );
      }
    });

    it("rejects empty strategy ID", async () => {
      const emptyId = "";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, emptyId);

      try {
        await program.methods
          .registerStrategy(emptyId, 396)
          .accounts({
            treasury: treasuryPDA,
            strategyRecord: strategyPDA,
            authority: payer.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected empty strategy ID");
      } catch (err: any) {
        assert.include(
          err.toString(),
          "InvalidStrategyId",
          `Expected InvalidStrategyId, got: ${err}`
        );
      }
    });

    it("rejects unauthorized caller (non-authority)", async () => {
      const attacker = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(
        attacker.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");

      const strategyId = "UNAUTH";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      try {
        await program.methods
          .registerStrategy(strategyId, 396)
          .accounts({
            treasury: treasuryPDA,
            strategyRecord: strategyPDA,
            authority: attacker.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([attacker])
          .rpc();
        assert.fail("Should have rejected unauthorized caller");
      } catch (err: any) {
        assert.include(
          err.toString(),
          "UnauthorizedStrategyOp",
          `Expected UnauthorizedStrategyOp, got: ${err}`
        );
      }
    });
  });

  // =========================================================================
  // update_strategy_performance
  // =========================================================================

  describe("update_strategy_performance", () => {
    it("happy path: healthy update keeps status Live", async () => {
      await setupFresh();

      const strategyId = "HEALTHY";
      const promoSharpe = 396;
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, promoSharpe)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Healthy update
      await program.methods
        .updateStrategyPerformance(
          350,  // rolling_pnl_bps (+3.5%)
          280,  // rolling_sharpe_x100 (2.8)
          1,    // consecutive_losses
          50,   // drawdown_24h_bps (0.5%)
          false // new_soft_strike
        )
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { live: {} });
      assert.equal(record.rollingPnlBps, 350);
      assert.equal(record.rollingSharpeX100, 280);
      assert.equal(record.consecutiveLosses, 1);
      assert.equal(record.drawdown24HBps, 50);
      assert.equal(record.totalTrades, 1);
    });

    it("hard stop: drawdown >= 1000 bps → Suspended", async () => {
      await setupFresh();

      const strategyId = "DRAWDN";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await program.methods
        .updateStrategyPerformance(0, 200, 2, 1200, false) // 12% drawdown
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { suspended: {} });
    });

    it("hard stop: 5 consecutive losses → Suspended", async () => {
      await setupFresh();

      const strategyId = "CONSEC";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await program.methods
        .updateStrategyPerformance(0, 200, 5, 100, false)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { suspended: {} });
    });

    it("hard stop: rolling Sharpe < 0.5 → Suspended", async () => {
      await setupFresh();

      const strategyId = "SHARPE";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await program.methods
        .updateStrategyPerformance(-50, 30, 1, 100, false) // Sharpe 0.3 < 0.5
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { suspended: {} });
    });

    it("soft decay: 3 strikes → Retired", async () => {
      await setupFresh();

      const strategyId = "SOFT";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Strike 1
      await program.methods
        .updateStrategyPerformance(100, 200, 0, 100, true)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      let record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { live: {} });
      assert.equal(record.softDecayStrikes, 1);

      // Strike 2
      await program.methods
        .updateStrategyPerformance(100, 200, 0, 100, true)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { live: {} });
      assert.equal(record.softDecayStrikes, 2);

      // Strike 3 — retirement
      await program.methods
        .updateStrategyPerformance(100, 200, 0, 100, true)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { retired: {} });
      assert.equal(record.softDecayStrikes, 3);
    });

    it("rejects update on Suspended strategy", async () => {
      await setupFresh();

      const strategyId = "REJ_SUS";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Suspend via hard stop
      await program.methods
        .updateStrategyPerformance(0, 200, 5, 100, false)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      // Try to update again — should fail
      try {
        await program.methods
          .updateStrategyPerformance(100, 200, 0, 50, false)
          .accounts({
            treasury: treasuryPDA,
            strategyRecord: strategyPDA,
            authority: payer.publicKey,
          })
          .rpc();
        assert.fail("Should have rejected update on Suspended strategy");
      } catch (err: any) {
        assert.include(
          err.toString(),
          "StrategyNotLive",
          `Expected StrategyNotLive, got: ${err}`
        );
      }
    });
  });

  // =========================================================================
  // hydrate_swarm with strategy lifecycle gate
  // =========================================================================

  describe("hydrate_swarm with strategy gate", () => {
    it("happy path: register → update (healthy) → hydrate succeeds", async () => {
      await setupFresh();
      await generateAndWithdrawFees(5, BigInt(10_000_000_000));

      const strategyId = "HYD_OK";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Healthy update
      await program.methods
        .updateStrategyPerformance(350, 280, 1, 50, false)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const vault = await getAccount(
        provider.connection,
        vaultPDA,
        "confirmed",
        TOKEN_2022_PROGRAM_ID
      );
      const excess = Number(vault.amount) - MIN_RUNWAY;
      if (excess <= Number(DEFAULT_MIN_REDISTRIBUTE)) {
        console.log("  Skipping hydration happy path — excess too low");
        return;
      }

      const hydrateAmt = new BN(
        Math.min(excess - Number(DEFAULT_MIN_REDISTRIBUTE), 5_000_000)
      );

      // This should succeed — strategy is Live
      await program.methods
        .hydrateSwarm(hydrateAmt)
        .accounts({
          mint,
          treasury: treasuryPDA,
          treasuryVault: vaultPDA,
          swarmVault: swarmVaultPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Allow state propagation after CPI transfer
      await new Promise(r => setTimeout(r, 300));

      const svAfter = await getAccount(
        provider.connection,
        swarmVaultPDA,
        "confirmed",
        TOKEN_2022_PROGRAM_ID
      );
      assert.isTrue(
        Number(svAfter.amount) > 0,
        "Swarm vault should have received tokens"
      );
    });

    it("hard stop path: drawdown >= 1000 → hydrate fails", async () => {
      await setupFresh();
      await generateAndWithdrawFees(5, BigInt(10_000_000_000));

      const strategyId = "HYD_FAIL";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Trigger hard stop via drawdown
      await program.methods
        .updateStrategyPerformance(0, 200, 2, 1200, false)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      const vault = await getAccount(
        provider.connection,
        vaultPDA,
        "confirmed",
        TOKEN_2022_PROGRAM_ID
      );
      const excess = Number(vault.amount) - MIN_RUNWAY;
      if (excess <= Number(DEFAULT_MIN_REDISTRIBUTE)) {
        console.log("  Skipping hydration rejection test — excess too low");
        return;
      }

      const hydrateAmt = new BN(1_000_000);

      try {
        await program.methods
          .hydrateSwarm(hydrateAmt)
          .accounts({
            mint,
            treasury: treasuryPDA,
            treasuryVault: vaultPDA,
            swarmVault: swarmVaultPDA,
            strategyRecord: strategyPDA,
            authority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected hydration of Suspended strategy");
      } catch (err: any) {
        assert.include(
          err.toString(),
          "StrategyNotLive",
          `Expected StrategyNotLive, got: ${err}`
        );
      }
    });

    it("soft decay path: 3 strikes → Retired → hydrate fails", async () => {
      await setupFresh();
      await generateAndWithdrawFees(5, BigInt(10_000_000_000));

      const strategyId = "HYD_SOFT";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // 3 soft strikes → Retired
      for (let i = 0; i < 3; i++) {
        await program.methods
          .updateStrategyPerformance(100, 200, 0, 100, true)
          .accounts({
            treasury: treasuryPDA,
            strategyRecord: strategyPDA,
            authority: payer.publicKey,
          })
          .rpc();
      }

      const record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { retired: {} });

      const hydrateAmt = new BN(1_000_000);

      try {
        await program.methods
          .hydrateSwarm(hydrateAmt)
          .accounts({
            mint,
            treasury: treasuryPDA,
            treasuryVault: vaultPDA,
            swarmVault: swarmVaultPDA,
            strategyRecord: strategyPDA,
            authority: payer.publicKey,
            tokenProgram: TOKEN_2022_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected hydration of Retired strategy");
      } catch (err: any) {
        assert.include(
          err.toString(),
          "StrategyNotLive",
          `Expected StrategyNotLive, got: ${err}`
        );
      }
    });
  });

  // =========================================================================
  // force_retire_strategy
  // =========================================================================

  describe("force_retire_strategy", () => {
    it("authority can force-retire a Live strategy", async () => {
      await setupFresh();

      const strategyId = "FORCE";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Confirm Live
      let record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { live: {} });

      // Force retire
      await program.methods
        .forceRetireStrategy()
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
        })
        .rpc();

      record = await program.account.strategyRecord.fetch(strategyPDA);
      assert.deepEqual(record.status, { retired: {} });
    });

    it("rejects non-authority force-retire", async () => {
      await setupFresh();

      const strategyId = "FORCE2";
      const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);

      await program.methods
        .registerStrategy(strategyId, 396)
        .accounts({
          treasury: treasuryPDA,
          strategyRecord: strategyPDA,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const attacker = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(
        attacker.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig, "confirmed");

      try {
        await program.methods
          .forceRetireStrategy()
          .accounts({
            treasury: treasuryPDA,
            strategyRecord: strategyPDA,
            authority: attacker.publicKey,
          })
          .signers([attacker])
          .rpc();
        assert.fail("Should have rejected non-authority force-retire");
      } catch (err: any) {
        assert.include(
          err.toString(),
          "UnauthorizedStrategyOp",
          `Expected UnauthorizedStrategyOp, got: ${err}`
        );
      }
    });
  });
});
