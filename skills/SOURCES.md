# Skills — sources and provenance

Every skill bundled in this directory, where it came from, and its
status. Retrieved / vendored **2026-11-27** unless noted.

| Directory | Skill | Origin | License | Status |
|---|---|---|---|---|
| `certora-solana-fv/` | Certora formal verification for Solana (CVLR) | First-party fleet skill, authored by the Clawd fleet (`local_certora_solana_fv`) | Fleet's own | **Vendored byte-faithful** from the fleet skill source, including `references/` |
| `harmonic-aristotle/` | Harmonic Aristotle prover for Rust/Solana programs | First-party fleet skill, authored by the Clawd fleet (`local_harmonic_aristotle`) | Fleet's own | **Vendored byte-faithful** from the fleet skill source, including `references/` |
| `solana-dev/` | Solana Development Skill (`solana-dev`) | Solana Foundation — [github.com/solana-foundation/solana-dev-skill](https://github.com/solana-foundation/solana-dev-skill), `skills/solana-dev/` at commit `c9da2624cc6baca5c8c2123ea50d92ef19f1c917`; also served at [solana.com/SKILL.md](https://solana.com/SKILL.md) | MIT (per the source repo) | **Vendored byte-faithful** — see the note below |

## The two fleet FV skills ship as-is

`certora-solana-fv/` and `harmonic-aristotle/` are vendored
byte-faithful, hard rails intact: per-project disclosure approval
before any Aristotle submission, never buy credits, never answer
Aristotle's clarifying questions for the operator, `CERTORAKEY`
handling, and the honest-verdict rules. Do not soften them in this
repo; changes belong in the fleet skill sources first.

## Note on `solana-dev/` (upstream, vendored byte-faithful)

The Solana Foundation's `solana-dev` skill is the canonical upstream
development skill for Solana. As of 2026-11-27 it **is** byte-vendored
here, with the operator's explicit go-ahead to process the source
repo:

- Contents: `SKILL.md` + 36 reference files under `references/`
  (`@solana/kit`, frontend, testing with Surfpool/LiteSVM/Mollusk,
  tokens, payments, Kora, security, program patterns for Anchor and
  Pinocchio, and more), copied unmodified from the upstream repo's
  `skills/solana-dev/` at commit
  `c9da2624cc6baca5c8c2123ea50d92ef19f1c917`, plus the repo's MIT
  `LICENSE` copied alongside.
- Verification: a sha256 manifest of every vendored file matches the
  source clone exactly, and `diff -r` against the clone is clean
  (the only delta is the added `LICENSE`).
- Scope: vendoring is text only. The upstream repo also carries
  tooling that is **not** part of the skill and was neither copied
  nor run: `install.sh`, `scripts/validate-skill.py`,
  `scripts/validate-skills.sh`, `tests/`, CI workflows, and a binary
  banner image. Supported install paths for the skill itself remain
  as upstream documents them (e.g. `npx skills add
  solana-foundation/solana-dev-skill`); this copy is a pinned,
  in-repo snapshot — refresh it by re-vendoring from a newer
  upstream commit, never by editing the vendored files in place.
