// RTP CLI — rtp status: Protocol health and Railway services.

import { Command } from "commander";
import Table from "cli-table3";
import chalk from "chalk";
import fs from "fs";

import { loadConfig, resolveMint } from "../config.js";
import { truncatePubkey, formatSol } from "../keypair.js";
import { printOk, printInfo, printNote, printWarn, printBanner, getOutputMode } from "../format.js";
import { printError } from "../errors.js";
import { createConnection } from "../lib/rpc.js";
import { loadRailwayToken, fetchServiceStatus, type RailwayService } from "../lib/railway.js";

export function makeStatusCommand(): Command {
  const cmd = new Command("status")
    .description("Display protocol health overview");

  // rtp status
  cmd
    .option("--mint <pubkey>", "Token mint address")
    .option("--all", "Show status for all known treasuries")
    .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
    .option("--json", "JSON output")
    .option("--quiet", "Suppress output except errors")
    .action(async (opts) => {
      const mode = getOutputMode(opts);
      try {
        const config = loadConfig();
        const connection = createConnection(config);
        const sdk = await import("../../../sdk/index.ts");
        const PublicKey = (await import("@solana/web3.js")).PublicKey;

        const mintStr = resolveMint(opts.mint, config);
        const mintPk = new PublicKey(mintStr);

        // Treasury state
        const state = await sdk.fetchTreasuryState(connection, mintPk);

        // Night shift results
        let nightShiftInfo: Record<string, unknown> | null = null;
        const summaryPath = findLatestSummary(config.nightResultsDir);
        if (summaryPath) {
          nightShiftInfo = JSON.parse(fs.readFileSync(summaryPath, "utf-8"));
        }

        // Devnet cycle data
        let cycleInfo: Record<string, unknown> | null = null;
        const cyclePath = "data/devnet-cycles/latest/cycle.json";
        if (fs.existsSync(cyclePath)) {
          cycleInfo = JSON.parse(fs.readFileSync(cyclePath, "utf-8"));
        }

        if (mode === "json") {
          console.log(JSON.stringify({
            mint: mintStr,
            treasury: state,
            nightShift: nightShiftInfo ? { date: nightShiftInfo.date } : null,
            devnetCycle: cycleInfo ? { mutations: cycleInfo.mutations_accepted } : null,
          }, null, 2));
          return;
        }

        printBanner("RTP Protocol Status");

        const table = new Table({
          head: ["Metric", "Value"],
          style: { head: ["cyan"] },
        });

        table.push(["Mint", truncatePubkey(mintStr)]);
        table.push(["Cluster", config.cluster]);
        table.push(["Frozen", state.isFrozen ? chalk.red("YES") : chalk.green("NO")]);
        table.push(["Phase", state.phase ?? "Sustenance"]);
        table.push(["Vault Balance", `${formatSol(state.vaultBalance)} SOL`]);
        table.push(["Open Positions", "N/A (fetch on-chain)"]);
        table.push(["Strategy", "N/A (fetch on-chain)"]);

        if (nightShiftInfo) {
          table.push(["Last Night Shift", String(nightShiftInfo.date ?? "unknown")]);
          const candidates = nightShiftInfo.top_candidates as Array<Record<string, unknown>> ?? [];
          table.push(["Night Shift Candidates", String(candidates.length)]);
        } else {
          table.push(["Last Night Shift", chalk.dim("no results")]);
        }

        if (cycleInfo) {
          table.push(["Last Devnet Cycle", String(cycleInfo.timestamp ?? "unknown")]);
        }

        console.log(table.toString());
      } catch (e) {
        printError(e);
        process.exit(1);
      }
    });

  // rtp status services
  cmd.addCommand(
    new Command("services")
      .description("Show Railway service status")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const token = loadRailwayToken(config.railwayTokenPath);

          if (!token) {
            printWarn("Railway token not found. Set railwayTokenPath in config or run 'rtp init'.");
            printNote("Generate a token at: railway.com/account/tokens");
            process.exit(1);
          }

          const services = await fetchServiceStatus(token);

          if (mode === "json") {
            console.log(JSON.stringify(services, null, 2));
            return;
          }

          const table = new Table({
            head: ["Service", "Status", "Cron", "URL"],
            style: { head: ["cyan"] },
          });

          for (const svc of services) {
            const statusIcon = svc.status === "SUCCESS" ? chalk.green("✓ deployed")
              : svc.status === "BUILDING" ? chalk.yellow("⟳ building")
              : svc.status === "CRASHED" ? chalk.red("✗ crashed")
              : chalk.dim(svc.status);
            table.push([
              svc.name,
              statusIcon,
              svc.cronSchedule ?? chalk.dim("—"),
              svc.url ?? chalk.dim("—"),
            ]);
          }

          console.log(table.toString());
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  return cmd;
}

function findLatestSummary(resultsDir: string): string | null {
  if (!fs.existsSync(resultsDir)) return null;
  const dateDirs = fs.readdirSync(resultsDir)
    .filter(d => /^\d{4}-\d{2}-\d{2}$/.test(d))
    .sort()
    .reverse();
  for (const d of dateDirs) {
    const p = `${resultsDir}/${d}/summary.json`;
    if (fs.existsSync(p)) return p;
  }
  return null;
}
