# Configuration

All configuration is environment variables (see `.env.example` for the
annotated template). The kit loads nothing implicitly beyond these, and
every missing piece degrades toward safety, never toward permission.

## Jev gate

| Variable | Effect |
|---|---|
| `TYPESAFE_API_KEY` | Enables live Jev judgments. The gate reads the key from this variable only — never from files or flags. Unset ⇒ consequential calls escalate (fail closed). |

Gate thresholds are **code**, not config: they live in
`crates/kit-core/src/gate.rs` (`NON_CONSEQUENTIAL_BELOW`,
`PROCEED_MIN_PROBABILITY`, `PROCEED_MIN_CONFIDENCE`, `DENY_MIN_CONFIDENCE`)
so a deployment can't quietly loosen them with an env var.

## Solana

| Variable | Effect |
|---|---|
| `SOLANA_RPC_URL` | RPC endpoint for `kit-solana` reads/builds. Default: public mainnet. Prefer a private endpoint in production. |
| `SOLANA_PRIVATE_KEY` | Only used if a host explicitly constructs the feature-gated `LocalKeypairSigner` (`local-signer` feature). The kit never reads it implicitly. |
| `KIT_REMOTE_SIGNER_URL` / `KIT_REMOTE_SIGNER_TOKEN` | Operator-run remote signing endpoint + optional bearer token for `RemoteSigner`. |

## MCP

| Variable | Effect |
|---|---|
| `CLAWD_COMPUTE_MCP_URL` | Default `https://mcp.clawdcompute.us/mcp` |
| `PUMPAI_MCP_URL` | Default `https://pumpai.ai/mcp` |

The spend policy is **code** (`kit_mcp::SpendPolicy`, read-only default).
Allowing state-changing MCP calls is a deliberate code change by the
operator, not an env flip — and payments have no code path at all.

## HTTP service

| Variable | Effect |
|---|---|
| `KIT_HTTP_LIVE` | `1` lets `kit-http` execute gate-cleared consequential calls. Anything else (including unset) = dry-run service. |

## Constitution

| Variable | Effect |
|---|---|
| `CLAWD_CONSTITUTION_PATH` | Path to an installed constitution file. Otherwise the kit searches `~/.automaton/`, `~/.clawd/`, `./constitution/`, and falls back to the byte-for-byte inline text. |

## Logging

`RUST_LOG` (e.g. `info`, `debug`) drives `tracing` output, including the
gate's per-evaluation judgment records.
