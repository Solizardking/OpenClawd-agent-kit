# Introduction

**OpenClawd Agent Kit** is a Rust workspace for building Solana agents
whose consequential actions are gated by machine-checked judgment — and
whose core safety invariants are stated in Lean 4 against the Solana
Foundation's solanalib. It is the world's first Rust-based Solana agent
kit with Lean.

The kit is a rewrite (2026-11-27) of the original single-crate
`openclawd-solana-kit`. What carried over: the Clawd constitution
(byte-for-byte), the signer-context design, and the scope — Solana tools,
EVM/cross-chain rails, an optional HTTP service. What is new:

- **Rust, upgraded.** Edition 2024, pinned toolchain (1.93.1), a Cargo
  workspace, and the modern split-crate Solana SDK (3.x) instead of the
  2.1.9 monolith.
- **Jev in the reasoning loop.** TypeSafe's System One model gates every
  consequential tool call with typed judgments and probabilities; policy
  stays in code; the gate fails closed.
- **MCP access.** First-class client for the Clawd Compute gateway
  (272 tools) and PumpAI (213 tools), behind a fail-closed spend policy —
  the client cannot pay x402 charges or trade autonomously.
- **Lean readiness.** A Lake package pinned to solanalib states the kit's
  invariants (spend caps, gate discipline, signer policy), and
  `kit-solana::verify` defines the trait seam where a Solana Lean client
  plugs in.

## Crates

| Crate | Role |
|---|---|
| `kit-core` | Constitution, tool registry, Jev gate, reasoning loop |
| `kit-solana` | Solana reads/builds, signers, state-proof seam |
| `kit-mcp` | MCP client + spend policy |
| `kit-lean` | Executable invariant models (Rust side of the Lean specs) |
| `kit-cli` | The `kit` binary |
| `kit-evm` | EVM + LiFi cross-chain (opt-in build) |
| `kit-http` | axum HTTP service (opt-in build) |

## Safety posture

Dry-run by default. Fail-closed gate. No key custody by default. No
autonomous payments. The constitution in every preamble. These are fleet
standing rules, and the kit's defaults encode them rather than merely
recommending them.
