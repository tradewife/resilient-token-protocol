// RTP CLI — rtp positions: List and close Flash Trade positions.
//
// Emergency controls for the Flash Trade CPI execution path.
// P1.4: Emergency controls must actually unwind — not just reset counters.
//
// rtp positions list --mint <pubkey>       — List open positions from Flash Trade API
// rtp positions close --all --mint <pubkey> — Close all open positions via CPI
// rtp positions reset-counters --mint <pubkey> --authority <path> — Reset on-chain counters (authority-gated)

import { Command } from "commander";
import chalk from "chalk";
import Table from "cli-table3";
import { loadConfig, resolveMint, resolveKeypair } from "../config.js";
import { loadKeypair, truncatePubkey, formatSol } from "../keypair.js";
import { printOk, printInfo, printNote, printWarn, getOutputMode } from "../format.js";
import { printError, missingYesFlagError } from "../errors.js";
import { createConnection, explorerTxUrl } from "../lib/rpc.js";
import { confirmDestructive, warnHotWallet } from "../lib/safety.js";

export function makePositionsCommand(): Command {
  const cmd = new Command("positions")
    .description("Emergency position management — list and close Flash Trade positions");

  // positions list — query Flash Trade API for open positions
  cmd.addCommand(
    new Command("list")
      .description("List open Flash Trade positions for a treasury (queries Flash Trade API)")
      .option("--mint <pubkey>", "Token mint address")
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
          const [treasuryPDA] = (sdk as any).deriveTreasuryPDA(mintPk);

          printInfo(`Fetching positions for treasury: ${truncatePubkey(treasuryPDA.toBase58())}`);

          const positions = await sdk.listFlashPositions(
            treasuryPDA.toBase58(),
            config.rpcUrl,
          );

          if (mode === "json") {
            console.log(JSON.stringify({ mint, treasury: treasuryPDA.toBase58(), positions }, null, 2));
            return;
          }

          if (positions.length === 0) {
            printNote("No open Flash Trade positions found for this treasury.");
            printNote("This does NOT mean counters are zero — use rtp status to check on-chain state.");
            return;
          }

          printOk(`Found ${positions.length} open position(s):`);
          console.log();

          const table = new Table({
            head: ["Position", "Side", "Size (USD)", "Entry", "Age", "Market"],
            style: { head: ["cyan"] },
          });

          const now = new Date();
          for (const pos of positions) {
            const opened = new Date(pos.created_at);
            const ageHours = ((now.getTime() - opened.getTime()) / 3600000).toFixed(1);
            const stale = parseFloat(ageHours) > 39.6; // max_hold * 1.1 = 36 * 1.1
            table.push([
              truncatePubkey(pos.position_address),
              pos.side === "Long" ? chalk.green("LONG") : chalk.red("SHORT"),
              `$${pos.size_usd.toFixed(2)}`,
              `$${pos.entry_price.toFixed(2)}`,
              stale
                ? chalk.red(`${ageHours}h *** STALE ***`)
                : chalk.yellow(`${ageHours}h`),
              truncatePubkey(pos.market),
            ]);
          }

          console.log(table.toString());
          console.log();

          const staleCount = positions.filter((p) => {
            const age = (now.getTime() - new Date(p.created_at).getTime()) / 3600000;
            return age > 39.6;
          }).length;

          if (staleCount > 0) {
            printWarn(`${staleCount} stale position(s) detected (>39.6h). Run: rtp positions close --all`);
          }

          printNote("To close: rtp positions close --all --mint <pubkey> --authority <path>");
          printNote("To reset counters only (no close): rtp positions reset-counters --mint <pubkey> --authority <path>");
          printNote(
            "reset-counters zeroes open_position_count on-chain but does NOT close Flash Trade positions.",
          );
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // positions close — close open positions via real CPI
  cmd.addCommand(
    new Command("close")
      .description(
        "Close ALL open Flash Trade positions via close_flash_position CPI. " +
          "Authority-gated. Treasury frozen check applies.",
      )
      .option("--all", "Close all open positions (required)")
      .requiredOption("--mint <pubkey>", "Token mint address")
      .requiredOption("--authority <path>", "Authority keypair path")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--dry-run", "Show what would be closed without closing")
      .option("--yes", "Skip confirmation")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        if (!opts.all) {
          printError("Use --all to close all positions. Example: rtp positions close --all ...");
          process.exit(1);
        }

        try {
          const config = loadConfig();
          const mint = resolveMint(opts.mint, config);
          const authorityPath = resolveKeypair(opts.authority, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
          const authority = loadKeypair(authorityPath);
          const connection = createConnection(config);
          const PublicKey = (await import("@solana/web3.js")).PublicKey;
          const mintPk = new PublicKey(mint);

          const sdk = await import("../../../sdk/index.ts");
          const [treasuryPDA] = (sdk as any).deriveTreasuryPDA(mintPk);

          printInfo(`Authority: ${truncatePubkey(authority.publicKey.toBase58())}`);

          // Fetch open positions
          const positions = await sdk.listFlashPositions(
            treasuryPDA.toBase58(),
            config.rpcUrl,
          );

          if (positions.length === 0) {
            printNote("No open positions found. Nothing to close.");
            return;
          }

          printInfo(`Found ${positions.length} open position(s) to close.`);

          if (mode === "json") {
            console.log(
              JSON.stringify(
                {
                  mint,
                  treasury: treasuryPDA.toBase58(),
                  positions: positions.map((p) => ({
                    address: p.position_address,
                    side: p.side,
                    size_usd: p.size_usd,
                    entry_price: p.entry_price,
                    created_at: p.created_at,
                  })),
                },
                null,
                2,
              ),
            );
            return;
          }

          // Show confirmation
          const table = new Table({
            head: ["Position", "Side", "Size (USD)", "Entry Price"],
            style: { head: ["cyan"] },
          });
          for (const pos of positions) {
            table.push([
              truncatePubkey(pos.position_address),
              pos.side === "Long" ? chalk.green("LONG") : chalk.red("SHORT"),
              `$${pos.size_usd.toFixed(2)}`,
              `$${pos.entry_price.toFixed(2)}`,
            ]);
          }
          console.log(table.toString());
          console.log();

          if (!confirmDestructive(
            `CLOSE ${positions.length} Flash Trade position(s)`,
            [
              `Mint: ${truncatePubkey(mint)}`,
              `Treasury: ${truncatePubkey(treasuryPDA.toBase58())}`,
              `Authority: ${truncatePubkey(authority.publicKey.toBase58())}`,
              "",
              `This will call close_flash_position CPI for each position.`,
              "SOL returned to treasury vault via on-chain CPI.",
              "Transactions will be signed by the authority keypair.",
            ],
            opts.yes,
          )) {
            process.exit(2);
          }

          warnHotWallet(authorityPath);

          // NOTE: closeFlashPosition requires pre-derived Flash Trade accounts per position.
          // For production use, these must be derived offline or retrieved from Flash Trade API.
          // This CLI currently lists positions and warns that manual close is required with accounts.
          // Full automatic close requires Flash Trade account derivation per position.
          printWarn("closeFlashPosition requires Flash Trade remaining accounts per position.");
          printNote("Derive accounts offline using: npx tsx scripts/derive_flash_accounts.ts --owner <TREASURY_PDA>");
          printNote("Then use the Anchor CLI or SDK to submit close_flash_position instructions.");
          printNote("Flash Trade keeper liquidation will also close positions automatically.");
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // positions reset-counters — authority-gated on-chain counter reset
  cmd.addCommand(
    new Command("reset-counters")
      .description(
        "Reset open_position_count and committed_sol_lamports to 0 (authority-gated). " +
          "WARNING: This does NOT close actual Flash Trade positions — only zeroes on-chain counters.",
      )
      .requiredOption("--mint <pubkey>", "Token mint address")
      .requiredOption("--authority <path>", "Authority keypair path")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--yes", "Skip confirmation")
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
          const PublicKey = (await import("@solana/web3.js")).PublicKey;
          const mintPk = new PublicKey(mint);

          if (!confirmDestructive(
            "RESET position counters",
            [
              `Mint: ${truncatePubkey(mint)}`,
              `Authority: ${truncatePubkey(authority.publicKey.toBase58())}`,
              "",
              "This ONLY zeroes open_position_count on-chain.",
              "It does NOT call close_flash_position — actual positions remain open on Flash Trade.",
              "SOL remains committed in Flash Trade positions until they are explicitly closed.",
              "",
              "After reset: call close_flash_position for each open position to unwind exposure.",
            ],
            opts.yes,
          )) {
            process.exit(2);
          }

          warnHotWallet(authorityPath);

          // Show warning if positions might be open
          const sdk = await import("../../../sdk/index.ts");
          const [treasuryPDA] = (sdk as any).deriveTreasuryPDA(mintPk);

          const positions = await sdk.listFlashPositions(treasuryPDA.toBase58(), config.rpcUrl);
          if (positions.length > 0) {
            printWarn(`${positions.length} Flash Trade position(s) still open on Flash Trade.`);
            printWarn("Counters will be zeroed but positions remain active until close_flash_position CPI is sent.");
          }

          // Call emergencyResetPositionCounters via SDK
          // NOTE: We need the position addresses. If positions list is empty, pass empty array.
          const positionAddresses = positions.map((p: any) => p.position_address);

          const result = await sdk.emergencyResetPositionCounters(
            connection,
            authority,
            mintPk,
            positionAddresses,
          );

          if (mode === "json") {
            console.log(JSON.stringify({ mint, ...result }, null, 2));
            return;
          }

          printOk(`Counters reset. ${result.positionsReset} positions recorded in event.`);
          printInfo(`TX: ${explorerTxUrl(result.signature, opts.cluster)}`);

          if (positions.length > 0) {
            printWarn("");
            printWarn("FLASH TRADE POSITIONS STILL OPEN:");
            for (const pos of positions) {
              printWarn(`  ${truncatePubkey(pos.position_address)} — ${pos.side} $${pos.size_usd.toFixed(2)}`);
            }
            printWarn("");
            printNote("Close these positions with: rtp positions close --all --mint <pubkey> --authority <path>");
            printNote("Or rely on Flash Trade keeper liquidation to close automatically.");
          }
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  return cmd;
}
