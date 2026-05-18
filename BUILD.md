# Prime Build Guide

## Prerequisites

```bash
# Rust (stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js 20+
# Windows: https://nodejs.org
# Linux/macOS: nvm install 20

# Tauri CLI
cargo install tauri-cli --version "^2.0"
```

## First Build

```bash
# 1. Install frontend dependencies
npm install

# 2. Build frontend
npm run build

# 3. Build Rust backend (this will download all dependencies)
cargo build --release

# 4. Run
cargo tauri dev
```

## Build Targets

| Target | Command | Output |
|--------|---------|--------|
| Development | `cargo tauri dev` | Hot-reload app |
| Release | `cargo build --release` | `target/release/prime` |
| Windows MSI | `cargo tauri build` | `target/release/bundle/msi/` |
| macOS DMG | `cargo tauri build` | `target/release/bundle/dmg/` |
| Linux deb | `cargo tauri build` | `target/release/bundle/deb/` |

## Verification Pipeline

```bash
# 1. Type check (fastest)
cargo check

# 2. Lint
cargo clippy -- -D warnings

# 3. Test
cargo test

# 4. Full build
cargo build --release

# 5. Frontend checks
npx tsc --noEmit
npm run lint
```

## Project Structure

```
prime/
├── src-tauri/src/          # Rust backend
│   ├── contracts/          # Interface definitions (anti-spaghetti)
│   ├── core/               # Tokio runtime, WASM, storage, gRPC
│   ├── memory/             # 7 memory tiers
│   ├── ai/                 # Multi-model router
│   ├── mcp/                # 12 MCP servers
│   ├── skills/             # WASM plugin system
│   ├── execution/          # Sandbox execution engine
│   ├── verification/       # Lint, test, self-heal
│   ├── browser/            # Playwright automation
│   ├── code_intel/         # Tree-sitter parsing, symbols, search
│   ├── arch/               # Event bus, actors, DAG, scheduler
│   ├── security/           # Sandboxing, encryption
│   ├── dev/                # Indexing, agents, live-reload
│   ├── observability/      # Metrics, tracing, telemetry
│   ├── proxy/              # Proxy pool & rotation
│   └── tools/              # Tools registry (145+ tools)
├── src/                    # React frontend
│   ├── components/ui/      # shadcn/ui components
│   ├── hooks/              # React hooks
│   ├── stores/             # Zustand stores
│   └── types/              # TypeScript types
└── skills/examples/        # Example WASM skills
```

## Architecture Rules

1. **No module imports another module's internal types.** Only use `contracts::*`.
2. **All cross-module communication via EventBus** or contract traits.
3. **Observability is mandatory** - every operation must record metrics.
4. **Skills must be sandboxed** - never run untrusted code outside sandbox.
5. **MCP is the external API** - everything accessible via MCP.
