// RTP CLI — rtp deploy: Deploy treasury or program.

import { Command } from "commander";
import chalk from "chalk";
import { execSync } from "child_process";

import { loadConfig, resolveKeypair } from "../config.js";
import { loadKeypair, truncatePubkey, formatSol } from "../keypair.js";
import { printOk, printInfo, printNote, getOutputMode } from "../format.js";
import { printError, missingYesFlagError } from "../errors.js";
import { createConnection, explorerTxUrl } from "../lib/rpc.js";
import { confirmMainnetDeploy, warnDevnetFlashTrade, warnMainnetFlashTrade, warnHotWallet } from "../lib/safety.js";

export function makeDeployCommand(): Command {
  const cmd = new Command("deploy")
    .description("Deploy treasury or program");

  // deploy treasury
  cmd.addCommand(
    new Command("treasury")
      .description("Deploy the treasury PDA for a new adopting token")
      .requiredOption("--authority <path>", "Authority keypair path")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const authorityPath = resolveKeypair(opts.authority, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
          const authority = loadKeypair(authorityPath);
          const connection = createConnection(config);

          if (mode !== "quiet") {
            printInfo(`Deploying treasury for authority: ${truncatePubkey(authority.publicKey)}`);
            warnHotWallet(authorityPath);
          }

          const sdk = await import("../../../sdk/index.ts");
          const result = await sdk.registerWithRTP(connection, authority, { authority: authority.publicKey });

          if (mode === "json") {
            console.log(JSON.stringify(result, null, 2));
            return;
          }

          printOk(`Treasury deployed!`);
          printInfo(`Treasury PDA: ${result.treasuryPDA ?? "see explorer"}`);
          printInfo(`TX: ${explorerTxUrl(result.signature ?? "", opts.cluster)}`);
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // deploy program
  cmd.addCommand(
    new Command("program")
      .description("Build and deploy the RTP treasury Anchor program")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--yes", "Skip confirmation prompt")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        try {
          const config = loadConfig();
          const cluster = opts.cluster;

          if (cluster === "mainnet") {
            warnMainnetFlashTrade();
            if (!opts.yes) {
              console.log(chalk.yellow("\n⚠ Type 'mainnet' to confirm mainnet program deployment:"));
              // In non-interactive context, require --yes
              throw missingYesFlagError("mainnet program deployment");
            }
          } else {
            warnDevnetFlashTrade();
          }

          if (!opts.quiet) {
            printInfo(`Building Anchor program for ${cluster}...`);
          }

          // Build
          execSync("anchor build", {
            cwd: "rtp/programs/rtp-treasury",
            stdio: opts.quiet ? "pipe" : "inherit",
          });

          if (!opts.quiet) printOk("Build complete.");

          // Deploy
          if (!opts.quiet) printInfo(`Deploying to ${cluster}...`);
          execSync(`anchor deploy --provider.cluster ${cluster}`, {
            cwd: "rtp/programs/rtp-treasury",
            stdio: opts.quiet ? "pipe" : "inherit",
          });

          if (!opts.quiet) printOk(`Program deployed to ${cluster}.`);
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  return cmd;
}
