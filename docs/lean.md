# Lean

The kit is **Lean-ready**: its core safety invariants exist in two forms
that mirror each other statement-for-statement — executable Rust models
that run as tests today, and Lean 4 statements that a prover (or the
emerging Solana Lean client stack) can check tomorrow.

Lean is one track of the kit's wider formal-verification story —
Certora CVLR and Harmonic Aristotle for whole Solana programs, Kani /
property-based tests for Rust kernels. See
[Formal verification](./formal-verification.md) for the full picture
and the which-tool-for-which-job table.

## The Lean side

`lean/` is a Lake package:

- **Toolchain** `leanprover/lean4:v4.34.0`
- **Dependency:** the Solana Foundation's
  [solanalib](https://github.com/solana-foundation/leanprover-solanalib) —
  Lean models of the Solana runtime and of the Rust crates Solana
  programs are built on (Anchor, Pinocchio, SPL Token, the SDK crates),
  following Agave 4.3.2 — pinned at a **fixed commit**,
  `8f3f5c6dcf4de6e175fecaefe6d589bdeb0b047e` (main as of 2026-11-24), per
  solanalib's own guidance for an experimental library.
- **`OpenClawdKit/Invariants.lean`** states:
  - `SpendState.spend_monotone` — spend never decreases *(proved)*
  - `SpendState.cap_respected` — spend never exceeds the cap *(proved)*
  - `GateLog.no_execution_without_judgment` — an executed call was
    necessarily judged `proceed` first *(stated; proof is target #1)*
  - `SignerPolicy.empty_permits_nothing` — an empty allowlist permits
    nothing *(stated)*

The two `sorry`s are deliberate and visible. Nothing in the package
should be cited as formally verified until they are discharged.

## The Rust side

`crates/kit-lean` holds the same models as executable Rust
(`SpendLedger`, `GateLog`, `SignerPolicy`) with unit + property tests —
including a 10,000-step randomized monotonicity/cap walk and the
no-execution-without-judgment state-machine test. Names match the Lean
side on purpose, so a statement and its test are greppable as a pair.

## The path: Rust → Lean specs → Lean client

1. **Today:** invariants are Rust models + tests; Lean statements exist
   for the same properties, two of them proved.
2. **Next:** discharge the gate and signer-policy proofs; restate the
   models over solanalib's runtime types (`Solanalib.Runtime`) instead
   of the kit's standalone `Nat` models, so the invariants talk about
   real Solana accounts and transactions.
3. **Then:** a Lean client verifies chain state against validator-voted
   roots. The kit is ready for it on the Rust side already:
   `kit_solana::verify::LightVerify` is the trait such a backend
   implements, and every consumer of verified state in the kit programs
   against that trait (see the [ohr note](./ohr.md)).

## Status (honest)

The Lean package was scaffolded on a machine with **no Lean toolchain
installed**, so it has not been `lake build`-verified yet. The first
build on a Lean-equipped machine is the shakedown step:

```bash
cd lean
lake update   # resolves solanalib at the pinned commit
lake build
```

Expected shakedown risks: solanalib's package layout moved recently
(sources under `packages/`), so the `require` path may need adjusting to
the pinned commit's actual library name; the two proved theorems use
only core Lean tactics and should check as written.
