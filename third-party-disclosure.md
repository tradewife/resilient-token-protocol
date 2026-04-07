# Third-Party Disclosure

RTP (Resilient Token Protocol) uses the following open-source frameworks, protocols, and sponsored tools.

## Open-Source Frameworks

| Component | License | Source | Use in RTP |
|-----------|---------|--------|------------|
| atlas-gic | MIT | https://github.com/chrisworsey55/atlas-gic | Multi-agent Darwinian loop — strategy evolution |
| karpathy/autoresearch | MIT | https://github.com/karpathy/autoresearch | Modify/Verify/Keep loop specification |
| uditgoenka/autoresearch | MIT | https://github.com/uditgoenka/autoresearch | Claude-native implementation |

## Solana Protocol Dependencies

| Dependency | License | Use |
|------------|---------|-----|
| Anchor Framework | Apache-2.0 | Solana program framework |
| SPL Token-2022 | Apache-2.0 | TransferFeeConfig for fee routing |
| Pyth Network | Apache-2.0 | TWAP oracle for price floor enforcement |
| Jupiter Aggregator | Apache-2.0 | Swap execution for buybacks + yield routing |
| Drift Protocol | Open | Perpetual futures for correlated SOL hedging |
| Kamino Finance | Open | Yield deployment for idle treasury capital |
| Marginfi | Open | Yield deployment for idle treasury capital |

## Sponsored Hackathon Resources

| Sponsor | Link | Use in RTP |
|---------|------|------------|
| Phantom Connect + CASH | https://docs.phantom.app/phantom-connect/introduction | Agentic wallet for treasury interactions + CASH stablecoin flows |
| Squads Multisig | https://docs.squads.so | Securing treasury PDA upgrade authority |
| MoonPay Agents | https://www.moonpay.com/developers/agents | Agent money movement infrastructure |
| Solana MCP | https://github.com/solana-developers/solana-mcp | AI-powered development assistant for Anchor programs |

## Black-Box Components

The yield brain ships as compiled binaries. Source is not included to protect the competitive strategy moat.

- **night_shift.bin** — Strategy optimizer (PyInstaller binary)
- **configs/encrypted/** — AES-encrypted strategy parameters

These binaries are deterministic — given the same input data and parameters, they produce identical outputs. The open-source agent swarm interacts with them via a typed JSON interface.

## Research

Pre-hackathon research and skill system design: https://github.com/tradewife/rtp-skills-research

---

Built for the Solana Frontier Hackathon (Colosseum × Canteen, 2026).
