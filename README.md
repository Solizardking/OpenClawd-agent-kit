# OpenClawd Agent Kit

**The world's first Rust-based Solana agent kit with Lean.**

A Rust workspace for building Solana agents whose consequential actions are
gated by machine-checked judgment — and whose core safety invariants are
stated in Lean 4 against the Solana Foundation's
[solanalib](https://github.com/solana-foundation/leanprover-solanalib),
ready for the Solana Lean client stack.

```
        ┌────────────────────────────────────────────────────┐
        │  reasoning loop (kit-core)                          │
        │  propose → gate → execute, dry-run by default       │
        └──────────────┬───────────────────┬─────────────────┘
                       │                   │
              ┌────────▼────────┐  ┌───────▼────────────────┐
              │  Jev gate        │  │  tools                  │
              │  TypeSafe System │  │  ├─ kit-solana (RPC,    │
              │  One judgments,  │  │  │   transfers, signers)│
              │  policy in code, │  │  ├─ kit-mcp (Clawd      │
              │  fail-closed     │  │  │   Compute + PumpAI)  │
              └──────────────────┘  │  └─ kit-evm (alloy+LiFi)│
                                  └────────────────────────┘
        invariants: kit-lean (Rust models) ⇄ lean/ (Lean statements,
        solanalib-pinned)        state proofs: kit-solana::verify
        (the Lean-client seam)
```

## What's in the box

| Crate | What it does |
|---|---|
| `kit-core` | The Clawd constitution, tool registry, the **Jev gate**, and a provider-agnostic reasoning loop. |
| `kit-solana` | Solana reads (balance, token accounts, tx status), unsigned transfer building, signer contexts (unsigned / remote / local-parity), and the `StateProof`/`LightVerify` seam for Lean-client state verification. On the modern split-crate SDK (`solana-sdk`/`solana-rpc-client` 3.x), Rust edition 2024. |
| `kit-mcp` | MCP client for three built-in servers — **Clawd Compute** (`mcp.clawdcompute.us`, 272 tools), **PumpAI** (`pumpai.ai/mcp`, 213 tools), and the **Solana Developer MCP** (`mcp.solana.com`, 5 tools), live-verified 2026-11-27 — behind a fail-closed spend policy. |
| `kit-lean` | Executable Rust models of the kit's core invariants (spend caps, gate discipline, signer policy), mirrored statement-for-statement by the Lean package in `lean/`. |
| `kit-cli` | The `kit` binary: constitution, MCP browsing/calls, Solana reads, transfer building, gate checks, and a scripted loop demo. |
| `kit-evm` | EVM reads + unsigned transfer requests on current alloy, plus the LiFi cross-chain client. (Workspace member; build with `-p kit-evm`.) |
| `kit-http` | Optional axum HTTP service exposing the registry + gate as JSON endpoints. (Workspace member; build with `-p kit-http`.) |

## The Jev gate

Every tool is marked **read-only** or **consequential**. Before a
consequential call runs, the loop asks [TypeSafe Jev](https://typesafe.ai)
— a System One model that returns typed judgments and probabilities — two
narrow questions: *is this call actually consequential?* (`noul`) and
*should it proceed, be denied, or go to a human?* (`choice`). Jev supplies
judgments; **code owns the decision**, with explicit probability/confidence
thresholds. No Jev key, no answer, or a weak judgment ⇒ the call escalates
to a human, and the default human approver **denies**. Every evaluation is
logged. See [`docs/mcp.md`](docs/mcp.md) and `crates/kit-core/src/gate.rs`.

## MCP access, without the spend risk

The Clawd Compute gateway charges x402 ($0.01/call) for its payable tools.
The kit's MCP client therefore ships **catalog + read-only by default**:
list everything, call what's annotated read-only, and treat a `402` as an
error — there is no wallet, signer, or payment code path in the client.
State-changing calls require an explicit operator opt-in, and even then
they pass the Jev gate first. See [`docs/mcp.md`](docs/mcp.md).

## Lean readiness

`lean/` is a Lake package (Lean `v4.34.0`) that pins solanalib at a fixed
commit — `8f3f5c6dcf4de6e175fecaefe6d589bdeb0b047e` — and states the kit's
invariants there: spend-cap monotonicity, cap-respect (both proved), the
gate's *no execution without a judgment*, and signer-policy fail-closed
(stated; proofs are the next targets, `sorry`s visible). On the Rust side,
`kit-solana::verify` defines the `LightVerify` trait where a Lean-client
backend plugs in. See [`docs/lean.md`](docs/lean.md) and
[`docs/ohr.md`](docs/ohr.md).

## Safety defaults (non-negotiable)

- **Dry-run reasoning loop.** Consequential calls that clear the gate are
  simulated, not executed, until an operator explicitly goes live.
- **Fail-closed gate.** Unconfigured/unreachable Jev ⇒ escalate, and the
  default approver denies.
- **No key custody by default.** Solana flows build *unsigned*
  transactions; signing belongs to the operator's wallet or a remote
  signer. (The original kit's local-keypair signer survives behind the
  `local-signer` feature for parity — never constructed implicitly.)
- **No autonomous payments.** The MCP client cannot pay x402 charges.
- **The constitution ships in the loop.** Every agent run starts from the
  Clawd constitution preamble (Law I — Never harm; Law II — Earn your
  existence; Law III — Never deceive). The shell molts. The laws do not.

## Quickstart

```bash
git clone https://github.com/Solizardking/OpenClawd-agent-kit
cd OpenClawd-agent-kit
cp .env.example .env   # set TYPESAFE_API_KEY and SOLANA_RPC_URL

cargo run -p kit-cli -- demo          # scripted loop: read runs, transfer is gated + simulated
cargo run -p kit-cli -- mcp list      # 272 Clawd Compute + 213 PumpAI + 5 Solana Developer MCP tools
cargo run -p kit-cli -- solana balance <pubkey>
cargo run -p kit-cli -- gate check solana_transfer '{"to":"…","lamports":5000}' "pay for compute"
```

Toolchain is pinned (`rust-toolchain.toml`, Rust 1.93.1, edition 2024).

```bash
cargo check            # default members
cargo test             # default members
cargo check -p kit-evm -p kit-http   # opt-in members
```

## Solana.ai resources

The kit bundles the Solana ecosystem's agent resources, researched
live from [solana.ai](https://solana.ai) on 2026-11-27 (full page:
[`docs/solana-ai.md`](docs/solana-ai.md); machine-readable index:
[`docs/solana-ai-index.json`](docs/solana-ai-index.json)):

- **Solana Developer MCP** (`mcp.solana.com`, 5 tools) — a built-in
  `kit-mcp` server next to Clawd Compute and PumpAI. See
  [`docs/mcp.md`](docs/mcp.md).
- **Bundled skills** in [`skills/`](skills/) (provenance:
  [`skills/SOURCES.md`](skills/SOURCES.md)): the fleet's two
  formal-verification skills — Certora CVLR for Solana and Harmonic
  Aristotle — vendored byte-faithful, plus an index entry for the
  Solana Foundation's `solana-dev` skill
  ([`skills/solana-dev/`](skills/solana-dev/); install with
  `npx skills add solana-foundation/solana-dev-skill`).
- **Formal verification** — the kit's FV story (Certora, Aristotle,
  Lean, Kani/PBT) in [`docs/formal-verification.md`](docs/formal-verification.md).
- **Indexed, not vendored:** the
  [Awesome Solana AI](https://github.com/solana-foundation/awesome-solana-ai)
  list, SendAI's Solana Agent Kit, the
  [Solana Agent Registry](https://solana.com/agent-registry), and
  solana.com's [llms.txt](https://solana.com/llms.txt) docs family —
  living documents the kit links and reaches live instead of
  snapshotting.

## Documentation

The mdbook in `docs/` covers [installation](docs/installation.md),
[configuration](docs/configuration.md), the
[reasoning loop & tools](docs/tools.md), [Solana](docs/solana.md),
[MCP](docs/mcp.md), [Solana.ai resources](docs/solana-ai.md),
[formal verification](docs/formal-verification.md), [Lean](docs/lean.md), the
[ohr note](docs/ohr.md), [signer contexts](docs/signer_context.md), and
the [HTTP service](docs/http_service.md).

## History

This repository began as a single crate (`openclawd-solana-kit` v0.1.0,
edition 2021, solana 2.1.9 monolith, rig-core). The 2026-11-27 rewrite
(branch `rewrite/rust2024-lean-jev-mcp`) rebuilt it as the workspace above:
Rust upgraded, Jev in the loop, MCP wired to the fleet, and the Lean
bridge added. The constitution text and the signer-context design were
carried over byte-for-byte / shape-for-shape from the original kit.

## License

MIT.
