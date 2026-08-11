# RTP Operator CLI

Operator CLI for the Resilient Token Protocol. Consolidates all operational scripts into a single cohesive tool for protocol operators.

## Quick Start

```bash
# From repo root
npx tsx cli/bin/rtp.ts --help

# Interactive setup wizard
npx tsx cli/bin/rtp.ts init

# Derive PDAs (offline, no RPC needed)
npx tsx cli/bin/rtp.ts accounts derive --mint <MINT_PUBKEY>

# Check protocol status
npx tsx cli/bin/rtp.ts status --mint <MINT_PUBKEY>

# Run the full demo (dry-run by default)
npx tsx cli/bin/rtp.ts demo
```

## Commands

| Command | Description |
|---------|-------------|
| `rtp init` | Interactive onboarding wizard |
| `rtp deploy treasury` | Deploy treasury PDA for a new token |
| `rtp deploy program` | Build and deploy the Anchor program |
| `rtp register adopter` | Register adopter record (permissionless) |
| `rtp register strategy` | Promote strategy to Live (authority-gated) |
| `rtp crank fees` | Sweep fees into treasury PDA vault |
| `rtp crank redistribute` | Trigger 70/20/10 redistribution |
| `rtp strategy list` | List strategy records |
| `rtp strategy promote` | Promote validated strategy (authority-gated) |
| `rtp strategy retire` | Force-retire strategy (authority-gated, --yes required) |
| `rtp freeze` | Emergency freeze (authority-gated, --yes required) |
| `rtp unfreeze` | Resume operations (authority-gated, --yes required) |
| `rtp accounts derive` | Derive PDAs offline |
| `rtp accounts show` | Fetch live treasury state |
| `rtp status` | Protocol health overview |
| `rtp status services` | Railway service status |
| `rtp demo` | Full 8-step demonstration pipeline |

## Global Flags

- `--json` — machine-readable JSON output
- `--quiet` — suppress everything except errors
- `--cluster <devnet|mainnet>` — target cluster
- `--mint <pubkey>` — target token mint

## Configuration

Config file: `~/.rtp/config.json` (created by `rtp init`)

```json
{
  "cluster": "devnet",
  "feePayerKeypairPath": "~/.config/solana/id.json",
  "authorityKeypairPath": "~/.config/solana/id.json",
  "defaultMint": null,
  "rpcUrl": null,
  "railwayTokenPath": null,
  "nightResultsDir": "./data/night_results"
}
```

## Development

```bash
cd cli
npm install
npm run typecheck    # tsc --noEmit
npm test             # unit tests
npm run rtp -- --help # run CLI locally
```

## Trust Model

- **Authority-gated**: deploy, register strategy, freeze, unfreeze, strategy retire
- **Permissionless**: crank fees, crank redistribute, accounts derive/show, status

See `SOULCONTRACT.md` for the full trust model.
