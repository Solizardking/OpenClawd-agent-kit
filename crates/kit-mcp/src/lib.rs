//! kit-mcp — MCP client for the OpenClawd agent kit.
//!
//! Ships with three built-in servers preconfigured
//! ([`client::McpClient::fleet`]): the Clawd Compute gateway
//! (272 tools, live-verified 2026-11-27), PumpAI (213 tools), and the
//! Solana Developer MCP at `mcp.solana.com` (5 tools, docs-class).
//! All calls run behind the fail-closed [`toolset::SpendPolicy`]:
//! read-only by default, and a 402 is an error — never a payment.

pub mod client;
pub mod toolset;

pub use client::{McpClient, McpTool, ServerInfo, CLAWD_COMPUTE_MCP_URL, PUMPAI_MCP_URL, SOLANA_MCP_URL};
pub use toolset::{register_mcp_servers, NamespacedMcpTool, SpendPolicy};
