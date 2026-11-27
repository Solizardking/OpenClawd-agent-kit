import Lake
open Lake DSL

/-!
# OpenClawd Kit — Lean package

Formal statements of the kit's core safety invariants, checked against the
Solana Foundation's **solanalib** (Lean models of the Solana runtime and of
the Rust crates Solana programs are built on, following Agave 4.3.2).

solanalib is pinned at a FIXED commit, per its own guidance ("depend on a
fixed commit" — the library is experimental and its layout may change):

  solana-foundation/leanprover-solanalib @ 8f3f5c6dcf4de6e175fecaefe6d589bdeb0b047e
  (main as of 2026-11-24; toolchain leanprover/lean4:v4.34.0)

The Rust side of these invariants lives in `crates/kit-lean` and runs as
tests today; this package is where proofs replace tests as they are
completed. Statements marked `sorry` are scaffolded, NOT proved — see
`OpenClawdKit/Invariants.lean`.
-/

package openclawdKit where
  leanOptions := #[
    ⟨`autoImplicit, false⟩
  ]

require solanalib from git
  "https://github.com/solana-foundation/leanprover-solanalib" @ "8f3f5c6dcf4de6e175fecaefe6d589bdeb0b047e"

@[default_target]
lean_lib OpenClawdKit where
