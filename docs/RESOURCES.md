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

### Phantom Connect + CASH + MCP
- Docs: https://docs.phantom.com/phantom-connect
- Get Started: https://phantom.app/phantom-connect
- React Template: https://github.com/phantom-labs/phantom-connect-react
- JS Template: https://github.com/phantom-labs/phantom-connect-js
- CASH stablecoin: https://phantom.app/cash (third-party, not currently used — treasury uses USDC)
- **Phantom MCP Server** (v1.2.x, 28+ tools — swap, sign, perps trading, yield distribution, balance queries): https://help.phantom.com/hc/en-us/articles/49235725504147
- MCP changelog: https://docs.phantom.com/updates
- **Phantom × Hyperliquid native perps** (UI feature only — NOT a programmatic API):
  https://unchainedcrypto.com/phantom-wallet-launches-direct-perpetual-trading-with-hyperliquid/
- **Per-token wallet isolation**: `derivationIndex` parameter gives each token its own wallet (Solana + EVM + HL account) from a single MCP auth session. Verified live with 3 separate indices.

> **RTP integration note (corrected Apr 11):** Phantom × HL native perps is a wallet UI feature,
> not a programmatic API. RTP's Hyperliquid execution uses an ETH keypair + EIP-712 signing
> directly in `trading/mod.rs` — this is the correct and final architecture for HL order placement.
> Phantom's role in RTP is Solana treasury signing (CPI transfer), not HL trading.
> For the devnet demo, a local keypair signs the treasury deposit tx. The signing cascade:
> Phantom KMS (production) → local devnet keypair (demo) → manual submission (fallback).

### Colosseum Sponsored Tools

### Squads Multisig
- Docs: https://docs.squads.so
- Get Started: https://squads.so
- Altitude (financial ops): https://altitude.finance

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
| Phantom × HL native perps for execution | UI feature only, not a programmatic API. HL execution uses ETH keypair + EIP-712 in `trading/mod.rs` directly. |
| Hyperliquid via Arbitrum bridge | Not needed — HL testnet API is accessed directly via REST. No bridge or EVM wallet needed for programmatic orders. |
| Phantom MCP for treasury signing | Partially implemented — MCP handles swaps, bridge, perps, yield distribution. Treasury CPI signing still uses local devnet keypair for demo. Production KMS path via Phantom Portal. |
| World Coin | Toxic sentiment — skip entirely |
| Privy | Not yet available |
| Coinbase | Not yet available |
