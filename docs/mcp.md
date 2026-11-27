# MCP: Clawd Compute + PumpAI

`kit-mcp` is a Model Context Protocol client (JSON-RPC over streamable
HTTP: `initialize`, `tools/list`, `tools/call`, with session handling and
SSE-or-JSON response parsing) preconfigured for the fleet's two servers.

## The servers (live-verified 2026-11-27)

| Server | Endpoint | Identity | Tools |
|---|---|---|---|
| Clawd Compute gateway | `https://mcp.clawdcompute.us/mcp` | `clawd-compute-mcp-portal` v1.0.0, stateless | **272** |
| PumpAI | `https://pumpai.ai/mcp` | `clawd-pump` (PumpAI) v2.4.0-clawd, session-based | **213** |

The gateway's 272 = 2 portal tools + the full 213-tool PumpAI mirror +
equities (22) + desk (5) + backpack (15) + massive (12) + ai (3).
Endpoints are overridable via `CLAWD_COMPUTE_MCP_URL` / `PUMPAI_MCP_URL`.

One operational quirk worth knowing: the gateway's edge blocks
non-browser user agents (Cloudflare error 1010), so the client sends a
normal UA string identifying itself as OpenClawdKit.

## Namespacing

Tools register into the kit registry as `<server>__<tool>` —
`clawdcompute__portal_identity`, `pumpai__get_health`, … — so both
catalogs coexist without collisions, and the reasoning loop (and Jev
gate) treat them like any other tool.

## The spend policy (fail-closed)

The gateway charges **x402, $0.01 per call**, for its payable tools.
Paying is a custody act, so the client is built so that it *cannot* pay:

- Tools are classified from MCP annotations. `readOnlyHint == true` →
  read-only: callable under every policy, not gated as consequential.
- Anything else — including **no annotation** — is state-changing:
  treated as consequential for the Jev gate, and refused outright unless
  the operator explicitly opted in with
  `SpendPolicy { allow_state_changing_calls: true }` (a code change, not
  an env var).
- A `402 Payment Required` response surfaces as
  `ToolError::PaymentRequired`. There is no wallet, no signer, and no
  payment code path in `kit-mcp`; `payments_enabled` is a reserved flag
  for a future host-provided payment handler and changes nothing today.

Net effect out of the box: the full 485-tool catalog is listable, the
read-only majority is callable, and nothing can spend, sign, or trade.

## Trying it

```bash
cargo run -p kit-cli -- mcp list
cargo run -p kit-cli -- mcp call pumpai__get_health '{}'
```
