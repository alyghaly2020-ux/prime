use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct MultiAgentWorkflow {
    agents: RwLock<Vec<AgentDef>>,
    running: RwLock<HashMap<String, Vec<String>>>,
    workflows: RwLock<Vec<AgentWorkflowDef>>,
}

// =============================================================================
// Workflow Definitions
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkflowStep {
    pub id: String,
    pub action: String,
    pub agent_id: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentWorkflowStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkflowDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<AgentWorkflowStep>,
    pub status: AgentWorkflowStatus,
}

#[derive(Debug, Clone)]
pub struct AgentDef {
    pub id: String,
    pub name: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub system_prompt: String,
    pub tool_ids: Vec<String>,
    pub model_preference: Vec<String>,
}

#[async_trait]
pub trait AgentExecutable {
    async fn execute(
        &self,
        task: &str,
        router: &crate::ai::Router,
        tool_registry: &crate::tools::registry::ToolRegistry,
    ) -> anyhow::Result<String>;
}

#[async_trait]
impl AgentExecutable for AgentDef {
    async fn execute(
        &self,
        task: &str,
        router: &crate::ai::Router,
        tool_registry: &crate::tools::registry::ToolRegistry,
    ) -> anyhow::Result<String> {
        let mut tools_info = String::new();
        if !self.tool_ids.is_empty() {
            tools_info.push_str("\n\nYou have access to the following tools:\n");
            for tool_id in &self.tool_ids {
                if let Some(tool) = tool_registry.get(tool_id).await {
                    tools_info.push_str(&format!("- {} ({}): {}\n", tool.name, tool.id, tool.description));
                }
            }
        }

        let system_prompt = format!("{}{}", self.system_prompt, tools_info);

        let messages = vec![
            crate::ai::ChatMessage {
                role: "system".into(),
                content: system_prompt,
                tool_calls: None,
                timestamp: None,
            },
            crate::ai::ChatMessage {
                role: "user".into(),
                content: task.into(),
                tool_calls: None,
                timestamp: None,
            },
        ];

        let model_id = self.model_preference.first()
            .map(|s| s.as_str())
            .unwrap_or("default");

        let response = router.chat(messages, model_id).await
            .map_err(|e| anyhow::anyhow!("Agent execution failed: {}", e))?;

        Ok(response)
    }
}

impl Default for MultiAgentWorkflow {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiAgentWorkflow {
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(Vec::new()),
            running: RwLock::new(HashMap::new()),
            workflows: RwLock::new(Vec::new()),
        }
    }

    pub async fn register_agent(&self, agent: AgentDef) {
        self.agents.write().await.push(agent);
    }

    pub async fn start_workflow(&self, name: &str, agents: Vec<String>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.running.write().await.insert(id.clone(), agents);
        tracing::info!("Multi-agent workflow '{}' started: {}", name, id);
        id
    }

    pub async fn get_status(&self, workflow_id: &str) -> Option<Vec<String>> {
        self.running.read().await.get(workflow_id).cloned()
    }

    pub async fn available_agents(&self) -> Vec<AgentDef> {
        self.agents.read().await.clone()
    }

    /// Return the list of registered workflow definitions.
    pub async fn list_workflow_defs(&self) -> Vec<AgentWorkflowDef> {
        self.workflows.read().await.clone()
    }

    /// Seed 4 default DAG workflows into the registry.
    /// Call at app startup after agent registration.
    pub async fn seed_default_workflows(&self) {
        let workflows = vec![
            AgentWorkflowDef {
                id: "code-review".into(),
                name: "Code Review".into(),
                description: "Search codebase, review for issues, and suggest fixes".into(),
                status: AgentWorkflowStatus::Idle,
                steps: vec![
                    AgentWorkflowStep {
                        id: "search".into(),
                        action: "search_code".into(),
                        agent_id: "repo-intelligence-agent".into(),
                        depends_on: vec![],
                    },
                    AgentWorkflowStep {
                        id: "review".into(),
                        action: "review_code".into(),
                        agent_id: "code-review-agent".into(),
                        depends_on: vec!["search".into()],
                    },
                    AgentWorkflowStep {
                        id: "fix".into(),
                        action: "suggest_fixes".into(),
                        agent_id: "refactoring-agent".into(),
                        depends_on: vec!["review".into()],
                    },
                ],
            },
            AgentWorkflowDef {
                id: "research".into(),
                name: "Deep Research".into(),
                description: "Web search across multiple sources, summarize results, store in memory".into(),
                status: AgentWorkflowStatus::Idle,
                steps: vec![
                    AgentWorkflowStep {
                        id: "web-search".into(),
                        action: "search_web".into(),
                        agent_id: "research-agent".into(),
                        depends_on: vec![],
                    },
                    AgentWorkflowStep {
                        id: "summarize".into(),
                        action: "summarize_results".into(),
                        agent_id: "prompt-engineering-agent".into(),
                        depends_on: vec!["web-search".into()],
                    },
                    AgentWorkflowStep {
                        id: "store".into(),
                        action: "store_in_memory".into(),
                        agent_id: "memory-keeper".into(),
                        depends_on: vec!["summarize".into()],
                    },
                ],
            },
            AgentWorkflowDef {
                id: "chat".into(),
                name: "AI Chat".into(),
                description: "Simple AI chat with automatic model routing and context management".into(),
                status: AgentWorkflowStatus::Idle,
                steps: vec![
                    AgentWorkflowStep {
                        id: "route".into(),
                        action: "route_model".into(),
                        agent_id: "llm-routing-agent".into(),
                        depends_on: vec![],
                    },
                    AgentWorkflowStep {
                        id: "respond".into(),
                        action: "generate_response".into(),
                        agent_id: "coding-agent".into(),
                        depends_on: vec!["route".into()],
                    },
                ],
            },
            AgentWorkflowDef {
                id: "agent-orchestrate".into(),
                name: "Agent Orchestration".into(),
                description: "Plan a task, delegate to specialist agents, aggregate results, and verify output".into(),
                status: AgentWorkflowStatus::Idle,
                steps: vec![
                    AgentWorkflowStep {
                        id: "plan".into(),
                        action: "decompose_task".into(),
                        agent_id: "task-planner-agent".into(),
                        depends_on: vec![],
                    },
                    AgentWorkflowStep {
                        id: "delegate".into(),
                        action: "delegate_to_specialists".into(),
                        agent_id: "agent-orchestration-agent".into(),
                        depends_on: vec!["plan".into()],
                    },
                    AgentWorkflowStep {
                        id: "aggregate".into(),
                        action: "aggregate_results".into(),
                        agent_id: "agent-orchestration-agent".into(),
                        depends_on: vec!["delegate".into()],
                    },
                    AgentWorkflowStep {
                        id: "verify".into(),
                        action: "verify_output".into(),
                        agent_id: "reviewer".into(),
                        depends_on: vec!["aggregate".into()],
                    },
                ],
            },
        ];

        let mut reg = self.workflows.write().await;
        *reg = workflows;
        tracing::info!("Seeded {} default workflows", reg.len());
    }

    /// Seed all 90+ agent definitions into the registry.
    /// Call once at app startup.
    pub async fn seed_all(&self) {
        let agents = Self::all_agent_defs();
        let mut reg = self.agents.write().await;
        *reg = agents;
        tracing::info!("Seeded {} agents into registry", reg.len());
        // Also seed the 4 default DAG workflows
        self.seed_default_workflows().await;
    }

    fn all_agent_defs() -> Vec<AgentDef> {
        vec![
            // =====================================================================
            // Core Architecture & System Design
            // =====================================================================
            AgentDef {
                id: "system-architect-agent".into(),
                name: "System Architect".into(),
                role: "High-level distributed systems design".into(),
                system_prompt: "You design scalable distributed systems. Evaluate technology choices, create architecture documentation, and plan for scalability, reliability, and maintainability.".into(),
                capabilities: vec!["system-design".into(), "architecture".into(), "scalability".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "infrastructure-architect-agent".into(),
                name: "Infrastructure Architect".into(),
                role: "Cloud architecture, Kubernetes, Terraform, networking".into(),
                system_prompt: "You design cloud infrastructure, plan migrations, optimize costs, and improve reliability and disaster recovery.".into(),
                capabilities: vec!["infrastructure".into(), "devops".into(), "cloud".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Development
            // =====================================================================
            AgentDef {
                id: "coding-agent".into(),
                name: "Coding Agent".into(),
                role: "General-purpose implementation across all languages".into(),
                system_prompt: "You implement features, fix bugs, and generate code following DRY, SOLID, and clean code principles.".into(),
                capabilities: vec!["code".into(), "implementation".into(), "development".into()],
            tool_ids: vec!["aider".into(), "openhands".into()],
            model_preference: vec![],
            },
            AgentDef {
                id: "debugging-agent".into(),
                name: "Debugging Wizard".into(),
                role: "Root cause analysis via stack traces and logs".into(),
                system_prompt: "You systematically analyze errors, stack traces, and log output to isolate root causes through hypothesis-driven methodology.".into(),
                capabilities: vec!["debugging".into(), "code-review".into(), "error-analysis".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "refactoring-agent".into(),
                name: "Refactoring Specialist".into(),
                role: "Code smell removal, design patterns, complexity reduction".into(),
                system_prompt: "You identify code smells, apply design patterns, reduce complexity, and eliminate duplication while preserving behavior.".into(),
                capabilities: vec!["refactoring".into(), "code-quality".into(), "optimization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "performance-optimization-agent".into(),
                name: "Performance Optimizer".into(),
                role: "Profiling, bottleneck identification, caching".into(),
                system_prompt: "You profile, identify hot paths, and apply caching, algorithmic optimization, and I/O optimization.".into(),
                capabilities: vec!["performance".into(), "profiling".into(), "optimization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Language Specialists
            // =====================================================================
            AgentDef {
                id: "rust-expert-agent".into(),
                name: "Rust Expert".into(),
                role: "Safe, idiomatic Rust with async/tokio".into(),
                system_prompt: "You write safe, idiomatic Rust with async/tokio, lifetimes, unsafe code, macros, FFI, and performance optimization.".into(),
                capabilities: vec!["rust".into(), "systems-programming".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "tauri-expert-agent".into(),
                name: "Tauri Expert".into(),
                role: "Tauri v2 desktop and mobile".into(),
                system_prompt: "You build Tauri v2 applications with IPC, commands, capabilities, plugins, window management, and auto-updater.".into(),
                capabilities: vec!["tauri".into(), "desktop-app".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "wasm-engineer-agent".into(),
                name: "WASM Engineer".into(),
                role: "WebAssembly browser and server modules".into(),
                system_prompt: "You build high-performance WASM modules with wasm-pack, wasm-bindgen, wasmtime, and WASI.".into(),
                capabilities: vec!["wasm".into(), "webassembly".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "react-ui-agent".into(),
                name: "React UI Expert".into(),
                role: "Modern React 19 with Tailwind and shadcn/ui".into(),
                system_prompt: "You build React 19 UIs with shadcn/ui, Tailwind CSS, accessibility, responsive design, and performance optimization.".into(),
                capabilities: vec!["react".into(), "typescript".into(), "frontend".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "async-systems-agent".into(),
                name: "Async Systems Expert".into(),
                role: "Event loops, structured concurrency, backpressure".into(),
                system_prompt: "You design async architectures with event loops, structured concurrency, backpressure, and cancellation.".into(),
                capabilities: vec!["async".into(), "concurrency".into(), "event-loops".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "tokio-expert-agent".into(),
                name: "Tokio Expert".into(),
                role: "Tokio runtime, async, channels, cancellation".into(),
                system_prompt: "You optimize Tokio runtimes, use channels, CancellationToken, and design async Rust applications.".into(),
                capabilities: vec!["async".into(), "tokio".into(), "concurrency".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // AI/ML
            // =====================================================================
            AgentDef {
                id: "llm-routing-agent".into(),
                name: "LLM Router".into(),
                role: "Model selection, cost optimization, fallback chains".into(),
                system_prompt: "You select models, optimize costs, design fallback chains, and A/B test inference providers.".into(),
                capabilities: vec!["ai-routing".into(), "model-selection".into(), "cost-optimization".into()],
            tool_ids: vec!["omniroute".into(), "litellm".into()],
            model_preference: vec![],
            },
            AgentDef {
                id: "prompt-engineering-agent".into(),
                name: "Prompt Engineer".into(),
                role: "System prompts, few-shot, CoT, structured output".into(),
                system_prompt: "You craft system prompts, design few-shot examples, chain-of-thought, and structured output schemas.".into(),
                capabilities: vec!["prompt-engineering".into(), "prompt-optimization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "context-engineering-agent".into(),
                name: "Context Engineer".into(),
                role: "Sliding windows, summarization, token budgets".into(),
                system_prompt: "You manage LLM context windows, prune history, summarize, and optimize token budgets.".into(),
                capabilities: vec!["context-management".into(), "token-optimization".into(), "summarization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "rag-engineer-agent".into(),
                name: "RAG Engineer".into(),
                role: "Document chunking, embeddings, hybrid search".into(),
                system_prompt: "You build RAG pipelines: chunk documents, generate embeddings, configure vector stores, and optimize retrieval.".into(),
                capabilities: vec!["rag".into(), "embeddings".into(), "vector-search".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "local-llm-agent".into(),
                name: "Local LLM Expert".into(),
                role: "Ollama, llama.cpp, vLLM, quantization".into(),
                system_prompt: "You deploy local LLMs with ollama, llama.cpp, vLLM; select quantization; optimize hardware.".into(),
                capabilities: vec!["local-llm".into(), "inference".into(), "quantization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },

            AgentDef {
                id: "model-fine-tuning-agent".into(),
                name: "Fine-tuning Expert".into(),
                role: "LoRA/QLoRA, dataset prep, PEFT".into(),
                system_prompt: "You fine-tune LLMs with LoRA/QLoRA, prepare datasets, and deploy adapters.".into(),
                capabilities: vec!["fine-tuning".into(), "model-training".into(), "lora".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "inference-optimization-agent".into(),
                name: "Inference Optimizer".into(),
                role: "vLLM, TensorRT, ONNX, KV-cache".into(),
                system_prompt: "You optimize LLM inference with vLLM, TensorRT, ONNX, quantization, and speculative decoding.".into(),
                capabilities: vec!["inference".into(), "quantization".into(), "model-serving".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "gpu-optimization-agent".into(),
                name: "GPU Optimizer".into(),
                role: "CUDA, ROCm, kernel fusion, mixed precision".into(),
                system_prompt: "You optimize GPU utilization with kernel fusion, memory coalescing, and mixed precision.".into(),
                capabilities: vec!["gpu".into(), "cuda".into(), "compute-optimization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "cpu-optimization-agent".into(),
                name: "CPU Optimizer".into(),
                role: "SIMD, cache, NUMA, PGO".into(),
                system_prompt: "You optimize CPU execution with SIMD, cache optimization, and profile-guided optimization.".into(),
                capabilities: vec!["cpu".into(), "simd".into(), "optimization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "resource-manager-agent".into(),
                name: "Resource Manager".into(),
                role: "Memory limits, circuit breakers, rate limiting".into(),
                system_prompt: "You manage system resources with memory limits, OOM prevention, circuit breakers, and rate limiting.".into(),
                capabilities: vec!["resource-management".into(), "rate-limiting".into(), "circuit-breaker".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Search & Intelligence
            // =====================================================================
            AgentDef {
                id: "repo-intelligence-agent".into(),
                name: "Repo Intelligence".into(),
                role: "Code graph, dependency analysis, impact analysis".into(),
                system_prompt: "You analyze repository structure, map dependencies, understand ownership, and measure codebase health.".into(),
                capabilities: vec!["code-search".into(), "code-understanding".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "search-engine-agent".into(),
                name: "Search Engine Architect".into(),
                role: "Full-text search, inverted indices, BM25".into(),
                system_prompt: "You implement full-text search with inverted indices, BM25, typo tolerance, and faceted search.".into(),
                capabilities: vec!["search".into(), "indexing".into(), "retrieval".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "tantivy-expert-agent".into(),
                name: "Tantivy Expert".into(),
                role: "Tantivy (Rust) search library".into(),
                system_prompt: "You build search with Tantivy: index creation, schema design, tokenizers, and query parsing.".into(),
                capabilities: vec!["search".into(), "tantivy".into(), "indexing".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "treesitter-expert-agent".into(),
                name: "Tree-sitter Expert".into(),
                role: "AST parsing, syntax queries, code analysis".into(),
                system_prompt: "You build language tooling with tree-sitter: AST queries, code highlighting, and symbol extraction.".into(),
                capabilities: vec!["parsing".into(), "ast".into(), "code-analysis".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "semantic-search-agent".into(),
                name: "Semantic Search Expert".into(),
                role: "Embedding-based dense/sparse/hybrid search".into(),
                system_prompt: "You build semantic search with embedding-based retrieval, hybrid search, and relevance feedback.".into(),
                capabilities: vec!["semantic-search".into(), "embeddings".into(), "retrieval".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "vector-search-agent".into(),
                name: "Vector Search Expert".into(),
                role: "HNSW, IVF, PQ, DiskANN".into(),
                system_prompt: "You optimize vector search with HNSW, IVF, PQ indexes and tune for performance.".into(),
                capabilities: vec!["vector-search".into(), "similarity".into(), "embeddings".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "knowledge-graph-agent".into(),
                name: "Knowledge Graph Expert".into(),
                role: "RDF, SPARQL, Neo4j, GraphRAG".into(),
                system_prompt: "You build knowledge graphs with RDF, SPARQL, Neo4j, and graph-based RAG pipelines.".into(),
                capabilities: vec!["knowledge-graph".into(), "relationships".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Security & System
            // =====================================================================
            AgentDef {
                id: "security-engineer-agent".into(),
                name: "Security Engineer".into(),
                role: "OWASP, SAST/DAST, vulnerability assessment".into(),
                system_prompt: "You assess OWASP Top 10, perform SAST/DAST, audit dependencies, and produce structured security reports.".into(),
                capabilities: vec!["security".into(), "vulnerability-assessment".into()],
            tool_ids: vec!["transilience-ai".into(), "threatswarm".into()],
            model_preference: vec![],
            },
            AgentDef {
                id: "sandbox-engineer-agent".into(),
                name: "Sandbox Engineer".into(),
                role: "Secure execution environments, seccomp, WASM".into(),
                system_prompt: "You design sandboxed execution with filesystem isolation, network policies, and resource limits.".into(),
                capabilities: vec!["sandbox".into(), "isolation".into(), "containment".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "capability-permission-agent".into(),
                name: "Permission Expert".into(),
                role: "Capability-based security, RBAC/ABAC".into(),
                system_prompt: "You design authorization systems with least-privilege, RBAC/ABAC, and permission auditing.".into(),
                capabilities: vec!["authorization".into(), "rbac".into(), "permissions".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "privacy-engineer-agent".into(),
                name: "Privacy Engineer".into(),
                role: "PII detection, GDPR/CCPA, anonymization".into(),
                system_prompt: "You implement data minimization, PII detection, consent flows, and GDPR/CCPA compliance.".into(),
                capabilities: vec!["privacy".into(), "data-protection".into(), "compliance".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "encryption-agent".into(),
                name: "Encryption Expert".into(),
                role: "AES, ECC, TLS, zero-knowledge".into(),
                system_prompt: "You design encryption schemes, key management, and secure communication protocols.".into(),
                capabilities: vec!["encryption".into(), "cryptography".into(), "key-management".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "enterprise-security-agent".into(),
                name: "Enterprise Security".into(),
                role: "SSO, PKI, zero trust, compliance".into(),
                system_prompt: "You build SSO/SAML/OIDC, PKI infrastructure, zero trust networks, and meet SOC 2/ISO 27001.".into(),
                capabilities: vec!["enterprise-security".into(), "sso".into(), "zero-trust".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "audit-logging-agent".into(),
                name: "Audit Logger".into(),
                role: "Structured logging, tamper-evident stores".into(),
                system_prompt: "You design audit systems with immutable log stores, tamper-evident logging, and forensics-ready trails.".into(),
                capabilities: vec!["audit".into(), "logging".into(), "forensics".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "os-integration-agent".into(),
                name: "OS Integration Expert".into(),
                role: "FS, processes, IPC, syscalls".into(),
                system_prompt: "You design file system operations, process management, IPC, and OS-level abstractions.".into(),
                capabilities: vec!["os".into(), "system-calls".into(), "platform".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "windows-native-agent".into(),
                name: "Windows Native Expert".into(),
                role: "Win32, COM, PowerShell, Windows Services".into(),
                system_prompt: "You build Windows-native apps with Win32 APIs, COM, PowerShell cmdlets, and Windows Services.".into(),
                capabilities: vec!["windows".into(), "win32".into(), "native".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "linux-native-agent".into(),
                name: "Linux Native Expert".into(),
                role: "systemd, D-Bus, io_uring, cgroups".into(),
                system_prompt: "You build Linux daemons with systemd, D-Bus, cgroups, and io_uring.".into(),
                capabilities: vec!["linux".into(), "systemd".into(), "native".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "macos-native-agent".into(),
                name: "macOS Native Expert".into(),
                role: "Mach APIs, GCD, Keychain, code signing".into(),
                system_prompt: "You build macOS apps with Mach APIs, Grand Central Dispatch, Keychain, and code signing.".into(),
                capabilities: vec!["macos".into(), "native".into(), "apple".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "cross-platform-agent".into(),
                name: "Cross-Platform Expert".into(),
                role: "CMake, PAL, CI matrix, cross-compilation".into(),
                system_prompt: "You build software targeting multiple OSes with CMake, platform abstraction, and CI matrix builds.".into(),
                capabilities: vec!["cross-platform".into(), "portability".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Database
            // =====================================================================
            AgentDef {
                id: "sqlite-expert-agent".into(),
                name: "SQLite Expert".into(),
                role: "WAL, FTS5, performance tuning".into(),
                system_prompt: "You optimize SQLite with WAL mode, FTS5, R-Tree, and concurrent access patterns.".into(),
                capabilities: vec!["database".into(), "sql".into(), "query-optimization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Automation & DevOps
            // =====================================================================
            AgentDef {
                id: "playwright-automation-agent".into(),
                name: "Playwright Automation".into(),
                role: "Browser automation, E2E tests, anti-detection".into(),
                system_prompt: "You write browser tests with Playwright: page objects, anti-detection, visual testing, and parallel execution.".into(),
                capabilities: vec!["browser-automation".into(), "playwright".into(), "e2e".into()],
            tool_ids: vec!["invisible-playwright".into(), "cloakbrowser".into()],
            model_preference: vec![],
            },
            AgentDef {
                id: "terminal-automation-agent".into(),
                name: "Terminal Automation".into(),
                role: "PTY, ANSI, cross-platform scripting".into(),
                system_prompt: "You automate terminal operations with PTY/spawn, ANSI parsing, and interactive command handling.".into(),
                capabilities: vec!["terminal".into(), "shell".into(), "automation".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "git-operations-agent".into(),
                name: "Git Operations".into(),
                role: "Branch management, merge conflicts, bisect".into(),
                system_prompt: "You manage branches, resolve merge conflicts, bisect bugs, and maintain repository health.".into(),
                capabilities: vec!["git".into(), "version-control".into(), "cicd".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "devops-agent".into(),
                name: "DevOps Engineer".into(),
                role: "Docker, CI/CD, Kubernetes, monitoring".into(),
                system_prompt: "You design infrastructure, configure CI/CD, manage Kubernetes, and plan production operations.".into(),
                capabilities: vec!["devops".into(), "ci-cd".into(), "deployment".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "cicd-agent".into(),
                name: "CI/CD Expert".into(),
                role: "GitHub Actions, GitLab CI, build matrix".into(),
                system_prompt: "You design CI/CD pipelines, optimize build caching, and manage artifact promotion.".into(),
                capabilities: vec!["ci-cd".into(), "pipelines".into(), "automation".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "testing-agent".into(),
                name: "Testing Expert".into(),
                role: "Unit, integration, property-based, fuzzing".into(),
                system_prompt: "You write tests: unit, integration, property-based, fuzzing, and design test architectures.".into(),
                capabilities: vec!["testing".into(), "quality-assurance".into(), "unit-test".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "qa-agent".into(),
                name: "QA Engineer".into(),
                role: "Test plans, exploratory testing, bug reports".into(),
                system_prompt: "You plan QA activities, write test cases, manage bug reports, and assess quality.".into(),
                capabilities: vec!["qa".into(), "test-planning".into(), "quality".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Observability & Resilience
            // =====================================================================
            AgentDef {
                id: "observability-agent".into(),
                name: "Observability Expert".into(),
                role: "Metrics, logging, tracing, OpenTelemetry".into(),
                system_prompt: "You implement metrics pipelines, structured logging, distributed tracing, and OpenTelemetry.".into(),
                capabilities: vec!["observability".into(), "monitoring".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },

            AgentDef {
                id: "crash-recovery-agent".into(),
                name: "Crash Recovery".into(),
                role: "Panic handling, graceful shutdown, checkpoint".into(),
                system_prompt: "You design crash-resilient systems with panic handling, graceful shutdown, and checkpoint/restore.".into(),
                capabilities: vec!["crash-recovery".into(), "fault-tolerance".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "self-healing-agent".into(),
                name: "Self-Healing Expert".into(),
                role: "Health checks, circuit breakers, failover".into(),
                system_prompt: "You design autonomous recovery with health checks, circuit breakers, and failover.".into(),
                capabilities: vec!["self-healing".into(), "auto-recovery".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "verification-agent".into(),
                name: "Verification Expert".into(),
                role: "Formal verification, property-based testing".into(),
                system_prompt: "You verify correctness with formal methods, property-based testing, and invariants.".into(),
                capabilities: vec!["verification".into(), "validation".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Orchestration & Workflow
            // =====================================================================
            AgentDef {
                id: "workflow-engine-agent".into(),
                name: "Workflow Engine".into(),
                role: "DAG execution, saga patterns, compensation".into(),
                system_prompt: "You design workflow engines with DAG execution, saga patterns, and compensation transactions.".into(),
                capabilities: vec!["workflow".into(), "orchestration".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "agent-orchestration-agent".into(),
                name: "Agent Orchestrator".into(),
                role: "Routing, load balancing, agent discovery".into(),
                system_prompt: "You orchestrate multi-agent systems with routing, load balancing, and inter-agent communication.".into(),
                capabilities: vec!["agent-orchestration".into(), "multi-agent".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "event-bus-engineer-agent".into(),
                name: "Event Bus Engineer".into(),
                role: "Pub/sub, Kafka, event sourcing".into(),
                system_prompt: "You design pub/sub systems with Kafka, event sourcing, CQRS, and reliable messaging.".into(),
                capabilities: vec!["event-bus".into(), "events".into(), "messaging".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "actor-systems-engineer-agent".into(),
                name: "Actor Systems Engineer".into(),
                role: "Akka, Orleans, Actix, ProtoActor".into(),
                system_prompt: "You build actor-based systems with supervision, location transparency, and cluster sharding.".into(),
                capabilities: vec!["actor-model".into(), "concurrency".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Plugin & Extension Systems
            // =====================================================================
            AgentDef {
                id: "plugin-system-agent".into(),
                name: "Plugin System Architect".into(),
                role: "Plugin lifecycle, sandboxing, dependency resolution".into(),
                system_prompt: "You design extensible applications with plugin APIs, lifecycle management, and sandboxed execution.".into(),
                capabilities: vec!["plugin-system".into(), "extensibility".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },

            AgentDef {
                id: "mcp-engineer-agent".into(),
                name: "MCP Engineer".into(),
                role: "MCP protocol, servers, tool/resource providers".into(),
                system_prompt: "You design MCP servers with tool handlers, resource providers, and transport layers.".into(),
                capabilities: vec!["mcp".into(), "protocol".into(), "integration".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },

            AgentDef {
                id: "skill-builder-agent".into(),
                name: "Skill Builder".into(),
                role: "opencode skill creation".into(),
                system_prompt: "You design opencode skills with SKILL.md, rule files, workflows, and progressive disclosure.".into(),
                capabilities: vec!["skills".into(), "wasm".into(), "plugins".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "workflow-builder-agent".into(),
                name: "Workflow Builder".into(),
                role: "Multi-agent workflow design".into(),
                system_prompt: "You design multi-agent workflows, delegation chains, and quality gates.".into(),
                capabilities: vec!["workflow-builder".into(), "automation".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Packaging & Distribution
            // =====================================================================
            AgentDef {
                id: "packaging-agent".into(),
                name: "Packaging Expert".into(),
                role: "Cross-platform packaging, code signing, SBOM".into(),
                system_prompt: "You build packaging pipelines for .exe, .dmg, .deb, .rpm, with code signing and SBOM.".into(),
                capabilities: vec!["packaging".into(), "distribution".into(), "cross-platform".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "installer-builder-agent".into(),
                name: "Installer Builder".into(),
                role: "NSIS, WiX, Electron Builder".into(),
                system_prompt: "You create installers with NSIS, WiX, Electron Builder with silent install and upgrades.".into(),
                capabilities: vec!["installer".into(), "distribution".into(), "packaging".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "auto-update-agent".into(),
                name: "Auto-Update Expert".into(),
                role: "Differential updates, rollback safety".into(),
                system_prompt: "You implement auto-update systems with differential updates, rollback safety, and staged rollouts.".into(),
                capabilities: vec!["auto-update".into(), "distribution".into(), "versioning".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "desktop-runtime-agent".into(),
                name: "Desktop Runtime Expert".into(),
                role: "Tauri, Electron, NW.js runtime".into(),
                system_prompt: "You optimize desktop runtimes with process models, IPC, native APIs, and bundle optimization.".into(),
                capabilities: vec!["desktop".into(), "runtime".into(), "tauri".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Design & Documentation
            // =====================================================================
            AgentDef {
                id: "documentation-writer-agent".into(),
                name: "Documentation Writer".into(),
                role: "API docs, README, migration guides".into(),
                system_prompt: "You write API docs, architecture docs, migration guides, and user documentation.".into(),
                capabilities: vec!["documentation".into(), "technical-writing".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "api-designer-agent".into(),
                name: "API Designer".into(),
                role: "REST, GraphQL, gRPC API design".into(),
                system_prompt: "You design REST/GraphQL/gRPC APIs with resource modeling, versioning, and pagination.".into(),
                capabilities: vec!["api-design".into(), "rest".into(), "graphql".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "product-designer-agent".into(),
                name: "Product Designer".into(),
                role: "User stories, PRDs, A/B testing".into(),
                system_prompt: "You define product requirements, user stories, acceptance criteria, and design features.".into(),
                capabilities: vec!["product-design".into(), "requirements".into(), "specifications".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "ux-wizard-flow-agent".into(),
                name: "UX Flow Designer".into(),
                role: "User journeys, multi-step flows, onboarding".into(),
                system_prompt: "You design user journeys, multi-step flows, onboarding, and wizard interfaces.".into(),
                capabilities: vec!["ux".into(), "user-experience".into(), "design".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "ui-animation-agent".into(),
                name: "UI Animation Expert".into(),
                role: "Framer Motion, GSAP, 60fps animations".into(),
                system_prompt: "You add animations with Framer Motion, GSAP, micro-interactions, and accessible prefers-reduced-motion.".into(),
                capabilities: vec!["animation".into(), "ui-effects".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "ai-operating-system-agent".into(),
                name: "AI OS Architect".into(),
                role: "AI-native OS architecture, agent scheduling".into(),
                system_prompt: "You design AI-native operating systems with agent scheduling and resource orchestration.".into(),
                capabilities: vec!["ai-os".into(), "agent-scheduling".into(), "ai-architecture".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Style & Research
            // =====================================================================
            AgentDef {
                id: "claude-code-style-agent".into(),
                name: "Claude Code Style".into(),
                role: "Careful planning, thorough testing style".into(),
                system_prompt: "You emulate deliberate, thorough code generation: plan first, test thoroughly, explain clearly.".into(),
                capabilities: vec!["code-style".into(), "planning".into(), "thorough".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "codex-style-agent".into(),
                name: "Codex Style".into(),
                role: "Manager-worker efficient style".into(),
                system_prompt: "You use manager-worker architecture, sandboxed execution, plan-first with minimal explanation.".into(),
                capabilities: vec!["code-style".into(), "efficient".into(), "worker".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "research-agent".into(),
                name: "Deep Research Agent".into(),
                role: "Multi-language web research".into(),
                system_prompt: "You perform deep multi-language web research across 5+ languages and cross-validate sources.".into(),
                capabilities: vec!["deep-research".into(), "analysis".into()],
            tool_ids: vec!["searxng".into(), "crawl4ai".into(), "jina-reader".into()],
            model_preference: vec![],
            },
            AgentDef {
                id: "benchmarking-agent".into(),
                name: "Benchmarking Expert".into(),
                role: "Performance benchmarks, regression detection".into(),
                system_prompt: "You measure performance, detect regressions, and automate benchmark reporting.".into(),
                capabilities: vec!["benchmarking".into(), "evaluation".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "ai-evaluation-agent".into(),
                name: "AI Evaluation Expert".into(),
                role: "LLM eval, bias testing, safety".into(),
                system_prompt: "You evaluate LLMs with benchmarks, bias testing, safety evaluation, and adversarial testing.".into(),
                capabilities: vec!["ai-evaluation".into(), "testing".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            // =====================================================================
            // Built-in Infrastructure
            // =====================================================================
            AgentDef {
                id: "explore".into(),
                name: "Explore".into(),
                role: "Fast read-only codebase exploration".into(),
                system_prompt: "You quickly search and read codebase files to understand structure.".into(),
                capabilities: vec!["exploration".into(), "code-search".into(), "reading".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "general".into(),
                name: "General Agent".into(),
                role: "Multi-step research and execution".into(),
                system_prompt: "You perform multi-step research and execution tasks with fan-out parallel work.".into(),
                capabilities: vec!["general".into(), "multi-purpose".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "architect".into(),
                name: "Architect".into(),
                role: "Plan architecture first, clean structure".into(),
                system_prompt: "You plan architecture first, eliminate duplication, and enforce clean structure.".into(),
                capabilities: vec!["architecture".into(), "planning".into(), "design".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "reviewer".into(),
                name: "Reviewer".into(),
                role: "Strict verification, correctness, security".into(),
                system_prompt: "You review all work for correctness, edge cases, security, and consistency.".into(),
                capabilities: vec!["review".into(), "code-review".into(), "verification".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "code-review-agent".into(),
                name: "Code Review Agent".into(),
                role: "Search codebase, review code for issues, and produce structured review reports".into(),
                system_prompt: "You are a code review specialist. You search the codebase, analyze code for bugs, anti-patterns, security vulnerabilities, and maintainability issues, then produce structured review reports with actionable feedback.".into(),
                capabilities: vec!["code-review".into(), "analysis".into(), "verification".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "memory-keeper".into(),
                name: "Memory Keeper".into(),
                role: "Persistent context across sessions".into(),
                system_prompt: "You store and retrieve persistent context across project sessions.".into(),
                capabilities: vec!["memory".into(), "persistence".into(), "context".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "memory-compression-agent".into(),
                name: "Memory Compression".into(),
                role: "Context pruning, summarization".into(),
                system_prompt: "You compress LLM context with summarization and hierarchical memory management.".into(),
                capabilities: vec!["memory-compression".into(), "context-management".into(), "summarization".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "tool-calling-agent".into(),
                name: "Tool Calling Expert".into(),
                role: "Tool definitions, chaining, error handling".into(),
                system_prompt: "You design tool-calling systems with schemas, error handling, and parallel execution.".into(),
                capabilities: vec!["tool-calling".into(), "tools".into(), "execution".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "execution-supervisor-agent".into(),
                name: "Execution Supervisor".into(),
                role: "Monitor agents, detect loops/timeouts".into(),
                system_prompt: "You supervise agent execution, detect loops and timeouts, and escalate issues.".into(),
                capabilities: vec!["supervision".into(), "monitoring".into(), "error-detection".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
            AgentDef {
                id: "task-planner-agent".into(),
                name: "Task Planner".into(),
                role: "Decompose complex tasks, estimate effort".into(),
                system_prompt: "You break complex tasks into actionable steps and identify dependencies.".into(),
                capabilities: vec!["task-planning".into(), "decomposition".into(), "estimation".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },

            AgentDef {
                id: "system-guardian-agent".into(),
                name: "System Guardian".into(),
                role: "Monitor CPU temp, RAM, disk, load. Auto-throttle when resources are critical.".into(),
                system_prompt: "You monitor system resources (CPU, RAM, disk, temperature) and detect threats. When the system is overloaded, you throttle agents, reduce concurrency, and protect the machine from OOM or thermal damage. You're always watching.".into(),
                capabilities: vec!["system-monitor".into(), "resource-throttle".into(), "auto-preservation".into()],
            tool_ids: vec![],
            model_preference: vec![],
            },
        ]
    }
}

#[tauri::command]
pub async fn execute_agent(
    agent_id: String,
    task: String,
    dev: tauri::State<'_, std::sync::Arc<crate::dev::Engine>>,
    router: tauri::State<'_, std::sync::Arc<crate::ai::Router>>,
    tool_registry: tauri::State<'_, std::sync::Arc<crate::tools::registry::ToolRegistry>>,
) -> Result<String, crate::AppError> {
    let agents = dev.agents.available_agents().await;
    let agent = agents.iter().find(|a| a.id == agent_id)
        .ok_or_else(|| crate::AppError::Workspace(format!("Agent not found: {}", agent_id)))?;

    let result = agent.execute(&task, &router, &tool_registry).await
        .map_err(|e| crate::AppError::Workspace(e.to_string()))?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_seed_all_agents() {
        let workflow = MultiAgentWorkflow::new();
        workflow.seed_all().await;
        let agents = workflow.available_agents().await;
        assert_eq!(agents.len(), 87, "Expected exactly 87 agents, got {}", agents.len());

        // Check first agent
        assert_eq!(agents[0].id, "system-architect-agent");

        // Verify no duplicate IDs
        let mut ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), agents.len(), "Duplicate agent IDs found");
    }

    #[tokio::test]
    async fn test_register_and_list() {
        let workflow = MultiAgentWorkflow::new();
        let agent = AgentDef {
            id: "test-agent".into(),
            name: "Test".into(),
            role: "Testing".into(),
            capabilities: vec!["general".into()],
            system_prompt: "You test things.".into(),
            tool_ids: vec![],
            model_preference: vec![],
        };
        workflow.register_agent(agent).await;
        let agents = workflow.available_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "test-agent");
    }
}
