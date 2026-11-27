# Installation

## Requirements

- **Rust** — the workspace pins its toolchain in `rust-toolchain.toml`
  (1.89.0, edition 2024). Install [rustup](https://rustup.rs/) and the
  pinned toolchain is fetched automatically on first `cargo` run.
- **A TypeSafe API key** for live Jev gate judgments (`TYPESAFE_API_KEY`).
  Without it the kit still runs — the gate simply escalates every
  consequential call (fail-closed).
- **A Solana RPC endpoint** for `kit-solana` reads/builds. A private
  endpoint (Helius, QuikNode, Triton) is strongly recommended; the public
  mainnet endpoint is the fallback.
- **No LLM key is required by the kit itself.** The reasoning loop is
  provider-agnostic (`kit_core::Reasoner`); hosts plug in their own
  backend. The CLI's `demo` uses a scripted reasoner.
- *Optional:* **Lean 4** via [elan](https://github.com/leanprover/elan)
  if you want to build the `lean/` package (toolchain
  `leanprover/lean4:v4.34.0`).

## From this repository

```bash
git clone https://github.com/Solizardking/OpenClawd-agent-kit
cd OpenClawd-agent-kit
cp .env.example .env
cargo check
cargo test
```

On minimal Linux images, install the usual build prerequisites first
(`build-essential`, `pkg-config`, `libssl-dev`); the kit's default
members use rustls, so no OpenSSL is needed at runtime.

## As a dependency

The crates are path-friendly workspace members; consume them from git:

```toml
kit-core = { git = "https://github.com/Solizardking/OpenClawd-agent-kit", branch = "main" }
kit-solana = { git = "https://github.com/Solizardking/OpenClawd-agent-kit", branch = "main" }
kit-mcp = { git = "https://github.com/Solizardking/OpenClawd-agent-kit", branch = "main" }
```
