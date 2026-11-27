#!/bin/sh
# OpenClawd Agent Kit — installer
#
#   curl -fsSL https://clawdcompute.us/install.sh | sh
#
# or from a clone of the kit:
#
#   ./install.sh
#
# What it does, in order:
#   1. Builds and installs the `kit` CLI from this repository
#      (into ~/.local/bin — no sudo, nothing system-wide).
#   2. Installs the bundled skills — solana-dev, certora-solana-fv,
#      harmonic-aristotle — into ~/.agents/skills and ~/.claude/skills,
#      the two directories the agent ecosystem reads.
#   3. Builds the Clawd Connectors MCP server (clawd-connectors-2)
#      under ~/.openclawd/ and registers it, plus the hosted MCP fleet
#      (Clawd Compute gateway, PumpAI, Solana Developer MCP), in your
#      MCP client config (~/.claude.json, merged — never clobbered).
#   4. Verifies everything before declaring success: the gateway
#      answers an MCP handshake, `kit mcp list` sees the fleet, the
#      skills are on disk, the config is written.
#
# No sudo. No secret prompts. Writes only inside your HOME.
# Idempotent: re-running converges to the same state.
#
# Source: https://github.com/Solizardking/OpenClawd-agent-kit

set -eu

KIT_REPO_URL="${OPENCLAWD_KIT_REPO:-https://github.com/Solizardking/OpenClawd-agent-kit.git}"
CONNECTORS_REPO_URL="${OPENCLAWD_CONNECTORS_REPO:-https://github.com/Solizardking/clawd-connectors-2.git}"
STATE_DIR="$HOME/.openclawd"
INSTALL_ROOT="$HOME/.local"
KIT_BIN="$INSTALL_ROOT/bin/kit"
GATEWAY_URL="https://mcp.clawdcompute.us/mcp"
PUMPAI_URL="https://pumpai.ai/mcp"
SOLANA_URL="https://mcp.solana.com/mcp"
SKILLS="solana-dev certora-solana-fv harmonic-aristotle"

say()  { printf '%s\n' "$*"; }
step() { printf '\n==> %s\n' "$*"; }
warn() { printf 'install: warning: %s\n' "$*" >&2; }
die()  { printf 'install: error: %s\n' "$*" >&2; exit 1; }

usage() {
    say "OpenClawd Agent Kit installer"
    say ""
    say "Usage: install.sh [--help]"
    say ""
    say "Installs the kit CLI, the bundled skills, and the Clawd Compute"
    say "MCP connections into your HOME. No sudo, no secrets, idempotent."
    say ""
    say "Environment overrides:"
    say "  OPENCLAWD_KIT_REPO         kit git URL (default: GitHub)"
    say "  OPENCLAWD_CONNECTORS_REPO  connectors git URL (default: GitHub)"
}

case "${1:-}" in
    "") ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (try --help)" ;;
esac

# ---------------------------------------------------------------- preflight
step "Preflight"
case "$(uname -s)" in
    Linux|Darwin) ;;
    *) die "this installer supports Linux and macOS; found $(uname -s)" ;;
esac

if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
fi
command -v cargo >/dev/null 2>&1 || die "cargo not found. Install Rust first via rustup:
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
then re-run this installer."
command -v curl >/dev/null 2>&1 || die "curl not found — it is needed to verify the Clawd Compute connection."
say "cargo: $(cargo --version)"

# ------------------------------------------------------- resolve kit source
step "Getting the kit source"
KIT_SRC=""
case "$0" in
    */*)
        _d=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || _d=""
        if [ -n "$_d" ] && [ -f "$_d/Cargo.toml" ] && [ -d "$_d/crates/kit-cli" ] && [ -d "$_d/skills" ]; then
            KIT_SRC="$_d"
        fi
        ;;
esac

if [ -n "$KIT_SRC" ]; then
    say "Running from a kit checkout: $KIT_SRC"
else
    command -v git >/dev/null 2>&1 || die "git not found — needed to download the kit.
Install git, or clone https://github.com/Solizardking/OpenClawd-agent-kit and run ./install.sh from the clone."
    KIT_SRC="$STATE_DIR/agent-kit"
    mkdir -p "$STATE_DIR"
    if [ -d "$KIT_SRC/.git" ]; then
        say "Updating the kit checkout in $KIT_SRC"
        git -C "$KIT_SRC" pull --ff-only >/dev/null 2>&1 || \
            warn "could not fast-forward the checkout (offline?); continuing with what is on disk"
    elif [ -d "$KIT_SRC" ]; then
        die "$KIT_SRC exists but is not a git checkout — move it aside and re-run"
    else
        say "Downloading the OpenClawd agent kit to $KIT_SRC"
        git clone --depth 1 "$KIT_REPO_URL" "$KIT_SRC" || die "git clone of $KIT_REPO_URL failed"
    fi
fi

# ------------------------------------------------------------ build the CLI
step "Building the kit CLI (this takes a few minutes the first time)"
( cd "$KIT_SRC" && cargo install --path crates/kit-cli --locked --force --root "$INSTALL_ROOT" ) || \
    die "cargo could not build/install the kit CLI — see the cargo output above"
[ -x "$KIT_BIN" ] || die "the build finished but $KIT_BIN is missing"
say "kit CLI installed: $KIT_BIN"

# ------------------------------------------------------------------- skills
step "Installing skills"
for _s in $SKILLS; do
    _src="$KIT_SRC/skills/$_s"
    [ -f "$_src/SKILL.md" ] || die "bundled skill '$_s' is missing from $KIT_SRC/skills"
    for _base in "$HOME/.agents/skills" "$HOME/.claude/skills"; do
        mkdir -p "$_base"
        rm -rf "${_base:?}/$_s"
        cp -R "$_src" "$_base/$_s"
        say "  $_s -> $_base/$_s"
    done
done

# ------------------------------------------------- Clawd Connectors (local)
CONNECTORS_DIR="$STATE_DIR/clawd-connectors"
CONNECTORS_SERVER=""
step "Installing the Clawd Connectors MCP server"
if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
    if [ -d "$CONNECTORS_DIR/.git" ]; then
        say "Updating the connectors checkout in $CONNECTORS_DIR"
        git -C "$CONNECTORS_DIR" pull --ff-only >/dev/null 2>&1 || \
            warn "could not fast-forward the connectors checkout; continuing with what is on disk"
    elif [ ! -d "$CONNECTORS_DIR" ]; then
        say "Downloading Clawd Connectors to $CONNECTORS_DIR"
        git clone --depth 1 "$CONNECTORS_REPO_URL" "$CONNECTORS_DIR" || die "git clone of $CONNECTORS_REPO_URL failed"
    fi
    ( cd "$CONNECTORS_DIR" && npm install --no-audit --no-fund && npm run build ) || \
        die "could not build Clawd Connectors — see the npm output above"
    [ -f "$CONNECTORS_DIR/dist/mcp/server.js" ] || die "connectors build did not produce dist/mcp/server.js"
    CONNECTORS_SERVER="$CONNECTORS_DIR/dist/mcp/server.js"
    say "Connectors MCP server built: $CONNECTORS_SERVER"
else
    warn "node/npm not found — skipping the local Clawd Connectors server."
    warn "The hosted Clawd Compute gateway (registered below) still gives your agent the full 272-tool surface."
    warn "Install Node.js 20+ and re-run this installer to add the local connectors."
fi

# ------------------------------------------------------------- MCP config
step "Connecting your agent to Clawd Compute (MCP config)"
MERGE_PY="$STATE_DIR/.merge_mcp.py"
mkdir -p "$STATE_DIR"
cat > "$MERGE_PY" <<'PYEOF'
import json, os, sys

path, gateway, pumpai, solana, connectors = sys.argv[1:6]
desired = {
    "clawdcompute": {"type": "http", "url": gateway},
    "pumpai": {"type": "http", "url": pumpai},
    "solana-dev": {"type": "http", "url": solana},
}
if connectors:
    desired["clawd-connectors"] = {
        "type": "stdio", "command": "node", "args": [connectors],
    }

data = {}
if os.path.exists(path):
    with open(path, encoding="utf-8") as f:
        raw = f.read()
    if raw.strip():
        try:
            data = json.loads(raw)
        except json.JSONDecodeError:
            print(f"INVALID_JSON {path}", file=sys.stderr)
            sys.exit(3)
    if not isinstance(data, dict):
        print(f"INVALID_JSON {path}", file=sys.stderr)
        sys.exit(3)

servers = data.setdefault("mcpServers", {})
if not isinstance(servers, dict):
    print(f"INVALID_JSON {path}", file=sys.stderr)
    sys.exit(3)

before = set(servers)
for name, cfg in desired.items():
    servers[name] = cfg
kept = sorted(before - set(desired))
os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
tmp = path + ".tmp-openclawd"
with open(tmp, "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
os.replace(tmp, path)
print("registered: " + ", ".join(sorted(desired)))
if kept:
    print("left untouched: " + ", ".join(kept))
PYEOF

merge_config() {
    _target="$1"
    if command -v python3 >/dev/null 2>&1; then
        _out=$(python3 "$MERGE_PY" "$_target" "$GATEWAY_URL" "$PUMPAI_URL" "$SOLANA_URL" "$CONNECTORS_SERVER" 2>&1) || {
            case "$_out" in
                INVALID_JSON*) die "$_target exists but is not valid JSON — fix or move it, then re-run. Nothing was written to it." ;;
                *) die "could not update $_target" ;;
            esac
        }
        say "  $_target — $_out" | tr '\n' ' ' ; say ""
    elif [ ! -e "$_target" ]; then
        # No python3 and no existing file: write a fresh config directly.
        {
            printf '{\n  "mcpServers": {\n'
            printf '    "clawdcompute": {"type": "http", "url": "%s"},\n' "$GATEWAY_URL"
            printf '    "pumpai": {"type": "http", "url": "%s"},\n' "$PUMPAI_URL"
            printf '    "solana-dev": {"type": "http", "url": "%s"}' "$SOLANA_URL"
            if [ -n "$CONNECTORS_SERVER" ]; then
                printf ',\n    "clawd-connectors": {"type": "stdio", "command": "node", "args": ["%s"]}\n' "$CONNECTORS_SERVER"
            else
                printf '\n'
            fi
            printf '  }\n}\n'
        } > "$_target"
        say "  $_target — written (fresh)"
    else
        rm -f "$MERGE_PY"
        die "$_target already exists and python3 is not available to merge into it safely.
Install python3 and re-run, or add these entries to \"mcpServers\" by hand:
  \"clawdcompute\": {\"type\": \"http\", \"url\": \"$GATEWAY_URL\"}
  \"pumpai\": {\"type\": \"http\", \"url\": \"$PUMPAI_URL\"}
  \"solana-dev\": {\"type\": \"http\", \"url\": \"$SOLANA_URL\"}"
    fi
}

merge_config "$HOME/.claude.json"

if [ "$(uname -s)" = "Darwin" ]; then
    merge_config "$HOME/Library/Application Support/Claude/claude_desktop_config.json"
fi

# A standalone copy any other MCP client can be pointed at.
{
    printf '{\n  "mcpServers": {\n'
    printf '    "clawdcompute": {"type": "http", "url": "%s"},\n' "$GATEWAY_URL"
    printf '    "pumpai": {"type": "http", "url": "%s"},\n' "$PUMPAI_URL"
    printf '    "solana-dev": {"type": "http", "url": "%s"}' "$SOLANA_URL"
    if [ -n "$CONNECTORS_SERVER" ]; then
        printf ',\n    "clawd-connectors": {"type": "stdio", "command": "node", "args": ["%s"]}\n' "$CONNECTORS_SERVER"
    else
        printf '\n'
    fi
    printf '  }\n}\n'
} > "$STATE_DIR/mcp.json"
say "  $STATE_DIR/mcp.json — standalone copy for any other MCP client"
rm -f "$MERGE_PY"

# -------------------------------------------------------------- verification
step "Verifying the install"

"$KIT_BIN" constitution >/dev/null 2>&1 || die "the kit CLI was installed but does not run ($KIT_BIN constitution failed)"
say "  kit CLI runs"

_init='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"openclawd-installer","version":"1"}}}'
_resp=$(curl -fsS -m 30 -X POST "$GATEWAY_URL" \
    -H 'Content-Type: application/json' \
    -H 'Accept: application/json, text/event-stream' \
    -d "$_init") || die "could not reach the Clawd Compute gateway at $GATEWAY_URL — check your network connection and re-run"
case "$_resp" in
    *clawd-compute-mcp-portal*) say "  Clawd Compute gateway answered the MCP handshake" ;;
    *) die "the gateway at $GATEWAY_URL answered, but not with the expected MCP server identity" ;;
esac

_list=$("$KIT_BIN" mcp list 2>/dev/null) || die "'kit mcp list' failed to run"
printf '%s\n' "$_list" | sed 's/^/  /'
_gw=$(printf '%s\n' "$_list" | sed -n 's/^clawdcompute: \([0-9][0-9]*\) tools.*$/\1/p')
[ -n "$_gw" ] && [ "$_gw" -gt 0 ] || die "kit could not list tools on the Clawd Compute gateway"
say "  Clawd Compute gateway: $_gw tools via the kit CLI"

for _s in $SKILLS; do
    for _base in "$HOME/.agents/skills" "$HOME/.claude/skills"; do
        [ -f "$_base/$_s/SKILL.md" ] || die "skill $_s is missing from $_base after install"
    done
done
say "  skills on disk: $SKILLS (in ~/.agents/skills and ~/.claude/skills)"

grep -q 'mcp.clawdcompute.us' "$HOME/.claude.json" || die "~/.claude.json does not contain the Clawd Compute gateway entry after merge"
say "  MCP config contains the gateway entry"

# ------------------------------------------------------------------ summary
step "Done — OpenClawd agent kit installed"
say "  kit CLI:        $KIT_BIN"
say "  skills:         ~/.agents/skills + ~/.claude/skills ($SKILLS)"
say "  MCP servers:    clawdcompute ($_gw tools), pumpai, solana-dev${CONNECTORS_SERVER:+ , clawd-connectors (local)}"
say "  config written: ~/.claude.json (merged), $STATE_DIR/mcp.json"
case ":$PATH:" in
    *":$INSTALL_ROOT/bin:"*) ;;
    *) say ""
       say "  One last thing: ~/.local/bin is not on your PATH. Add it with:"
       say "    export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
esac
say ""
say "Try it:  kit mcp list        # see the fleet your agent can call"
say "         kit demo            # the gated reasoning loop, dry-run"
