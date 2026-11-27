# Formal verification

The kit ships with a formal-verification (FV) story in four parts —
two prover tracks for Solana programs, the Lean track for the kit's
own invariants, and lightweight Rust verification for kernels — plus
the two fleet skills that operate the prover tracks, bundled under
[`skills/`](../skills/) (provenance:
[`skills/SOURCES.md`](../skills/SOURCES.md)).

The claim discipline across all four: a result is *"formally verified
under documented assumptions"* — never more. A counterexample is a
finding to record verbatim, never a spec to weaken until it passes.

## Which tool for which job

| Tool | Target | What it proves | Cost / access model | Bundled skill |
|---|---|---|---|---|
| **Certora (CVLR)** | Solana programs in Rust (Pinocchio/Anchor-style) compiled to sBPF | Rule-based properties over the actual program artifact, checked by the Certora prover | CERTORAKEY in the prover environment only; cloud jobs | [`skills/certora-solana-fv/`](../skills/certora-solana-fv/) |
| **Harmonic Aristotle** | Rust (Solana) programs, source bundles | Property verification by Harmonic's agentic prover, with a clarifying-questions protocol | Credits; **every submission needs the operator's per-project disclosure approval** | [`skills/harmonic-aristotle/`](../skills/harmonic-aristotle/) |
| **Lean 4 + solanalib** | The kit's own invariants; Solana runtime/library models | Machine-checked theorems about stated models | Free; local toolchain (see the shakedown status below) | — (see [Lean](./lean.md)) |
| **Kani / proptest (PBT)** | Rust kernels and pure functions | Bounded model checking (Kani) and property-based tests for arithmetic/state machines | Free; local cargo toolchain | — |

## Certora CVLR for Solana

Certora's Solana route is CVLR — Rust specs written against the
program, run through the Certora prover with the Solana
platform-tools toolchain. The bundled skill
([`skills/certora-solana-fv/SKILL.md`](../skills/certora-solana-fv/SKILL.md))
covers connection, toolchain setup, cloud submission, and honest
verdict reporting, including the platform engines and the REST API.

**Scoping caveat, stated plainly:** Certora's AutoProver and
AutoFuzzer are **Solidity-only** — never promise them for Rust/Solana.
The AI Auditor is language-agnostic and usable on Solana today. The
prover's rule-encoding limits are real (stack-offset and encoding
failures surface as UNKNOWN job results, which are *not* verdicts
about the program).

## Harmonic Aristotle

Aristotle verifies Rust programs from a source bundle. The bundled
skill ([`skills/harmonic-aristotle/SKILL.md`](../skills/harmonic-aristotle/SKILL.md))
carries the operating discipline, and its hard rails ship unmodified:

- **Per-project disclosure approval** — every new project submission
  needs the operator's explicit go for that project; a bundle leaves
  the machine only after it.
- **Never buy credits unasked**; `aristotle continue` starts a new
  billed run — never unasked.
- **Never answer Aristotle's clarifying questions for the operator** —
  surface them verbatim.
- **OUT_OF_BUDGET is not a verdict** — 0 proved / 0 refuted is a
  budget outcome, reported as such.

## The Lean track

The kit's own invariants are stated in Lean 4 against the Solana
Foundation's solanalib — see [Lean](./lean.md) for the package, the
pinned commit, and the statement list. Status today: spend
monotonicity and cap-respect are **proved**; the gate
no-execution-without-judgment and signer-policy invariants are
**stated with two deliberate, visible `sorry`s**; and `lake build`
has **not** been run in the kit's build environment (no Lean
toolchain there) — it is the documented shakedown step, not a claimed
result. The same invariants exist as tested executable models in
`crates/kit-lean`, so the properties are enforced by tests today and
by proofs as the Lean side matures.

## Kani / PBT for Rust kernels

For self-contained Rust — arithmetic kernels, state machines, the
kit's own policy predicates — Kani (bounded model checking, available
in this workspace's toolchain) and property-based tests are the right
first tool: no cloud, no credits, no disclosure surface. Reach for
Certora or Aristotle when the property is about a whole program's
behavior against its account model, not a single function's contract.

## How this connects

- The Jev gate and spend policy that guard the kit's tool calls are
  exactly the invariants the Lean track states — FV here is not
  decorative; it targets the kit's own safety claims first.
- The Solana Developer MCP's `program_autofixer` (see
  [MCP](./mcp.md)) is the lightweight front door: lint program Rust
  with it before escalating a property to a prover.
- The upstream `solana-dev` skill's security references (see
  [Solana.ai resources](./solana-ai.md)) pair with the Certora skill:
  patterns first, proofs second.
