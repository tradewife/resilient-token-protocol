// RTP CLI — rtp accounts: Derive PDAs and show treasury state.

import { Command } from "commander";
import Table from "cli-table3";
import chalk from "chalk";

import { loadConfig, resolveMint } from "../config.js";
import { truncatePubkey, formatSol, formatSolFull } from "../keypair.js";
import { printOk, printInfo, getOutputMode } from "../format.js";
import { printError } from "../errors.js";
import { createConnection } from "../lib/rpc.js";

export function makeAccountsCommand(): Command {
  const cmd = new Command("accounts")
    .description("Derive PDAs and show treasury state");

  // accounts derive — offline, read-only
  cmd.addCommand(
    new Command("derive")
      .description("Derive all Flash Trade + treasury PDAs offline")
      .requiredOption("--mint <pubkey>", "Token mint address")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const mint = resolveMint(opts.mint, config);
          const PublicKey = (await import("@solana/web3.js")).PublicKey;
          const mintPk = new PublicKey(mint);

          const sdk = await import("../../../sdk/index.ts");
          const [treasuryPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("treasury"), mintPk.toBuffer()],
            new PublicKey("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"),
          );
          const [vaultPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("treasury"), mintPk.toBuffer(), Buffer.from("vault")],
            new PublicKey("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"),
          );

          // Flash Trade PDAs
          const { exportDeriveAccounts } = await import("../../../scripts/derive_flash_accounts.ts");
          const flashAccounts = exportDeriveAccounts(treasuryPda, opts.cluster === "mainnet" ? "mainnet" : "devnet");

          if (mode === "json") {
            console.log(JSON.stringify({
              mint,
              cluster: opts.cluster,
              treasuryPda: treasuryPda.toBase58(),
              vaultPda: vaultPda.toBase58(),
              ...flashAccounts,
            }, null, 2));
            return;
          }

          const table = new Table({
            head: ["Account", "Address"],
            style: { head: ["cyan"] },
          });

          table.push(["Mint", truncatePubkey(mint)]);
          table.push(["Treasury PDA", truncatePubkey(treasuryPda)]);
          table.push(["Vault PDA", truncatePubkey(vaultPda)]);

          if (flashAccounts) {
            table.push(["Flash Program", truncatePubkey(flashAccounts.programId ?? "")]);
            table.push(["Transfer Authority", truncatePubkey(flashAccounts.transferAuthority ?? "")]);
            table.push(["Event Authority", truncatePubkey(flashAccounts.eventAuthority ?? "")]);
            if (flashAccounts.markets) {
              for (const m of flashAccounts.markets) {
                table.push([`Position PDA (${m.symbol})`, truncatePubkey(m.positionPda ?? "")]);
              }
            }
          }

          console.log(table.toString());
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // accounts show — live on-chain state
  cmd.addCommand(
    new Command("show")
      .description("Fetch and display live treasury account state")
      .requiredOption("--mint <pubkey>", "Token mint address")
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

          const sdk = await import("../../../sdk/index.ts");
          const state = await sdk.fetchTreasuryState(connection, mintPk);

          if (mode === "json") {
            console.log(JSON.stringify({
              mint,
              balance: state.vaultBalance,
              isFrozen: state.isFrozen,
              phase: state.phase,
              totalFeesReceived: state.totalFeesReceived,
              totalFeesWithdrawn: state.totalFeesWithdrawn,
              totalDistributedHolders: state.totalDistributedHolders,
              totalDistributedDev: state.totalDistributedDev,
              totalDistributedEcosystem: state.totalDistributedEcosystem,
              totalHydration: state.totalHydration,
              minRunwayBalance: state.minRunwayBalance,
            }, null, 2));
            return;
          }

          const table = new Table({
            head: ["Field", "Value"],
            style: { head: ["cyan"] },
          });

          table.push(["Mint", truncatePubkey(mint)]);
          table.push(["Balance", `${formatSol(state.vaultBalance)} SOL`]);
          table.push(["Frozen", state.isFrozen ? chalk.red("YES") : chalk.green("NO")]);
          table.push(["Phase", state.phase]);
          table.push(["Total Fees Received", formatSol(state.totalFeesReceived)]);
          table.push(["Total Fees Withdrawn", formatSol(state.totalFeesWithdrawn)]);
          table.push(["Distributed (Holders)", formatSol(state.totalDistributedHolders)]);
          table.push(["Distributed (Dev)", formatSol(state.totalDistributedDev)]);
          table.push(["Distributed (Ecosystem)", formatSol(state.totalDistributedEcosystem)]);
          table.push(["Total Hydration", formatSol(state.totalHydration)]);
          table.push(["Min Runway Balance", formatSol(state.minRunwayBalance)]);

          console.log(table.toString());
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  return cmd;
}
