# solana-dev — the Solana Foundation's development skill (pointer)

> This directory is an **index entry**, not the skill itself. See
> [`../SOURCES.md`](../SOURCES.md) for provenance and for why the full
> skill is not byte-vendored in this repo yet.

**Upstream:** Solana Foundation —
[github.com/solana-foundation/solana-dev-skill](https://github.com/solana-foundation/solana-dev-skill)
(commit `c9da2624cc6baca5c8c2123ea50d92ef19f1c917` as of 2026-11-27) ·
License: MIT · Also served at
[solana.com/SKILL.md](https://solana.com/SKILL.md)

## Install (supported paths, per the upstream README)

```bash
npx skills add solana-foundation/solana-dev-skill   # any agent, via skills.sh
# or: clone the repo and run ./install.sh [--project] [--link]
# or: copy skills/solana-dev/ into your agent's skills directory
```

## What it covers

A comprehensive Agent Skill for modern Solana development (Agent
Skills format — works with Claude Code, Codex, Copilot, Gemini CLI,
Cursor, Windsurf, Cline, OpenCode, and anything else that reads
`SKILL.md`):

- **SDK:** `@solana/kit` plugin clients, transaction v1 by default
- **UI:** Wallet Standard via `@solana/kit-plugin-wallet` + `@solana/react`
- **Legacy interop:** web3.js v3 and v1 migration
- **Programs:** Anchor 1.2.x by default, Pinocchio 0.11+ for performance
- **Testing:** Surfpool integration tests, LiteSVM/Mollusk unit tests
- **Codegen:** Codama-first IDL and client generation
- **Security:** vulnerability patterns and prevention
- **Toolchain:** version compatibility, common errors, troubleshooting

Published structure: `SKILL.md` plus `references/` (kit/, programs/,
anchor/, surfpool/ subdirectories and topic files — frontend, testing,
idl-codegen, tokens, payments, kora, keychain, mosaic, security,
concepts, compatibility-matrix, common-errors, confidential-transfers,
rpc-quick-lookups, transactions-v1).

## How the kit relates

The kit's own Solana guidance lives in the book
([Solana](../docs/solana.md), [Solana.ai resources](../docs/solana-ai.md))
and the Solana Developer MCP is wired into `kit-mcp` as the `solana`
server. This upstream skill is the coding-agent companion to those:
install it in your coding agent for current APIs and conventions.
