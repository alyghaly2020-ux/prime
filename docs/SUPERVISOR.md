# Supervisor Heartbeat System

Internal observer that monitors AI agent execution in real time, detecting and correcting issues automatically.

## Architecture

```
Agent ──(mpsc Sender)──▶ Supervisor ──(watchdog timer)──▶ Intervention
  ▲                                                        │
  └──────────────────── (shutdown signal) ─────────────────┘
```

## Detection Capabilities

| Detector | What it catches | Default threshold | Severity |
|----------|----------------|-------------------|----------|
| **Loop** | Repeated identical output/thought ≥ N times | 3 repetitions | 7/10 |
| **Stall** | Same step with no progress > timeout | 30s | 6/10 |
| **Hallucination** | Output drifts from objective keywords (≥3 objective words ≥4 chars with 0 matches) | 3 key-words | 8/10 |
| **Token Explosion** | Token count doubling each step for 3+ steps OR exceeding hard limit | 32K tokens | 9/10 |
| **Timeout** | No heartbeat within watchdog window OR total run time exceeded | 15s watchdog / 5min max | 5-10/10 |

## Interventions

| Intervention | Description |
|-------------|-------------|
| `ResetContext` | Clears last N context turns, injects correction |
| `CorrectionPrompt` | Injects `[SYSTEM CORRECTION]` message re-focusing agent |
| `SwitchModel` | Falls back to configured alternative model |
| `Kill` | Force-terminates the agent |
| `WarnOnly` | Logs warning, takes no action |

## Configuration

All configurable via `SupervisorConfig` with builder:

```rust
use prime_core::core::supervisor::SupervisorBuilder;

let supervisor = SupervisorBuilder::new()
    .heartbeat_interval(1000)     // ms between agent ticks
    .watchdog_timeout(10_000)     // ms before watchdog triggers
    .loop_threshold(5)            // N identical outputs = loop
    .stall_timeout(60_000)        // ms before stall declared
    .max_tokens(64_000)           // hard token limit
    .max_run_time(600_000)        // 10 minutes max
    .auto_intervene(true)         // auto-fix or just log
    .fallback_model("gpt-4o")     // model to switch to on loop
    .build();
```

## Tauri Commands

| Command | Returns | Description |
|---------|---------|-------------|
| `supervisor_start` | `()` | Creates channel + spawns supervisor run loop |
| `supervisor_stats` | `SupervisorStats` | tracked_agents, active_issues, resolved_issues, total_interventions, is_running |
| `supervisor_stop` | `()` | Shuts down supervisor gracefully |

## Usage from Rust

```rust
use prime_core::core::supervisor::{Supervisor, Heartbeat, AgentState};

let supervisor = Arc::new(Supervisor::default());
let (tx, rx) = Supervisor::channel();

// Spawn supervisor
let sup = supervisor.clone();
tokio::spawn(async move { sup.run(rx).await; });

// Agent sends heartbeats
tx.send(Heartbeat::new("agent-1")
    .with_state(AgentState::Thinking)
    .with_thought("Solving problem...")
    .with_output("Intermediate result")
    .with_step(3)
    .with_elapsed(5000)
    .with_tokens(1500)
    .with_model("gpt-4")
    .with_objective("Write fibonacci in Rust"))
.await;
```

## File Location

- Core logic: `prime_core/src/core/supervisor.rs` (1026 lines, 12 unit tests)
- Tauri bridge: `src-tauri/src/lib.rs` (SupervisorState + 3 commands)
- Tests: `cargo test -p prime_core -- core::supervisor`
