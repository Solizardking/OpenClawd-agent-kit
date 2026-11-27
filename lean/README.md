# OpenClawd Kit — Lean package

Formal statements of the kit's core safety invariants, pinned against the
Solana Foundation's [solanalib](https://github.com/solana-foundation/leanprover-solanalib)
(Lean models of the Solana runtime and the Rust crates Solana programs are
built on, following Agave 4.3.2).

- **Toolchain:** `leanprover/lean4:v4.34.0` (see `lean-toolchain`)
- **solanalib pin:** commit `8f3f5c6dcf4de6e175fecaefe6d589bdeb0b047e`
  (main as of 2026-11-24) — a fixed commit, per solanalib's own guidance
- **Contents:** `OpenClawdKit/Invariants.lean` — spend-cap monotonicity and
  cap-respect (proved), the Jev-gate no-execution-without-judgment
  invariant and signer-policy fail-closed property (stated; proofs are
  the next targets — the `sorry`s are deliberate and visible)

The Rust counterparts live in `../crates/kit-lean` and run as tests today.
See `../docs/lean.md` for the full picture: Rust models → Lean statements →
the Solana Lean client path.

## Building

Requires [elan](https://github.com/leanprover/elan):

```bash
cd lean
lake update   # generates lake-manifest.json against the pinned commit
lake build
```

This package was scaffolded on a machine without a Lean toolchain
installed; the first `lake build` on a Lean-equipped machine is the
shakedown step (see `docs/lean.md` → "Status").
