# RTP — Resource Index

All canonical links for LLM sessions, development, and hackathon submission.
Extracted from BUILD_PLAN v2.2. Keep this file updated as new tools are integrated.

---

## Hackathon

| Resource | Link |
|---|---|
| Register (deadline May 4) | https://arena.colosseum.org/register |
| Rules | https://colosseum.com/legal/Solana%20Frontier%20Hackathon%20Rules.pdf |
| Resources page | https://colosseum.com/frontier/resources |
| Copilot (pressure-test vs 5400+ submissions) | https://arena.colosseum.org/copilot |

---

## Operator CLI

The `rtp` CLI (`cli/`) consolidates all operational scripts into a single Commander.js tool.

| Command | Description | Authority |
|---------|-------------|-----------|
| `rtp init` | Interactive onboarding wizard | None |
| `rtp deploy treasury` | Deploy treasury PDA for a new token | Authority-gated |
| `rtp deploy program` | Build and deploy Anchor program | Authority-gated |
| `rtp register adopter` | Register adopter record | Permissionless |
| `rtp register strategy` | Promote strategy to Live | Authority-gated |
| `rtp crank fees` | Sweep TransferFeeConfig fees | Permissionless |
| `rtp crank redistribute` | Trigger 70/20/10 split | Permissionless |
| `rtp strategy list` | List strategy records | Read-only |
| `rtp strategy promote` | Promote validated strategy | Authority-gated |
| `rtp strategy retire` | Force-retire strategy | Authority-gated + `--yes` |
| `rtp freeze` | Emergency freeze | Authority-gated + `--yes` |
| `rtp unfreeze` | Resume operations | Authority-gated + `--yes` |
| `rtp accounts derive` | Derive PDAs offline | Read-only |
| `rtp accounts show` | Fetch live treasury state | Read-only |
| `rtp status` | Protocol health overview | Read-only |
| `rtp status services` | Railway service status | Read-only |
| `rtp demo` | Full 8-step demo pipeline | `--execute` for live tx |

Usage: `npx tsx cli/bin/rtp.ts <command> [options]`
All commands support `--json`, `--quiet`, `--cluster <devnet|mainnet>`.
See `cli/README.md` for full documentation.

---

## Core Frameworks

| Framework | Link | Use in RTP |
|---|---|---|
| atlas-gic | https://github.com/chrisworsey55/atlas-gic | Multi-agent Darwinian loop → Evolve Wing |
| karpathy/autoresearch | https://github.com/karpathy/autoresearch | Modify/Verify/Keep loop spec |
| uditgoenka/autoresearch | https://github.com/uditgoenka/autoresearch | Claude-native autoresearch implementation |
| MetaClaw | https://github.com/aiming-lab/MetaClaw | Knowledge Wing + human override UI |
| revfactory/harness | https://github.com/revfactory/harness | Coordinator architecture reference |
| autoagent | https://github.com/kevinrgu/autoagent | Wing lifecycle scaffolding |

---

## Hackathon Tools & Integrations

### Flash Trade (Execution Venue)
- REST API: https://flashapi.trade
- SKILL.md (in repo): `flash-trade/SKILL.md`
- TransactionFlow: `flash-trade/TransactionFlow.md`
- ProtocolConcepts: `flash-trade/ProtocolConcepts.md`
- ErrorReference: `flash-trade/ErrorReference.md`
- Program (mainnet): `FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn`
- Program (devnet): `FTPP4jEWW1n8s2FEccwVfS9KCPjpndaswg7Nkkuz4ER4`
- Composability Program (mainnet): `FSWAPViR8ny5K96hezav8jynVubP2dJ2L7SbKzds2hwm`
- TypeScript SDK: `flash-sdk` (NPM package)
- **RTP integration:** CPI via `invoke_signed` from Treasury PDA. REST API for queries only (prices, positions, markets). Execution is CPI only — no REST API execution. Pyth oracle prices are mainnet-only (devnet has stale/zero prices).

### Phantom Connect + CASH (Browser Wallet)
- Docs: https://docs.phantom.com/phantom-connect
- Get Started: https://phantom.app/phantom-connect
- React Template: https://github.com/phantom-labs/phantom-connect-react
- JS Template: https://github.com/phantom-labs/phantom-connect-js
- CASH stablecoin: https://phantom.app/cash (third-party, not currently used — treasury uses USDC)
- **Phantom MCP Server** (v1.2.x, 28+ tools — swap, sign, perps trading, yield distribution, balance queries): https://help.phantom.com/hc/en-us/articles/49235725504147
- MCP changelog: https://docs.phantom.com/updates
- **[ARCHIVED]** Phantom MCP is gated behind `#[cfg(feature = "hyperliquid")]` in the Rust swarm. Not compiled by default. Available for legacy reference.
- **Phantom × Hyperliquid native perps** (UI feature only — NOT a programmatic API):
  https://unchainedcrypto.com/phantom-wallet-launches-direct-perpetual-trading-with-hyperliquid/

> **RTP integration note (updated Apr 28):** The execution venue is now Flash Trade (on-chain Solana CPI).
> The Treasury PDA signs via `invoke_signed` — no human keypair involved.
> Phantom's role in RTP is the browser wallet (dashboard freeze/unfreeze, wallet connect).
> The Hyperliquid/Phantom MCP execution path is archived behind a feature flag.
> Flash Trade handles all perps execution on Solana.

### Colosseum Sponsored Tools

### Squads Multisig
- Docs: https://docs.squads.so
- Get Started: https://squads.so
- Audits: OtterSec (3 rounds), Neodyme (3 rounds), Certora (3 rounds), Trail of Bits (1 round)
- Altitude (financial ops): https://altitude.finance
- Post-launch integration: `treasury.authority` rotation to Squads PDA for 2-of-3 multisig governance

### Swig (programmable smart wallets)
- Overview: https://docs.swig.fi/overview
- TypeScript SDK: https://docs.swig.fi/typescript-sdk
- TypeScript SDK Tutorial: https://docs.swig.fi/typescript-sdk/tutorial
- Rust SDK: https://docs.swig.fi/rust-sdk
- Developer Portal: https://portal.swig.fi

### MoonPay Agents
- Docs: https://www.moonpay.com/developers/agents
- Install: `npm install -g @moonpay/cli`
- Skills repo: https://github.com/moonpay/agents-skills

### Solana MCP
- Repo: https://github.com/solana-developers/solana-mcp
- Use: AI-powered dev assistant for Anchor programs

### Arcium (stretch goal — not yet integrated)
- Docs: https://docs.arcium.com
- Arcis Rust Framework: https://docs.arcium.com/arcis/getting-started
- Purple Paper: https://docs.arcium.com/resources/purple-paper

---

## Solana Development

### Start Here
- Intro: https://solana.com/developers/docs/intro
- Core Concepts: https://solana.com/developers/docs/core-concepts
- Setup: https://solana.com/developers/docs/setup
- Hello World: https://solana.com/developers/docs/hello-world

### Dev Tools
- Solana Playground (browser IDE): https://play.solana.com
- create-solana-dapp: https://github.com/solana-developers/create-solana-dapp
- Program Examples (Anchor, Rust, Python): https://github.com/solana-developers/program-examples
- Agent Skills: https://github.com/solana-developers/solana-agent-skills

### Anchor
- Intro: https://www.anchor-lang.com/docs/introduction
- Build a CRUD dApp: https://solana.com/developers/crud

### Guides
- Solana Cookbook: https://solanacookbook.com
- Bootcamp (7-hour): https://www.solana.com/developers/courses
- Solana Bytes (video): https://www.solana.com/developers/videos

### Token + Payment
- SPL Transfer Fees (TransferFeeConfig): https://solana.com/docs/tokens/extensions/transfer-fees
- Metaplex (NFTs): https://docs.metaplex.com
- Solana Pay: https://solanapay.com
- Blinks: https://solana.com/docs/advanced/blinks

### Governance / DAOs
- Realms: https://docs.realms.today
- Cubik (quadratic funding for Phase 3): https://solanacompass.com/projects/cubik

### RPC
- Triton One (free private devnet/testnet): https://triton.one

---

## RTP Repos

| Repo | Link |
|---|---|
| Main (this repo) | https://github.com/tradewife/resilient-token-protocol |
| Python research layer | `git@github.com:tradewife/fractal-swarm.git` (private) |
| Skills research | `git@github.com:tradewife/rtp-skills-research.git` (private) |

---

## NOT Using (and why)

| Tool | Reason |
|---|---|
| Hyperliquid for execution | **Archived.** Replaced by Flash Trade on-chain CPI. HL path gated behind `#[cfg(feature = "hyperliquid")]`. |
| Phantom MCP for execution | **Archived.** Replaced by Treasury PDA invoke_signed. MCP module not compiled by default. |
| Phantom × HL native perps for execution | UI feature only, not a programmatic API. Flash Trade provides on-chain CPI instead. |
| World Coin | Toxic sentiment — skip entirely |
| Privy | Not yet available |
| Coinbase | Not yet available |
