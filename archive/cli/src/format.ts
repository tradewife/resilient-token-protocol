// RTP CLI — Output formatting utilities.

import chalk from "chalk";

export type OutputMode = "human" | "json" | "quiet";

export function getOutputMode(opts: { json?: boolean; quiet?: boolean }): OutputMode {
  if (opts.json) return "json";
  if (opts.quiet) return "quiet";
  return "human";
}

export function printStep(stepNum: number, total: number, title: string): void {
  console.log(
    `\n${chalk.bold(`Step ${stepNum}/${total}  ${title}`)}\n` +
    chalk.dim("─".repeat(50)),
  );
}

export function printOk(msg: string): void {
  console.log(`  ${chalk.green("✓")} ${msg}`);
}

export function printInfo(msg: string): void {
  console.log(`  ${chalk.cyan("→")} ${msg}`);
}

export function printNote(msg: string): void {
  console.log(`  ${chalk.dim(msg)}`);
}

export function printWarn(msg: string): void {
  console.log(`  ${chalk.yellow("⚠")} ${msg}`);
}

export function printBanner(title: string): void {
  console.log(
    `\n${chalk.cyan("═".repeat(60))}\n` +
    chalk.cyan(`  ${title}\n`) +
    chalk.cyan("═".repeat(60)) + "\n",
  );
}

export function printJson(data: unknown): void {
  console.log(JSON.stringify(data, null, 2));
}
