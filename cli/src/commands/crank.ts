// RTP CLI — rtp crank: Fee sweep and redistribution.

import { Command } from "commander";
import ora from "ora";

import { loadConfig, resolveMint, resolveKeypair } from "../config.js";
import { loadKeypair, truncatePubkey, formatSol } from "../keypair.js";
import { printOk, printInfo, printNote, printWarn, getOutputMode } from "../format.js";
import { printError } from "../errors.js";
import { createConnection, explorerTxUrl } from "../lib/rpc.js";

export function makeCrankCommand(): Command {
  const cmd = new Command("crank")
    .description("Fee sweep and redistribution");

  // crank fees — permissionless
  cmd.addCommand(
    new Command("fees")
      .description("Sweep TransferFeeConfig fees into treasury PDA vaults")
      .option("--mint <pubkey>", "Token mint address")
      .option("--all", "Sweep all known treasuries")
      .option("--dry-run", "Show what would be swept without sending")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const keypairPath = resolveKeypair(undefined, "KEYPAIR_PATH", config.feePayerKeypairPath);
          const payer = loadKeypair(keypairPath);
          const connection = createConnection(config);

          // Import the refactored export from scripts
          const { exportSweepFees } = await import("../../../scripts/fee-crank.ts");
          const mintStr = opts.mint ?? config.defaultMint;
          const PublicKey = (await import("@solana/web3.js")).PublicKey;

          if (opts.dryRun) {
            if (mode !== "quiet") printInfo("Dry run — no transactions will be sent.");
          }

          const mints: string[] = [];
          if (opts.all) {
            // TODO: iterate registered adopters
            if (mode !== "quiet") printWarn("--all not yet implemented. Use --mint.");
            return;
          } else if (mintStr) {
            mints.push(mintStr);
          } else {
            throw new Error("No mint specified. Use --mint <pubkey> or --all.");
          }

          for (const mint of mints) {
            const spinner = mode === "human" ? ora(`Sweeping fees for ${truncatePubkey(mint)}...`).start() : null;

            try {
              const result = await exportSweepFees(connection, payer, new PublicKey(mint), {
                dryRun: opts.dryRun,
                jitterMaxMs: 0, // No jitter when run from CLI
              });

              spinner?.succeed();

              if (mode === "json") {
                console.log(JSON.stringify({ mint, ...result }, null, 2));
                return;
              }

              if (result.withdrawSig) {
                printOk(`Withdraw TX: ${explorerTxUrl(result.withdrawSig, opts.cluster)}`);
              }
              if (result.redistributeSig) {
                printOk(`Redistribute TX: ${explorerTxUrl(result.redistributeSig, opts.cluster)}`);
              } else {
                printNote("No redistribution (BelowThreshold — vault excess below runway floor)");
              }
            } catch (e) {
              spinner?.fail();
              throw e;
            }
          }
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // crank redistribute — permissionless
  cmd.addCommand(
    new Command("redistribute")
      .description("Trigger the 70/20/10 on-chain redistribution")
      .option("--mint <pubkey>", "Token mint address")
      .option("--dry-run", "Show pro-rata shares without sending")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const mint = resolveMint(opts.mint, config);
          const connection = createConnection(config);

          if (opts.dryRun) {
            // Show pro-rata shares using the pure function
            const { computeAdopterYieldShares } = await import("../../../scripts/compute_adopter_yield_share.ts");
            if (mode !== "quiet") printInfo("Dry run — computing pro-rata shares...");
            // Would need adopter records from chain; show example
            printNote("Dry run: fetch adopter records from chain, compute shares, display table.");
            printNote("Use without --dry-run to actually trigger redistribution on-chain.");
            return;
          }

          const keypairPath = resolveKeypair(undefined, "KEYPAIR_PATH", config.feePayerKeypairPath);
          const payer = loadKeypair(keypairPath);
          const PublicKey = (await import("@solana/web3.js")).PublicKey;

          const spinner = mode === "human" ? ora("Running redistribution...").start() : null;
          const sdk = await import("../../../sdk/index.ts");
          const result = await sdk.withdrawAndRedistribute(connection, payer, new PublicKey(mint));

          spinner?.succeed();
          if (mode === "json") {
            console.log(JSON.stringify(result, null, 2));
            return;
          }

          if (result.withdrawSig) {
            printOk(`Withdraw TX: ${explorerTxUrl(result.withdrawSig, opts.cluster)}`);
          }
          if (result.redistributeSig) {
            printOk(`Redistribute TX: ${explorerTxUrl(result.redistributeSig, opts.cluster)}`);
          } else {
            printNote("No redistribution (BelowThreshold)");
          }
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  return cmd;
}
