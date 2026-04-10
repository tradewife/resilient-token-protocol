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

## Sponsored Hackathon Tools

### Phantom Connect + CASH
- Docs: https://docs.phantom.app/phantom-connect/introduction
- Get Started: https://phantom.app/phantom-connect
- React Template: https://github.com/phantom-labs/phantom-connect-react
- JS Template: https://github.com/phantom-labs/phantom-connect-js
- CASH stablecoin: https://phantom.app/cash

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
| Hyperliquid live execution | Not in scope for hackathon — documented as production roadmap item |
| World Coin | Toxic sentiment — skip entirely |
| Privy | Not yet available |
| Coinbase | Not yet available |
