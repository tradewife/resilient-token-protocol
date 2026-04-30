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

  // crank fees — deprecated: Token-2022 TransferFeeConfig sweep no longer exists.
  // In the SOL-native model, fees are deposited via deposit_sol (anyone can trigger
  // a TransferFeeConfig transfer to the treasury vault — that flow is removed).
  // Fee accounting is handled by record_fee_deposit (authority-gated) and redistribution
  // is triggered via crank redistribute.
  cmd.addCommand(
    new Command("fees")
      .description("[DEPRECATED] Token-2022 fee sweep — use depositSol + crank redistribute")
      .option("--mint <pubkey>", "Token mint address")
      .option("--all", "Sweep all known treasuries")
      .option("--dry-run", "Show what would be swept without sending")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        if (mode !== "quiet") {
          printWarn("crank fees is deprecated. Token-2022 TransferFeeConfig sweep has been removed.");
          printNote("For native SOL redistribution, use: rtp crank redistribute --authority <pubkey>");
        }
        process.exit(1);
      }),
  );

  // crank redistribute — permissionless
  cmd.addCommand(
    new Command("redistribute")
      .description("Trigger the 70/20/10 on-chain redistribution")
      .option("--authority <pubkey>", "Treasury authority address")
      .option("--dry-run", "Show pro-rata shares without sending")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const authorityStr = opts.authority ?? (config as any).defaultAuthority;
          if (!authorityStr) {
            throw new Error("No authority specified. Use --authority <pubkey>.");
          }
          const connection = createConnection(config);

          if (opts.dryRun) {
            const { computeAdopterYieldShares } = await import("../../../scripts/compute_adopter_yield_share.ts");
            if (mode !== "quiet") printInfo("Dry run — computing pro-rata shares...");
            printNote("Fetch adopter records from chain, compute shares, display table.");
            printNote("Use without --dry-run to actually trigger redistribution on-chain.");
            return;
          }

          const keypairPath = resolveKeypair(undefined, "KEYPATH", config.feePayerKeypairPath);
          const payer = loadKeypair(keypairPath);
          const PublicKey = (await import("@solana/web3.js")).PublicKey;
          const authorityPk = new PublicKey(authorityStr);

          const spinner = mode === "human" ? ora("Running redistribution...").start() : null;
          const sdk = await import("../../../sdk/index.ts");
          const result = await sdk.checkRedistribute(connection, payer, authorityPk);

          spinner?.succeed();
          if (mode === "json") {
            console.log(JSON.stringify(result, null, 2));
            return;
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
