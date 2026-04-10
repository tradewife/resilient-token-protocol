# Third-Party Disclosure

RTP (Resilient Token Protocol) uses the following open-source frameworks and sponsored tools.

## Open-Source Frameworks

| Component | License | Source | Use in RTP |
|-----------|---------|--------|------------|
| atlas-gic | MIT | https://github.com/chrisworsey55/atlas-gic | Multi-agent Darwinian loop — Evolve Wing autoresearch engine |
| karpathy/autoresearch | MIT | https://github.com/karpathy/autoresearch | Core Modify/Verify/Keep loop specification |
| uditgoenka/autoresearch | MIT | https://github.com/uditgoenka/autoresearch | Claude-native autoresearch implementation |
| MetaClaw | MIT | https://github.com/aiming-lab/MetaClaw | Knowledge Wing memory + human override UI |
| revfactory/harness | MIT | https://github.com/revfactory/harness | Coordinator architecture reference |
| autoagent | MIT | https://github.com/kevinrgu/autoagent | Wing lifecycle scaffolding (spawn, health-check, retire) |

## Sponsored Hackathon Resources

| Sponsor | Link | Use in RTP |
|---------|------|------------|
| Phantom Connect + CASH | https://docs.phantom.app/phantom-connect/introduction | Agentic wallet for treasury interactions + CASH stablecoin flows |
| Squads Multisig | https://docs.squads.so | Securing treasury PDA upgrade authority |
| Swig | https://docs.swig.fi | Programmable smart wallets for wing message bus |
| MoonPay Agents | https://www.moonpay.com/developers/agents | Agent money movement infrastructure |
| Solana MCP | https://github.com/solana-developers/solana-mcp | AI-powered development assistant for Anchor programs |
| Arcium | https://docs.arcium.com | Encrypted computation (stretch goal, not yet integrated) |

## Solana Program Dependencies

| Dependency | License | Use |
|------------|---------|-----|
| Anchor Framework | Apache-2.0 | Solana program framework |
| SPL Token | Apache-2.0 | Token operations, TransferFeeConfig |
| Solana SDK | Apache-2.0 | On-chain program development |

## Black-Box Components

The following components are intended to ship as compiled binaries to protect the competitive strategy moat. However, **black-boxing is currently deferred** while the repo remains private for active collaboration. Source code is readable for all contributors.

- **night_shift.bin** — Yield brain optimizer (PyInstaller binary from `research/orchestration/night_shift.py`)
- **configs/** — Strategy parameters (not currently encrypted; encryption planned for production)
- **Bridge interface** — Typed JSON interface between Python research and Rust execution (`rtp/swarm/src/bridge.rs`)

The binaries are deterministic — given the same input data and parameters, they produce identical outputs. The open-source swarm architecture interacts with them via a typed JSON interface (`BridgeRequest` → stdin → `BridgeResponse` → stdout).

## Contact

Built for the Solana Frontier Hackathon (Colosseum × Canteen, 2026).
Repo: https://github.com/tradewife/resilient-token-protocol
