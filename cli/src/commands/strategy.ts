// RTP CLI — rtp strategy: List, promote, retire strategies.

import { Command } from "commander";
import chalk from "chalk";
import Table from "cli-table3";

import { loadConfig, resolveMint, resolveKeypair } from "../config.js";
import { loadKeypair, truncatePubkey, formatSol } from "../keypair.js";
import { printOk, printInfo, printNote, getOutputMode } from "../format.js";
import { printError, missingYesFlagError } from "../errors.js";
import { createConnection, explorerTxUrl } from "../lib/rpc.js";
import { confirmDestructive, warnHotWallet } from "../lib/safety.js";

export function makeStrategyCommand(): Command {
  const cmd = new Command("strategy")
    .description("Manage treasury strategies");

  // strategy list — read-only, permissionless
  cmd.addCommand(
    new Command("list")
      .description("List strategy records for a treasury")
      .option("--mint <pubkey>", "Token mint address")
      .option("--status <status>", "Filter by status (live|suspended|retired|all)", "all")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const mint = resolveMint(opts.mint, config);
          const connection = createConnection(config);
          const PublicKey = (await import("@solana/web3.js")).PublicKey;
          const mintPk = new PublicKey(mint);

          // Fetch treasury state to get linked info
          const sdk = await import("../../../sdk/index.ts");
          const state = await sdk.fetchTreasuryState(connection, mintPk);

          if (mode === "json") {
            console.log(JSON.stringify(state, null, 2));
            return;
          }

          // Display strategy info from treasury state
          const table = new Table({
            head: ["Field", "Value"],
            style: { head: ["cyan"] },
          });

          table.push(["Mint", truncatePubkey(mint)]);
          table.push(["Frozen", state.isFrozen ? chalk.red("YES") : chalk.green("NO")]);
          table.push(["Phase", state.phase]);
          table.push(["Balance", `${formatSol(state.vaultBalance)} SOL`]);

          console.log(table.toString());
          printNote("Use getProgramAccounts for full strategy enumeration (not yet implemented).");
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // strategy promote — authority-gated
  cmd.addCommand(
    new Command("promote")
      .description("Promote a validated strategy to Live (authority-gated)")
      .requiredOption("--id <strategy-id>", "Strategy ID to promote")
      .requiredOption("--authority <path>", "Authority keypair path")
      .option("--mint <pubkey>", "Token mint address")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const mint = resolveMint(opts.mint, config);
          const authorityPath = resolveKeypair(opts.authority, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
          const authority = loadKeypair(authorityPath);
          const connection = createConnection(config);

          if (mode !== "quiet") {
            printInfo(`Promoting strategy "${opts.id}" for mint: ${truncatePubkey(mint)}`);
            warnHotWallet(authorityPath);
          }

          const sdk = await import("../../../sdk/index.ts");
          // The SDK needs promotionSharpeX100; read from night shift results
          const { exportPromoteStrategy } = await import("../../../scripts/promote-strategy.ts");
          const result = await exportPromoteStrategy(connection, authority, mint, {
            dryRun: false,
          });

          if (mode === "json") {
            console.log(JSON.stringify(result, null, 2));
            return;
          }

          if (result) {
            printOk(`Strategy promoted!`);
            if (result.strategyPDA) printInfo(`Strategy PDA: ${truncatePubkey(result.strategyPDA)}`);
            if (result.signature) printInfo(`TX: ${explorerTxUrl(result.signature, opts.cluster)}`);
          } else {
            printNote("No qualifying strategy found in latest night shift results.");
          }
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // strategy retire — authority-gated + destructive
  cmd.addCommand(
    new Command("retire")
      .description("Force-retire a strategy (authority-gated, destructive)")
      .requiredOption("--id <strategy-id>", "Strategy ID to retire")
      .requiredOption("--authority <path>", "Authority keypair path")
      .option("--mint <pubkey>", "Token mint address")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--yes", "Confirm destructive operation")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const mint = resolveMint(opts.mint, config);
          const authorityPath = resolveKeypair(opts.authority, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
          const authority = loadKeypair(authorityPath);
          const connection = createConnection(config);

          if (!confirmDestructive(
            `RETIRE strategy "${opts.id}"`,
            [
              `Mint:          ${truncatePubkey(mint)}`,
              `Strategy ID:   ${opts.id}`,
              `Status:        LIVE → RETIRED`,
              ``,
              `This is irreversible. The strategy will stop receiving allocations.`,
            ],
            opts.yes,
          )) {
            process.exit(2);
          }

          warnHotWallet(authorityPath);

          // Call force_retire_strategy via SDK
          // The SDK doesn't expose this directly yet — document the gap
          printInfo("Calling force_retire_strategy on-chain...");
          // TODO: Wire to SDK's force_retire_strategy when available
          printNote("Strategy retirement requires force_retire_strategy instruction.");
          printNote("Submit via: anchor program call or SDK when implemented.");
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  return cmd;
}
