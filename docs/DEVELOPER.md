# Prime — Developer Guide

> Prime is a Tauri 2 desktop application that serves as an AI-powered developer platform.
> This guide covers architecture, extension points, and how to contribute.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Project Structure](#project-structure)
3. [How to Create a Skill](#how-to-create-a-skill)
4. [How to Add an MCP Server](#how-to-add-an-mcp-server)
5. [How to Add a Model Provider](#how-to-add-a-model-provider)
6. [How to Add a Tauri Command](#how-to-add-a-tauri-command)
7. [Frontend Development](#frontend-development)
8. [Configuration Reference](#configuration-reference)
9. [Testing](#testing)
10. [Building & Packaging](#building--packaging)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    FRONTEND (React + Vite)               │
│  Components  →  Stores (Zustand)  →  Tauri IPC invoke   │
├─────────────────────────────────────────────────────────┤
│                    TAURI IPC LAYER                        │
│  lib.rs  →  Commands  →  State management               │
├─────────────────────────────────────────────────────────┤
│                    RUST BACKEND                           │
│                                                           │
│  contracts/     Interface definitions (traits + types)   │
│  core/          Runtime, storage, WASM, gRPC             │
│  arch/          Event bus, actors, DAG scheduler         │
│  memory/        Working, episodic, semantic, vector      │
│  ai/            Model router, streaming, embeddings      │
│  mcp/           9 built-in MCP servers                   │
│  skills/        WASM plugin system                       │
│  execution/     Sandbox execution engine                 │
│  verification/  Lint, test, review, self-heal            │
│  browser/       Playwright automation                    │
│  security/      Sandboxing, encryption, permissions      │
│  dev/           Indexing, agents, live-reload            │
│  observability/ Metrics, tracing, telemetry              │
│  code_intel/    Tree-sitter, Tantivy search, symbols     │
│  proxy/         Proxy pool & rotation                    │
│  tools/         Tools registry (145+ tools, 22 categories)│
└─────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **No circular dependencies**: Modules communicate via `contracts::` traits
2. **Event-driven**: Cross-module communication goes through the EventBus
3. **Sandboxed**: All skill/plugin code runs in isolated WASM sandboxes
4. **Observable**: Every operation records metrics and traces
5. **MCP-first**: All capabilities are accessible via MCP protocol

---

## Project Structure

```
D:\temp\prime/
├── src/                          # Frontend (React + TypeScript)
│   ├── App.tsx                   # Main app with routing
│   ├── main.tsx                  # React entry point
│   ├── index.css                 # Global styles + CSS variables
│   ├── components/               # UI components
│   │   ├── ui/                   # shadcn/ui primitives
│   │   ├── WorkflowPanel.tsx     # Workflow management UI
│   │   ├── PluginManager.tsx     # Plugin/skill management UI
│   │   ├── MemoryViewer.tsx      # Memory browsing UI
│   │   ├── EventTimeline.tsx     # Real-time event stream
│   │   ├── LogsViewer.tsx        # Log viewer
│   │   ├── ModelManager.tsx      # AI model management
│   │   ├── SettingsPanel.tsx     # App settings
│   │   └── OnboardingWizard.tsx  # First-run setup
│   ├── hooks/                    # Custom React hooks
│   │   └── useTheme.ts          # Theme management
│   ├── stores/                   # Zustand state stores
│   │   ├── useAppStore.ts       # App-level state
│   │   ├── useMemoryStore.ts    # Memory state
│   │   ├── useMcpStore.ts       # MCP state
│   │   ├── useWorkflowStore.ts  # Workflow state
│   │   ├── usePluginStore.ts    # Plugin state
│   │   ├── useModelStore.ts     # Model state
│   │   └── useToolsStore.ts     # Tools registry state
│   ├── types/                    # TypeScript type definitions
│   │   └── index.ts
│   └── lib/                      # Utility functions
│       └── utils.ts
├── src-tauri/                    # Rust backend (Tauri 2)
│   ├── src/
│   │   ├── lib.rs               # Tauri commands + app setup
│   │   ├── main.rs              # Entry point
│   │   ├── contracts/           # Interface definitions
│   │   ├── core/                # Core runtime, storage
│   │   ├── arch/                # Event bus, actors, DAG
│   │   ├── memory/              # Memory systems
│   │   ├── ai/                  # AI model router
│   │   ├── mcp/                 # MCP servers
│   │   ├── skills/              # WASM plugin system
│   │   ├── execution/           # Sandbox execution
│   │   ├── verification/        # Code review, lint, test
│   │   ├── browser/             # Playwright automation
│   │   ├── security/            # Encryption, sandboxing
│   │   ├── dev/                 # Indexing, agents, live-reload
│   │   ├── observability/       # Metrics, tracing
│   │   ├── code_intel/          # Tree-sitter, search
│   │   ├── proxy/               # Proxy pool & rotation
│   │   └── tools/               # Tools registry (145+ tools)
│   ├── tauri.conf.json          # Tauri configuration
│   └── Cargo.toml               # Rust dependencies
├── skills/                       # Skill plugins directory
├── scripts/                      # Build + utility scripts
│   ├── build.ps1                # Build script
│   ├── dev.ps1                  # Development server
│   ├── build_env.ps1            # Environment setup
│   ├── portable.ps1             # Portable distribution
│   ├── crash_recovery.ps1       # Crash recovery tool
│   ├── scaffold-skill.ps1       # Skill scaffolding CLI
│   ├── security-audit.ps1       # Security audit tool
│   └── install-tools.ps1        # Tools registry installer
├── tests/                        # Rust integration tests
│   ├── integration_tests.rs     # Integration tests
│   └── benchmarks.rs            # Performance benchmarks
├── docs/                         # Documentation
│   ├── MASTER_PLAN.md           # Project master plan
│   ├── DEVELOPER.md             # This file
│   ├── TOOLS_REGISTRY.md        # Tools registry reference
│   └── VERIFICATION_REPORT.md   # Tools audit report
├── package.json                  # Node.js dependencies
├── tsconfig.json                 # TypeScript configuration
├── vite.config.ts                # Vite configuration
├── tailwind.config.ts            # Tailwind CSS configuration
└── components.json               # shadcn/ui configuration
```

---

## How to Create a Skill

### Using the CLI

```powershell
.\scripts\scaffold-skill.ps1 -Name "my-skill" -Language "python" -Description "Does something useful"
```

### Manual Creation

1. Create a directory in `skills/`:
```
skills/my-skill/
├── manifest.toml
├── my_skill.py       # or .rs, .ts, .js
├── tests/
│   └── test_my_skill.py
└── examples/
    └── example.txt
```

2. Create `manifest.toml`:
```toml
[skill]
id = "my-skill"
name = "My Skill"
version = "0.1.0"
description = "Does something useful"
language = "python"
entry = "my_skill.py"

[author]
name = "Your Name"

[permissions]
filesystem_read = false
filesystem_write = false
network = false
shell = false

[dependencies]
# List other skills this depends on
```

3. Implement the entry function:
```python
# my_skill.py
import json

def execute(input_data: str) -> str:
    params = json.loads(input_data)
    # Your logic here
    return json.dumps({"status": "success", "result": params})
```

4. The skill is automatically discovered on next startup.

### Skill API

Every skill must export an `execute(input: str) -> str` function:
- **input**: JSON string with parameters
- **return**: JSON string with results
- Errors should be returned as `{"status": "error", "message": "..."}`

---

## How to Add an MCP Server

### Built-in MCP Server

1. Create a new module in `src-tauri/src/mcp/`:
```rust
// src-tauri/src/mcp/my_server.rs
use async_trait::async_trait;
use crate::contracts::mcp::McpServer;

pub struct MyMcpServer;

#[async_trait]
impl McpServer for MyMcpServer {
    fn id(&self) -> &str { "my-server" }
    fn name(&self) -> &str { "My Server" }
    
    async fn handle_request(&self, req: McpRequest) -> McpResult {
        // Handle the request
        Ok(McpResponse { ... })
    }
}
```

2. Register it in `src-tauri/src/mcp/mod.rs`:
```rust
pub mod my_server;
```

3. Register in `src-tauri/src/lib.rs`:
```rust
mc.register(Arc::new(mcp::my_server::MyMcpServer::new())).await;
```

### External MCP Server

Add to `tauri.conf.json` or runtime config:
```json
{
  "mcp_servers": [
    {
      "id": "my-external-server",
      "name": "My External Server",
      "command": "npx",
      "args": ["-y", "@my/mcp-server"],
      "env": {
        "API_KEY": "${env:MY_API_KEY}"
      }
    }
  ]
}
```

---

## How to Add a Model Provider

1. Create a provider implementation in `src-tauri/src/ai/providers/`:
```rust
// src-tauri/src/ai/providers/my_provider.rs
use async_trait::async_trait;
use crate::contracts::ai::{ModelProvider, ChatRequest, ChatResponse};

pub struct MyProvider {
    api_key: String,
    model: String,
}

#[async_trait]
impl ModelProvider for MyProvider {
    fn id(&self) -> &str { "my-provider" }
    fn model_name(&self) -> &str { &self.model }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        // Implement chat completion
    }
}
```

2. Register in `src-tauri/src/ai/router.rs`:
```rust
router.register_provider(Arc::new(MyProvider::new(api_key, model)));
```

3. Add model config:
```json
{
  "models": [
    {
      "id": "my-model",
      "provider": "my-provider",
      "model": "my-model-v1",
      "max_tokens": 4096,
      "temperature": 0.7,
      "streaming": true
    }
  ]
}
```

---

## How to Add a Tauri Command

1. Add the command function in `src-tauri/src/lib.rs`:
```rust
#[tauri::command]
async fn my_command(
    state: tauri::State<'_, Arc<core::Runtime>>,
    param: String,
) -> Result<String, AppError> {
    // Implementation
    Ok("result".to_string())
}
```

2. Add frontend type in `src/types/index.ts`:
```typescript
export interface MyCommandResult {
    // ...
}
```

3. Call from frontend:
```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke<string>("my_command", { param: "value" });
```

---

## Frontend Development

### Tech Stack
- **Framework**: React 18 + TypeScript
- **Build**: Vite 5
- **Styling**: Tailwind CSS 3 + shadcn/ui
- **State**: Zustand 4 + TanStack Query 5
- **Desktop**: Tauri 2 API

### Component Conventions
- Each panel gets its own file in `src/components/`
- Stores are in `src/stores/` with `use*Store` naming
- Custom hooks in `src/hooks/`
- All types in `src/types/index.ts`
- shadcn/ui primitives in `src/components/ui/`

### State Management Pattern
```
User Action → Component → Store Action → Tauri invoke
                                              ↓
Component ← Store Update ← Response ← Rust command
```

### Adding a New Panel

1. Add the panel type to `App.tsx`:
```typescript
type Panel = "dashboard" | "chat" | ... | "my-panel";
```

2. Add nav item:
```typescript
const NAV_ITEMS = [
  { id: "my-panel", label: "My Panel", icon: MyIcon },
  // ...
];
```

3. Create component in `src/components/MyPanel.tsx`

4. Add render case in `renderPanel()`:
```typescript
case "my-panel": return <MyPanel />;
```

---

## Configuration Reference

### `tauri.conf.json`

| Key | Description |
|-----|-------------|
| `app.windows[].title` | Window title |
| `app.windows[].width` | Default width |
| `app.windows[].height` | Default height |
| `app.security.csp` | Content Security Policy |
| `bundle.active` | Enable bundling |
| `bundle.targets` | Bundle targets ("all", "msi", "nsis", "dmg", "deb", "appimage") |
| `plugins.updater.endpoints` | Update check URLs |
| `plugins.updater.pubkey` | Update signature public key |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `TAURI_DEV_HOST` | Dev server host (for mobile/network dev) |
| `PRIME_RECOVERY_MODE` | Enable crash recovery mode |
| `PRIME_DATA_DIR` | Override data directory |
| `RUST_LOG` | Rust log level (e.g., `prime=debug`) |

---

## Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration_tests -- --nocapture

# Run benchmarks (slow, release mode recommended)
cargo test --release --test benchmarks -- --nocapture --ignored

# Run with output
cargo test -- --nocapture

# Frontend type check
npm run typecheck

# Frontend lint
npm run lint
```

---

## Building & Packaging

```bash
# Development
npm run tauri dev

# Production build
.\scripts\build.ps1

# Portable distribution
.\scripts\portable.ps1

# Crash recovery
.\scripts\crash_recovery.ps1 -Mode run

# Security audit
.\scripts\security-audit.ps1

# Scaffold a new skill
.\scripts\scaffold-skill.ps1 -Name "my-skill" -Language "python"
```

---

## Supervisor Heartbeat System

The Supervisor is an internal observer that monitors AI agent execution in real time. It detects loops, stalls, hallucination, token explosion, and timeouts — then automatically intervenes via context reset, correction prompts, model switching, or kill.

### Architecture
```
Agent ──(mpsc)──▶ Supervisor ──(watchdog)──▶ Intervention
```

### Detection
| Issue | How | Threshold |
|-------|-----|-----------|
| Loop | Identical output ≥ N times | 3 |
| Stall | Same step > timeout | 30s |
| Hallucination | Objective key-words missing from output | 0/3+ matches |
| Token Explosion | Tokens doubling each step × 3 | 32K max |
| Timeout | No heartbeat > interval | 15s / 5min |

### Tauri Commands
- `supervisor_start` — spawns the run loop
- `supervisor_stats` — returns `SupervisorStats`
- `supervisor_stop` — graceful shutdown

### File Locations
- Core: `prime_core/src/core/supervisor.rs` (1026 lines, 12 tests)
- Bridge: `src-tauri/src/lib.rs` (SupervisorState + 3 commands)
- Tests: `cargo test -p prime_core -- core::supervisor`

### Details
See [SUPERVISOR.md](./SUPERVISOR.md) for full reference: builder, heartbeats, correction prompts, and usage examples.

---

## Connections & Config-Wired MCP

10 connection types (telegram, telegram_bot, whatsapp, discord, slack, email, wechat, signal, matrix, irc) are configurable from Settings → Connections. Each stores `HashMap<String, String>` fields in `UserConfig.connection_configs`.

### MCP Servers with Config
| Server | Config Fields | Env Fallback |
|--------|--------------|--------------|
| TelegramMcp | `bot_token`, `session_file` | `TELEGRAM_BOT_TOKEN`, `TELEGRAM_SESSION` |
| WhatsAppMcp | `api_key`, `api_url`, `phone_number_id` | `WHATSAPP_API_KEY`, `WHATSAPP_API_URL`, `WHATSAPP_PHONE_NUMBER_ID` |

MCP constructors accept `with_config(&fields)` for config-driven initialization with env var fallback. Registration in `setup()` loads config via `load_config_inner()` before spawning.

### Details
See [CONNECTIONS.md](./CONNECTIONS.md) for full field reference and adding new connection types.

---

## Further Reading

- [SUPERVISOR.md](./SUPERVISOR.md) — Supervisor Heartbeat System reference
- [CONNECTIONS.md](./CONNECTIONS.md) — Connection integration guide
- [MASTER_PLAN.md](./MASTER_PLAN.md) — Full project roadmap
- [BUILD.md](../BUILD.md) — Build instructions and requirements
- Tauri 2 docs: https://v2.tauri.app/
- shadcn/ui: https://ui.shadcn.com/
- Zustand: https://github.com/pmndrs/zustand
