set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# sdrcraft-mcp automation (ArtCraft pattern).
# Every recipe prefixes cargo via USERPROFILE so NSSM / IDE shells work
# even when .cargo/bin is not on the inherited PATH.

default:
    @just --list

# Strict ArtCraft lint gate across all workspace members
check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo clippy --workspace --all-targets --all-features -- -D warnings

# Unit + integration tests
test:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo test --workspace

# Formatting gate
fmt-check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo fmt --all --check

# Start the MCP server locally over stdio (logs on stderr, JSON-RPC on stdout)
run-mcp:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p sdrcraft-mcp -- mcp-stdio

# CLI smoke: status without hardware
run-cli:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p sdrcraft-mcp -- status

# Headless app smoke (no window)
run-smoke:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p sdrcraft-app -- --smoke

# Production build (GUI + MCP binaries)
build-release:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo build --release --workspace
