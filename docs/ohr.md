# The "ohr" note

The rewrite order named an **"ohr Solana lean client"** as an integration
target. As of 2026-11-27, no distinct Solana client by the name "ohr"
could be identified from public sources. What verifiably exists today:

- **solanalib** (Solana Foundation) — the Lean 4 model library for the
  Solana runtime and its Rust crates. This is the live Lean stack, and
  it is what the kit pins against (see [Lean](./lean.md)).
- Light-client work in the ecosystem generally (e.g. Tinydancer's
  SIMD-0052 line of work on transaction inclusion proofs) — relevant
  background, but not a Lean client and not named "ohr".

Rather than invent an integration, the kit makes the seam real and
leaves the nameplate blank:

- `kit_solana::verify::LightVerify` is the trait a Lean-client backend
  implements: `verify_account_state(pubkey, expected_proof) -> bool`.
- Today's implementation, `RpcStateProof`, verifies by re-reading via
  RPC and comparing digests. It is documented everywhere as an
  attestation stub, **not** cryptographic verification.
- When the "ohr" repo (or whichever Lean client the name refers to) is
  supplied, the work is: implement `LightVerify` over its verified state
  root, restate `AccountProof` checking against solanalib's runtime
  model, and swap the implementation at the composition root. No call
  sites change.

If you have the repo link, drop it in an issue — this page exists to be
replaced by the real integration notes.
