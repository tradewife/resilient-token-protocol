// RTP CLI — rtp init: Interactive onboarding wizard.

import { Command } from "commander";
import fs from "fs";
import path from "path";
import os from "os";
import chalk from "chalk";
import { createRequire } from "module";

const require = createRequire(import.meta.url);
// Inquirer v12 uses ESM default export
import inquirer from "inquirer";

import { saveConfig, GLOBAL_CONFIG_PATH_EXPORT, type RtpConfig } from "../config.js";
import { loadKeypair, truncatePubkey, formatSol } from "../keypair.js";
import { printBanner, printOk, printWarn, printNote } from "../format.js";

interface PrerequisiteResult {
  name: string;
  found: boolean;
  version: string;
  installUrl?: string;
}

async function checkPrerequisites(): Promise<PrerequisiteResult[]> {
  const checks: PrerequisiteResult[] = [];

  // Solana CLI
  try {
    const { execSync } = await import("child_process");
    const ver = execSync("solana --version 2>/dev/null").toString().trim();
    checks.push({ name: "Solana CLI", found: true, version: ver.split(" ").pop() ?? ver });
  } catch {
    checks.push({ name: "Solana CLI", found: false, version: "-", installUrl: "https://docs.solanalabs.com/cli/install" });
  }

  // Anchor CLI
  try {
    const { execSync } = await import("child_process");
    const ver = execSync("anchor --version 2>/dev/null").toString().trim();
    checks.push({ name: "Anchor CLI", found: true, version: ver.split(" ").pop() ?? ver });
  } catch {
    checks.push({ name: "Anchor CLI", found: false, version: "-", installUrl: "https://www.anchor-lang.com/docs/installation" });
  }

  // Node.js
  try {
    const { execSync } = await import("child_process");
    const ver = execSync("node --version").toString().trim();
    const major = parseInt(ver.replace("v", "").split(".")[0], 10);
    checks.push({ name: "Node.js", found: major >= 18, version: ver, installUrl: major < 18 ? "https://nodejs.org/en/download" : undefined });
  } catch {
    checks.push({ name: "Node.js", found: false, version: "-", installUrl: "https://nodejs.org/en/download" });
  }

  // Rust
  try {
    const { execSync } = await import("child_process");
    const ver = execSync("rustc --version").toString().trim();
    checks.push({ name: "Rust", found: true, version: ver.split(" ").pop() ?? ver });
  } catch {
    checks.push({ name: "Rust", found: false, version: "-", installUrl: "https://rustup.rs" });
  }

  return checks;
}

export function makeInitCommand(): Command {
  return new Command("init")
    .description("Interactive onboarding wizard for first-time setup")
    .action(async () => {
      printBanner("RTP Operator Setup");

      // Step 1: Prerequisites
      console.log(chalk.bold("  Prerequisites\n"));
      const prereqs = await checkPrerequisites();
      let allOk = true;
      for (const p of prereqs) {
        if (p.found) {
          console.log(`  ${chalk.green("✓")} ${p.name.padEnd(14)} ${p.version}`);
        } else {
          console.log(`  ${chalk.red("✗")} ${p.name.padEnd(14)} ${p.version}`);
          if (p.installUrl) console.log(`    Install: ${chalk.cyan(p.installUrl)}`);
          allOk = false;
        }
      }
      if (!allOk) {
        console.log(`\n${chalk.red("  Install missing prerequisites and re-run 'rtp init'.")}\n`);
        process.exit(1);
      }
      console.log(`\n  ${chalk.green("All prerequisites met.")}`);

      // Step 2: Cluster selection
      console.log(`\n${chalk.bold("  Cluster Selection")}\n`);
      const { cluster } = await inquirer.prompt<{ cluster: "devnet" | "mainnet" }>([
        {
          type: "list",
          name: "cluster",
          message: "Select cluster:",
          choices: [
            { name: "devnet", value: "devnet" },
            { name: "mainnet", value: "mainnet" },
          ],
        },
      ]);

      if (cluster === "devnet") {
        printWarn("Flash Trade uses Pyth oracles — position CPI will fail with");
        printNote("  StaleOraclePrice on devnet. Constraint logic tests work.");
        printNote("  Use mainnet for full CPI execution.");
      } else {
        printWarn("Minimum Flash Trade position: ~$11-12 USDC.");
        printNote("  Your fee-payer wallet needs SOL for gas.");
      }

      // Step 3: Fee-payer wallet
      console.log(`\n${chalk.bold("  Fee-Payer Wallet")} ${chalk.dim("(gas only, < 0.01 SOL needed)")}\n`);
      const { feePayerPath } = await inquirer.prompt<{ feePayerPath: string }>([
        {
          type: "input",
          name: "feePayerPath",
          message: "Path to Solana keypair JSON:",
          default: path.join(os.homedir(), ".config", "solana", "id.json"),
        },
      ]);

      let feePayerPubkey = "";
      let feePayerBalance = 0;
      try {
        const kp = loadKeypair(feePayerPath);
        feePayerPubkey = kp.publicKey.toBase58();
        printOk(`Keypair loaded: ${feePayerPubkey}`);

        // Try to check balance
        try {
          const { Connection } = await import("@solana/web3.js");
          const rpcUrl = cluster === "mainnet"
            ? "https://api.mainnet-beta.solana.com"
            : "https://api.devnet.solana.com";
          const conn = new Connection(rpcUrl, "confirmed");
          feePayerBalance = await conn.getBalance(kp.publicKey);
          printOk(`Balance: ${formatSol(feePayerBalance)} SOL`);
          if (feePayerBalance < 10_000_000) {
            if (cluster === "devnet") {
              printWarn(`Balance below minimum. Run: solana airdrop 0.05 ${feePayerPubkey} --url devnet`);
            } else {
              printWarn(`Balance below minimum. Transfer SOL to: ${feePayerPubkey}`);
            }
          }
        } catch {
          printNote("Could not check balance (offline or RPC unreachable)");
        }

        printNote("This wallet pays transaction gas only. It has zero authority over");
        printNote("treasury funds — the PDA owns everything.");
      } catch (e) {
        printWarn(`Could not load keypair: ${(e as Error).message}`);
      }

      // Step 4: Authority wallet
      console.log(`\n${chalk.bold("  Authority Wallet")} ${chalk.dim("(gates all privileged operations)")}\n`);
      const { authorityPath } = await inquirer.prompt<{ authorityPath: string }>([
        {
          type: "input",
          name: "authorityPath",
          message: "Path to authority keypair JSON:",
          default: feePayerPath,
        },
      ]);

      let authorityPubkey = "";
      try {
        const kp = loadKeypair(authorityPath);
        authorityPubkey = kp.publicKey.toBase58();
        printOk(`Keypair loaded: ${authorityPubkey}`);
        printNote("This keypair controls:");
        printNote("  • initialize (treasury deployment)");
        printNote("  • evolve_phase (irreversible phase transitions)");
        printNote("  • register_strategy (promote to Live)");
        printNote("  • force_retire_strategy (emergency retirement)");
        printNote("  • freeze_treasury / unfreeze_treasury (emergency halt/resume)");
        printNote("  • open_flash_position / close_flash_position (Flash Trade CPI)");
        printNote("  • emergency_close_all_positions (unwind all positions)");
      } catch (e) {
        printWarn(`Could not load keypair: ${(e as Error).message}`);
      }

      // Step 5: Default mint (optional)
      console.log(`\n${chalk.bold("  Default Mint")} ${chalk.dim("(optional — press Enter to skip)")}\n`);
      const { defaultMint } = await inquirer.prompt<{ defaultMint: string }>([
        {
          type: "input",
          name: "defaultMint",
          message: "Default token mint pubkey:",
          default: "",
        },
      ]);

      if (defaultMint) {
        try {
          const { PublicKey } = await import("@solana/web3.js");
          const mint = new PublicKey(defaultMint);
          printOk(`Deriving PDAs for mint: ${truncatePubkey(mint)}`);
          try {
            const [treasuryPda] = PublicKey.findProgramAddressSync(
              [Buffer.from("treasury"), mint.toBuffer()],
              new PublicKey("8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"),
            );
            printOk(`Treasury PDA: ${truncatePubkey(treasuryPda)}`);
          } catch {
            printNote("Could not derive PDAs (SDK import failed)");
          }
        } catch {
          printWarn("Invalid mint pubkey — skipping PDA derivation");
        }
      }

      // Step 6: Railway token (optional)
      console.log(`\n${chalk.bold("  Railway Integration")} ${chalk.dim("(optional — press Enter to skip)")}\n`);
      const { railwayTokenPath } = await inquirer.prompt<{ railwayTokenPath: string }>([
        {
          type: "input",
          name: "railwayTokenPath",
          message: "Path to Railway workspace token:",
          default: ".secrets/railway-workspace-token",
        },
      ]);

      if (railwayTokenPath) {
        if (fs.existsSync(railwayTokenPath)) {
          printOk("Token file found.");
        } else {
          printNote("Token file not found. You can set this up later in ~/.rtp/config.json.");
          printNote("Generate a token at: railway.com/account/tokens");
        }
      }

      // Step 7: Write config
      const config: RtpConfig = {
        cluster,
        feePayerKeypairPath: feePayerPath,
        authorityKeypairPath: authorityPath,
        defaultMint: defaultMint || null,
        rpcUrl: null,
        railwayTokenPath: railwayTokenPath || null,
        nightResultsDir: "./data/night_results",
      };

      saveConfig(config, "global");
      console.log(`\n${chalk.green("✓")} Configuration written to ${GLOBAL_CONFIG_PATH_EXPORT}\n`);
      console.log(JSON.stringify(config, null, 2));

      // Step 8: Next steps
      console.log(`\n${chalk.bold("  ━━━ Setup Complete ━━━")}\n`);
      console.log("  Next steps:");
      console.log("    1. Read the architecture:       cat CLAUDE.md");
      console.log("    2. Read governance invariants:  cat SOULCONTRACT.md");
      console.log("    3. Read the Flash Trade CPI:    cat FLASHTRADE-PDA-UPGRADE-SPEC.md");
      console.log("    4. Read the demo script:        cat docs/demo-flow.md");
      console.log("    5. Check protocol status:       npx tsx cli/bin/rtp.ts status --all");
      console.log("    6. Run the demo (dry-run):      npx tsx cli/bin/rtp.ts demo\n");
    });
}
