// RTP CLI — Main program setup, register all commands.

import { Command } from "commander";

import { makeInitCommand } from "./commands/init.js";
import { makeDeployCommand } from "./commands/deploy.js";
import { makeRegisterCommand } from "./commands/register.js";
import { makeCrankCommand } from "./commands/crank.js";
import { makeStrategyCommand } from "./commands/strategy.js";
import { makeFreezeCommand, makeUnfreezeCommand } from "./commands/freeze.js";
import { makeAccountsCommand } from "./commands/accounts.js";
import { makeStatusCommand } from "./commands/status.js";
import { makeDemoCommand } from "./commands/demo.js";
import { makePositionsCommand } from "./commands/positions.js";
import { printError } from "./errors.js";

export function createProgram(): Command {
  const program = new Command();

  program
    .name("rtp")
    .description("Operator CLI for the Resilient Token Protocol")
    .version("0.1.0")
    .exitOverride();

  program.addCommand(makeInitCommand());
  program.addCommand(makeDeployCommand());
  program.addCommand(makeRegisterCommand());
  program.addCommand(makeCrankCommand());
  program.addCommand(makeStrategyCommand());
  program.addCommand(makeFreezeCommand());
  program.addCommand(makeUnfreezeCommand());
  program.addCommand(makeAccountsCommand());
  program.addCommand(makeStatusCommand());
program.addCommand(makePositionsCommand());
  program.addCommand(makeDemoCommand());

  return program;
}
