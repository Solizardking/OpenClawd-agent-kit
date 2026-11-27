# Solana.ai resources

What [solana.ai](https://solana.ai) actually publishes for agents —
researched live on 2026-11-27, resolved from the homepage's own links
and verified by direct probes. The machine-readable version of this
page is [`solana-ai-index.json`](./solana-ai-index.json); the bundled
skills live under [`skills/`](../skills/) in the repo with provenance
in [`skills/SOURCES.md`](../skills/SOURCES.md).

solana.ai frames the stack as: *"Read, act, and get found: the MCP
server, the agent kit, the registry, and skills that make Solana
legible to any agent."* Each of those four resolves to a concrete
resource:

## The four agent resources

### 1. Solana Developer MCP — read

**Endpoint `https://mcp.solana.com/mcp`** (landing:
[mcp.solana.com](https://mcp.solana.com)) — the official remote MCP
server for Solana documentation: `list_sections` and
`get_documentation` (canonical docs by source/section id),
`Solana_Documentation_Search` (semantic RAG search),
`Solana_Expert__Ask_For_Help` (how-to/debugging Q&A), and
`program_autofixer` (a static security linter for Anchor and
Pinocchio program Rust). Free, no API key; server identity
`solana-mcp` v2.0.0.

**In this kit:** a built-in `kit-mcp` server (namespace `solana`),
live-verified at 5 tools. See [MCP](./mcp.md) — including why the two
unannotated Q&A tools are classified consequential under the kit's
fail-closed policy despite the server being docs-only by purpose.

### 2. Solana Agent Kit — act (a different kit)

The homepage's "agent kit" card is SendAI's
[solana-agent-kit](https://github.com/sendaifun/solana-agent-kit)
(Apache-2.0, TypeScript, 60+ onchain actions, LangChain / Vercel AI
SDK adapters; docs at [docs.sendai.fun](https://docs.sendai.fun)).
It is **not** this repo. The OpenClawd agent kit is the Rust-native
counterpart — constitution-gated, Jev-judged, Lean-scaffolded. Both
exist; don't confuse them in dependency choices.

### 3. Agent Registry — get found

[solana.com/agent-registry](https://solana.com/agent-registry) — an
onchain protocol for agent identity, reputation, and validation on
Solana, interoperable with ERC-8004 on Ethereum. Ecosystem surfaces:
[8004.org](https://8004.org) (the standard),
[8004scan.io](https://8004scan.io) (explorer),
[8004market.io](https://8004market.io) (marketplace), and the SATI
dashboard at [sati.cascade.fyi](https://sati.cascade.fyi). (Distinct
from the Metaplex Agent Registry this fleet registers its own agents
through.)

### 4. Agent Skills — the `solana-dev` skill

The "Agent Skills" card resolves to the Solana Foundation's
[solana-dev-skill](https://github.com/solana-foundation/solana-dev-skill)
(MIT): one comprehensive Agent Skill (`SKILL.md` + ~30 reference
files) covering `@solana/kit`, wallet UI, web3.js v3 interop, Anchor
1.2.x / Pinocchio programs, Surfpool + LiteSVM/Mollusk testing,
Codama codegen, security patterns, and toolchain compatibility.
Install anywhere with:

```bash
npx skills add solana-foundation/solana-dev-skill
```

It is also served as a single file at
[solana.com/SKILL.md](https://solana.com/SKILL.md). **In this kit:**
indexed at [`skills/solana-dev/`](../skills/solana-dev/) with
provenance — byte-vendoring is pending for the reasons recorded in
[`skills/SOURCES.md`](../skills/SOURCES.md).

## The index: Awesome Solana AI

"Browse all Solana AI tools" is
[solana-foundation/awesome-solana-ai](https://github.com/solana-foundation/awesome-solana-ai)
— a curated, community-contributed list: ~50 AI coding skills across
General (Metaplex, Helius+Phantom, MagicBlock, Para), DeFi (Jupiter,
Drift, Kamino, Orca, Raydium, Meteora, Sanctum, DFlow), Infrastructure
(Helius, Alchemy, QuickNode, Light Protocol, Squads, Pyth,
Switchboard, Arcium), and Mobile & Seeker, plus agents, developer
tools, and learning resources. The repo carries its own
not-endorsed/DYOR disclaimer; treat listings as leads, not audits.
It is a living list — this kit links it rather than vendoring it.

## The docs themselves: solana.com's llms.txt family

The canonical Solana documentation is published for agents at
[solana.com/llms.txt](https://solana.com/llms.txt): an index whose
links are raw-markdown pages (append `.md` to any docs URL, or send
`Accept: text/markdown`). The full corpus in one file is
[solana.com/llms-full.txt](https://solana.com/llms-full.txt), and
scoped per-section indexes (assets, payments, tokenization, …) exist
for single-task context saving. The kit reaches this corpus live
through the `solana` MCP server rather than vendoring a snapshot.

## The product stack (context)

solana.ai's own products sit behind the resources above: **pay.sh**
(per-call agent payments in USDC — no accounts, no API keys),
**Payment Channels** (authorize once, settle once), the **Solana
Developer Platform** APIs they run on (wallet provisioning, payments,
token issuance, spending policies, compliance screening, bank
payouts), **Milo** (an autonomous trading agent), and a
confidential-hardware AI chat.

## Checked and NOT found (2026-11-27)

- **No `llms.txt` on solana.ai itself** — `solana.ai/llms.txt`
  returns 404. The llms.txt family lives on solana.com.
- **No docs bundle on mcp.solana.com** — the host answers JSON-RPC
  only.
- **No second first-party skills repo** — the skills card resolves
  to `solana-dev-skill`; every other skill is community work
  catalogued in awesome-solana-ai.
- `solana.com/ai` and `solana.com/agents` could not be verified from
  this environment (bot checkpoint on plain fetches); the registry
  page solana.ai links is `/agent-registry`, recorded above.
