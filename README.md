# Prime

**The Native Desktop AI Agent Platform**

Created by [Aly Ghaly](https://github.com/alyghaly2020-ux)

Parallel multi-model orchestration · Computer Use · 93 specialized agents · 145+ tools · 12 MCP servers · 7-tier memory · 9 language UI

---

## What is Prime?

Prime is a native desktop operating system for AI agents — a single app that replaces CLI tools, VS Code extensions, Python libraries, and cloud dashboards.

- **Rust + Tauri v2** backend with **React/TypeScript** frontend
- Runs on **Windows, macOS, and Linux**
- Agents have the same desktop access you do: browser, terminal, code editor, files
- 100% local — no cloud dependency, no telemetry

## Quick Start

### One-liner install

**macOS & Linux**
```bash
curl -fsSL https://raw.githubusercontent.com/alyghaly2020-ux/prime/master/install.sh | bash
```

**Windows**
```powershell
powershell -c "irm https://raw.githubusercontent.com/alyghaly2020-ux/prime/master/install.ps1 | iex"
```

### From source

```bash
git clone https://github.com/alyghaly2020-ux/prime.git
cd prime
npm install
npm run dev           # Terminal 1 — Vite dev server
cargo build -p prime  # Terminal 2 — Rust backend
```

### Run tests

```bash
cargo test
npx tsc --noEmit
npm run lint
```

## Key Features

| Feature | Description |
|---------|-------------|
| Parallel AI Orchestration | Run multiple models simultaneously across Ollama, OpenAI, Anthropic |
| Computer Use | Agents control mouse, keyboard, screenshots via enigo + image |
| 93 Specialized Agents | Pre-built agents across 25 domains |
| 145+ Tools | Config-driven tools registry across 25 categories |
| 12 MCP Servers | Built-in Model Context Protocol servers |
| 7-Tier Memory | Working → Episodic → Semantic → Vector → RAG → Cache → Compression |
| WASM Plugins | Cryptographically signed plugin sandbox |
| 9 Language UI | Full i18n with RTL support |

## Architecture

```
┌─────────────────────────────────────────────┐
│           REACT/TYPESCRIPT FRONTEND          │
│  Chat · Dashboard · IDE · Settings · Memory  │
├─────────────────────────────────────────────┤
│              TAURI IPC LAYER                 │
│            (84 commands)                     │
├─────────────────────────────────────────────┤
│            RUST BACKEND                      │
│  AI · MCP · Memory · Security · Computer Use │
│  Browser · Code Intel · Execution · Skills   │
└─────────────────────────────────────────────┘
```

## Project Structure

```
prime/
├── src/              # React/TypeScript frontend
├── src-tauri/        # Rust backend + Tauri config
├── prime_core/       # Core library crate
├── docs/             # Documentation
├── scripts/          # Build & dev scripts
├── public/           # Static assets
└── install.sh/ps1    # One-liner installers
```

## License

MIT — use it, modify it, ship it.
