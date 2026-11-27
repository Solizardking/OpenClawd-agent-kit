# Skills — sources and provenance

Every skill bundled in this directory, where it came from, and its
status. Retrieved / vendored **2026-11-27** unless noted.

| Directory | Skill | Origin | License | Status |
|---|---|---|---|---|
| `certora-solana-fv/` | Certora formal verification for Solana (CVLR) | First-party fleet skill, authored by the Clawd fleet (`local_certora_solana_fv`) | Fleet's own | **Vendored byte-faithful** from the fleet skill source, including `references/` |
| `harmonic-aristotle/` | Harmonic Aristotle prover for Rust/Solana programs | First-party fleet skill, authored by the Clawd fleet (`local_harmonic_aristotle`) | Fleet's own | **Vendored byte-faithful** from the fleet skill source, including `references/` |
| `solana-dev/` | Solana Development Skill (`solana-dev`) | Solana Foundation — [github.com/solana-foundation/solana-dev-skill](https://github.com/solana-foundation/solana-dev-skill), `skills/solana-dev/` at commit `c9da2624cc6baca5c8c2123ea50d92ef19f1c917`; also served at [solana.com/SKILL.md](https://solana.com/SKILL.md) | MIT (per the source repo) | **Indexed, not byte-vendored** — see the note below |

## The two fleet FV skills ship as-is

`certora-solana-fv/` and `harmonic-aristotle/` are vendored
byte-faithful, hard rails intact: per-project disclosure approval
before any Aristotle submission, never buy credits, never answer
Aristotle's clarifying questions for the operator, `CERTORAKEY`
handling, and the honest-verdict rules. Do not soften them in this
repo; changes belong in the fleet skill sources first.

## Note on `solana-dev/` (upstream, indexed)

The Solana Foundation's `solana-dev` skill is the canonical upstream
development skill for Solana (SKILL.md + ~30 reference files under
`references/`: `@solana/kit`, frontend, testing with
Surfpool/LiteSVM/Mollusk, tokens, payments, Kora, security, program
patterns for Anchor and Pinocchio, and more). It is **not**
byte-vendored in this commit:

- Direct byte fetches of the canonical files were blocked from this
  build environment (solana.com serves a bot checkpoint to plain HTTP
  clients, and the git-clone route tripped an execution-risk check
  that requires the operator's explicit go-ahead before the cloned
  content can be processed).
- `solana-dev/README.md` in this repo is therefore a pointer: what the
  skill covers, its published structure, and the supported install
  paths (`npx skills add solana-foundation/solana-dev-skill`).

Byte-vendoring the full skill is a follow-up once the operator
approves processing the source repo. Nothing else in this directory
depends on it.
