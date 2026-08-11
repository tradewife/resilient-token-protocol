# Archive

This directory holds code and artifacts from earlier directions that are no
longer in the active product path. It exists so:

- Nothing is deleted — `git log --follow` still reaches every file.
- The portfolio story (where RTP came from) is preserved in place.
- Active `rg`/code review doesn't surface legacy noise.

## What's here and why

| Path | Reason archived |
|---|---|
| `flash-trade/` | Flash Trade SDK reference docs; venue wound down Aug 2026. |
| `media/` | Remotion video project + earlier marketing/spec drafts; superseded by live dashboard. |
| `cli/` | Operator CLI for the **token-onboarding era** (`rtp init`, `rtp freeze`, `rtp demo`, etc.). Bespoke strategy pipeline is sold differently — no client runs `rtp init`. |
| `scripts/` (subset) | Flash-specific scripts (`flash-fund-and-open-sol`, `flash-trade-loop`, `mainnet-proof`, `smoke-open-close-usdc`, `derive_flash_accounts`, etc.). |
| `HANDOVER.md` | One-off session handover from May 2026. |
| `SESSION-CONTEXT.md` | Compressed session memory — superseded by `docs/STRATEGIC-DIRECTION.md` and `CLAUDE.md`. |
| `docs/COLOSSEUM-AUDIT.md` | Hackathon-era security audit findings (v1.1). Superseded by mainnet hardening. |
| `docs/RESOURCES.md` | Hackathon links/sponsor list. |
| `dashboard/Dockerfile` | Deprecated. Canonical is `Dockerfile.dashboard` at repo root (Node 22 + SQLite intake). |
| `night-results/` | Historical `data/night_results/*` from April–early August 2026. Latest two days kept live for promote-strategy cron. |

## Live still in place

These must remain in the active tree:

- `cli/flash-sdk-wrapper.mjs` + `cli/package.json` — required by `rtp/swarm/Dockerfile.trader` for the Flash REST bridge.
- `scripts/redistribute.ts` + `scripts/promote-strategy.ts` — required by `Dockerfile.crank` and `Dockerfile.promote`.
- `data/ohlcv/` — referenced by `research/orchestration/night_shift.py`.
- `data/night_results/` (latest two days only) — baked into the `promote-strategy` image so Railway V3 (no volumes) can read candidates.
- `scripts/entrypoint-night-shift.sh` — `research/Dockerfile` entrypoint; runs night-shift and pushes results back to git.

## Restoring anything

```bash
git mv archive/<path> <path>
```

Every file is still tracked; history is intact.
