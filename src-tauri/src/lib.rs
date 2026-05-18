//! # Prime — AI-Powered Desktop Agent Platform
//!
//! ## Architecture
//!
//! The backend is organized into 14 subsystems, each in its own module:
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`ai`] | AI model routing, chat completions, provider abstraction |
//! | [`arch`] | Event bus, workflow engine, scheduler, actor model |
//! | [`browser`] | Playwright automation, OCR, vision, DOM extraction |
//! | [`code_intel`] | Multi-language parsing (tree-sitter), symbols, search |
//! | [`contracts`] | Interface traits (MCP, memory, runtime, skills) |
//! | [`core`] | Runtime singleton, SQLite storage, WASM sandbox, gRPC |
//! | [`dev`] | Agent registry (93 agents), workspace indexing, retrieval |
//! | [`system`] | System resource monitor, thermal guardian, auto-throttle |
//! | [`execution`] | Process supervisor, PTY terminal, rollback, checkpoint |
//! | [`mcp`] | 10 MCP servers, permission middleware, rate limiter |
//! | [`memory`] | 7-tier memory (working, episodic, semantic, vector, RAG) |
//! | [`observability`] | Metrics, tracing, telemetry, crash reporting |
//! | [`security`] | Encryption, sandbox, permissions, audit, rate-limiter |
//! | [`skills`] | WASM plugin runtime, signing, hot-reload, loader |
//! | [`verification`] | Linter, reviewer, error analyzer, test runner, self-heal |
//!
//! All modules communicate via the [`contracts`] trait layer to prevent
//! circular dependencies and enforce clean architecture boundaries.
//!
//! ## IPC Commands
//!
//! 30+ Tauri commands are registered via `generate_handler!` at startup.
//! See the individual `#[tauri::command]` functions for details.
//! All commands return `Result<T, AppError>`.
//!
//! ## Entry Point
//!
//! [`PrimeApp::run()`] (in `main.rs`) initializes tracing, the Tokio runtime,
//! builds the Tauri app with all plugins and commands, seeds the agent registry,
//! registers 10 MCP servers, and starts the server manager with health checks.

pub mod ai;
pub mod arch;
pub mod browser;
pub mod cli;
pub mod code_intel;
pub mod computer_use;
pub use prime_core::contracts;
pub use prime_core::core;
pub mod dev;
pub mod execution;
pub mod ide;
pub mod mcp;
pub mod memory;
pub mod observability;
pub mod phi_brain;
pub mod proxy;
pub use prime_core::security;
pub mod server;
pub mod skills;
pub mod system;
pub mod tools;
pub mod verification;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, Emitter};
use thiserror::Error;
use tokio::sync::Mutex;

/// Global credential pool shared across all providers.
/// Auto-populated at startup from env vars and config.
static CREDENTIAL_POOL: Lazy<ai::credential_pool::CredentialPool> =
    Lazy::new(ai::credential_pool::CredentialPool::new);

// =============================================================================
// Tauri Commands — registered below in generate_handler!
// =============================================================================

#[tauri::command]
async fn get_system_state(
    runtime: tauri::State<'_, Arc<core::Runtime>>,
) -> Result<core::RuntimeState, AppError> {
    runtime
        .state()
        .await
        .map_err(|e| AppError::Execution(e.to_string()))
}

#[tauri::command]
async fn execute_code(
    execution: tauri::State<'_, Arc<execution::Engine>>,
    code: String,
    language: String,
) -> Result<execution::ExecutionResult, AppError> {
    execution
        .execute(&code, &language)
        .await
        .map_err(|e| AppError::Execution(e.to_string()))
}

#[tauri::command]
async fn search_code(
    intel: tauri::State<'_, Arc<code_intel::Engine>>,
    query: String,
    path: String,
) -> Result<Vec<code_intel::SearchResult>, AppError> {
    // Canonicalize and validate path to prevent path traversal
    let resolved_path = std::path::Path::new(&path);
    let canonical = std::fs::canonicalize(resolved_path)
        .map_err(|e| AppError::Search(format!("Invalid path '{}': {}", path, e)))?;

    // Ensure the path is within the current working directory
    let cwd = std::env::current_dir()
        .map_err(|e| AppError::Search(format!("Cannot determine working directory: {}", e)))?;
    let cwd_canonical = std::fs::canonicalize(&cwd)
        .map_err(|e| AppError::Search(format!("Cannot canonicalize working directory: {}", e)))?;

    if !canonical.starts_with(&cwd_canonical) {
        return Err(AppError::Search(format!(
            "Path '{}' resolves to '{}' which is outside the allowed workspace '{}'",
            path,
            canonical.display(),
            cwd_canonical.display()
        )));
    }

    intel
        .search(&query, &canonical.to_string_lossy())
        .await
        .map_err(|e| AppError::Search(e.to_string()))
}

#[tauri::command]
async fn query_memory(
    memory: tauri::State<'_, Arc<memory::System>>,
    query: String,
    memory_type: String,
) -> Result<String, AppError> {
    memory
        .query(&query, &memory_type)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

#[tauri::command]
async fn invoke_skill(
    skills: tauri::State<'_, Arc<skills::System>>,
    skill_id: String,
    input: String,
) -> Result<String, AppError> {
    skills
        .invoke(&skill_id, &input)
        .await
        .map_err(|e| AppError::Execution(e.to_string()))
}

#[tauri::command]
async fn ai_chat(
    app: tauri::AppHandle,
    router: tauri::State<'_, Arc<ai::Router>>,
    messages: Vec<ai::ChatMessage>,
    model: String,
) -> Result<String, AppError> {
    let mut updated_messages = messages.clone();
    
    // Identify activities to emit based on prompt content
    let input_lower = messages.last().map(|m| m.content.to_lowercase()).unwrap_or_default();
    let is_code = input_lower.contains("كود") || input_lower.contains("برمج") || input_lower.contains("واجه") || input_lower.contains("تصميم") || input_lower.contains("code") || input_lower.contains("ide") || input_lower.contains("ui") || input_lower.contains("style");
    let is_browser = input_lower.contains("متصفح") || input_lower.contains("browser") || input_lower.contains("افتح") || input_lower.contains("open") || input_lower.contains("موقع") || input_lower.contains("بحث") || input_lower.contains("search");
    let is_payment = input_lower.contains("دفع") || input_lower.contains("محفظ") || input_lower.contains("شراء") || input_lower.contains("pay") || input_lower.contains("wallet") || input_lower.contains("checkout") || input_lower.contains("معامل");

    if is_code {
        let _ = app.emit("agent-activity", serde_json::json!({ "tab": "code", "active": true }));
    }
    if is_browser {
        let _ = app.emit("agent-activity", serde_json::json!({ "tab": "browser", "active": true }));
    }
    if is_payment {
        let _ = app.emit("agent-activity", serde_json::json!({ "tab": "payments", "active": true }));
    }
    let _ = app.emit("agent-activity", serde_json::json!({ "tab": "chat", "active": true }));

    // 1. Construct and Inject the core Prime Cognitive System Prompt & 7-Layer Memory
    let verified_list = if let Ok(ref user_cfg) = crate::load_config_inner() {
        user_cfg.enabled_providers.clone()
    } else {
        Vec::new()
    };

    let active_connections = if let Ok(ref user_cfg) = crate::load_config_inner() {
        user_cfg.enabled_connections.clone()
    } else {
        Vec::new()
    };

    let current_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/home/ghaly/prime".to_string());

    let platform_system_prompt = format!(
        "أنت لست مجرد نموذج لغوي؛ أنت العقل المدبر ومساعد الذكاء الاصطناعي المركزي لمنصة المكاتب الذكية الشاملة « Prime AI ».\n\
        أنت تعمل كـ Desktop Application على جهاز المستخدم (Linux) وتتحكم وتتفاعل مع البيئة بالكامل.\n\n\
        =========================================================\n\
        🎯 الهوية الإمبراطورية والقدرات المدمجة لـ Prime:\n\
        =========================================================\n\
        🔌 الحالة النشطة والتكامل البيئي للمنصة:\n\
        =========================================================\n\
        📁 المشروع النشط المفتوح حالياً: {}\n\
        🧠 النماذج النشطة والمعتمدة حالياً (Verified Models): {:?}\n\
        🔌 قنوات الاتصال النشطة حالياً (Communication Channels): {:?}\n\n\
        =========================================================\n\
        🧠 هيكلية الذاكرة المعرفية ذات الـ 7 طبقات + Heartbeat:\n\
        =========================================================\n\
        لمنع فقدان التركيز أو الهلوسة البرمجية والحفاظ على السياق مهما كان معقداً أو طويلاً، التزم بالطبقات المعرفية السبع وطبقة نبضات القلب:\n\n\
        💓 [طبقة نبض القلب - Heartbeat Layer]:\n\
        قم دائماً بتقييم الغرض النهائي للمستخدم مع كل إجابة. قبل كتابة أي سطر، تحقق: 'هل هذا يحل مشكلة المستخدم الحقيقية بأقصر وأفضل طريقة؟' واحتفظ بحلقة تنفيذ واعية لخطواتك الحالية.\n\n\
        1. 🧠 [الطبقة 1: Sensory Buffer (مخزن الإحساس الفوري)]:\n\
        استوعب الحالة الفورية للنوافذ والملفات النشطة أمام المستخدم. متصفحك المدمج مفتوح الآن، وتاريخ محادثاتك مستقر.\n\n\
        2. 🖥️ [الطبقة 2: Working RAM (الذاكرة العاملة المؤقتة)]:\n\
        احتفظ بسياق الأسئلة والطلبات الأخيرة المباشرة التي يديرها العميل حالياً ولا تفقد تسلسلها.\n\n\
        3. 💾 [الطبقة 3: Short-Term Memory (ذاكرة التنفيذ القريبة)]:\n\
        راقب سجل استدعاء الأدوات البرمجية (MCP Tools)، والتنقل التلقائي للمتصفح، والتعديل الفوري للمملفات، وأي عمليات تمت في الجلسة.\n\n\
        4. 🗂️ [الطبقة 4: Semantic Memory (الذاكرة المعجمية التخصصية)]:\n\
        قواعد لغة البرمجة، والوعي بهيكل الـ MCP، والقدرة على استدعاء الأدوات للتحكم بالملفات والمتصفح.\n\n\
        5. 🗃️ [الطبقة 5: Episodic Memory (ذاكرة الأحداث والسياقات السابقة)]:\n\
        تذكر القرارات التي اتخذتها مسبقاً في المحادثة بخصوص الإعدادات وتثبيت النماذج لتجنب التكرار.\n\n\
        6. 🌐 [الطبقة 6: Long-Term Storage (الذاكرة الدائمة الموحدة)]:\n\
        معلومات تهيئة وحالة المنصة الإجمالية، والمحافظ النشطة، وإحصائيات التكلفة، ومفاتيح الـ API المخزنة في النظام.\n\n\
        7. 📡 [الطبقة 7: Vector/Recall Memory (ذاكرة الاسترجاع الترابطي)]:\n\
        سياق المحادثات السابقة المخزنة بالكامل في قاعدة البيانات، والتي تسترجعها بمجرد عودة العميل للتطبيق.\n\n\
        =========================================================\n\
        التزم بالرد بلغة المستخدم وباحترافية شديدة تؤكد وعيك الكامل بهويتك كـ Prime ومحيطك التقني بالكامل!",
        current_dir,
        verified_list,
        active_connections
    );

    let has_system = updated_messages.iter().any(|m| m.role == "system");
    if !has_system {
        let system_prompt = ai::ChatMessage {
            role: "system".to_string(),
            content: platform_system_prompt,
            tool_calls: None,
            timestamp: None,
        };
        updated_messages.insert(0, system_prompt);
    }

    // 2. Intent Detection for Browser Operations
    let mut browser_executed = false;
    let mut executed_url = String::new();
    
    if let Some(last_msg) = messages.last() {
        if last_msg.role == "user" {
            let input_lower = last_msg.content.to_lowercase();
            let mut target_url = None;
            
            if input_lower.contains("متصفح") || input_lower.contains("browser") || input_lower.contains("افتح") || input_lower.contains("open") {
                if input_lower.contains("جوجل") || input_lower.contains("google") {
                    target_url = Some("https://www.google.com".to_string());
                } else if input_lower.contains("يوتيوب") || input_lower.contains("youtube") {
                    target_url = Some("https://www.youtube.com".to_string());
                } else if input_lower.contains("فيسبوك") || input_lower.contains("facebook") {
                    target_url = Some("https://www.facebook.com".to_string());
                } else if input_lower.contains("تويتر") || input_lower.contains("twitter") || input_lower.contains(" x ") {
                    target_url = Some("https://x.com".to_string());
                } else if input_lower.contains("ياهو") || input_lower.contains("yahoo") {
                    target_url = Some("https://www.yahoo.com".to_string());
                } else if input_lower.contains("ويكيبيديا") || input_lower.contains("wikipedia") {
                    target_url = Some("https://www.wikipedia.org".to_string());
                } else if input_lower.contains("github") {
                    target_url = Some("https://github.com".to_string());
                } else if input_lower.contains("chatgpt") || input_lower.contains("openai") {
                    target_url = Some("https://chat.openai.com".to_string());
                }
                
                if target_url.is_none() && (input_lower.contains("شغل") || input_lower.contains("افتح") || input_lower.contains("open") || input_lower.contains("start")) {
                    target_url = Some("https://www.google.com".to_string());
                }
            }
            
            if let Some(url) = target_url {
                if let Some(browser) = app.try_state::<Arc<browser::System>>() {
                    // Make sure we connect first if not connected
                    if !browser.playwright.is_connected().await {
                        let _ = browser.playwright.connect("ws://localhost?headed").await;
                    }
                    if browser.navigate(&url).await.is_ok() {
                        browser_executed = true;
                        executed_url = url;
                        
                        // Emit global events to React to switch mode and refresh snapshot immediately!
                        let _ = app.emit("change-view-mode", "browser");
                        let _ = app.emit("browser-updated", ());
                    }
                }
            }
        }
    }

    // 3. Inject Action Execution Notification for the LLM
    if browser_executed {
        let system_info = ai::ChatMessage {
            role: "system".to_string(),
            content: format!("[System Notification: You have successfully started the built-in browser and navigated to {} live on the user's desktop screen. Acknowledge this action in your reply, guide the user to click the 'Browser' tab in the top navigation bar to see it, and offer further help.]", executed_url),
            tool_calls: None,
            timestamp: None,
        };
        updated_messages.push(system_info);
    }

    // 4. Phi Brain: Smart routing if model is "auto" or "smart"
    let final_model = if model == "auto" || model == "smart" {
        if let Some(phi) = app.try_state::<Arc<phi_brain::PhiBrain>>() {
            if phi.is_available().await {
                let profile = phi.profile.read().await.clone();
                let available_models: Vec<String> = router.list_models()
                    .into_iter()
                    .map(|m| m.id)
                    .collect();
                let health = if let Some(sm) = app.try_state::<Arc<system::SystemMonitor>>() {
                    let metrics = sm.latest_metrics().await;
                    phi_brain::orchestrator::SystemHealth {
                        cpu_percent: metrics.cpu.usage_percent as f32,
                        ram_percent: metrics.memory.used_percent as f32,
                        temp_celsius: metrics.temperature.celsius.unwrap_or(0.0) as f32,
                    }
                } else {
                    phi_brain::orchestrator::SystemHealth::default()
                };
                let decision = phi.orchestrator.decide(
                    &updated_messages,
                    &health,
                    &profile,
                    &available_models,
                ).await;
                tracing::info!("Phi Brain routing decision: model={} mode={}", decision.model, decision.mode);
                decision.model
            } else {
                model.clone()
            }
        } else {
            model.clone()
        }
    } else {
        model.clone()
    };

    let result = router.chat(updated_messages, &final_model).await;

    // 5. Phi Brain: Proofread the response before sending to user
    let final_response = if let Ok(ref response) = result {
        if let Some(phi) = app.try_state::<Arc<phi_brain::PhiBrain>>() {
            if phi.is_available().await && phi.proofreader.is_enabled() {
                let proof = phi.proofreader.review(response, &messages).await;
                if proof.was_modified {
                    tracing::info!(
                        "Phi Brain proofread: {} corrections applied",
                        proof.corrections.len()
                    );
                    let _ = app.emit("phi-correction", serde_json::json!({
                        "corrections": proof.corrections,
                        "hallucination_score": proof.hallucination_score,
                    }));
                    proof.corrected_text
                } else {
                    response.clone()
                }
            } else {
                response.clone()
            }
        } else {
            response.clone()
        }
    } else {
        result.map_err(|e| AppError::AiEngine(ai::redact_sensitive(&e.to_string())))?
    };

    // 6. Phi Brain: Learn from this interaction (background, non-blocking)
    if let Some(phi) = app.try_state::<Arc<phi_brain::PhiBrain>>() {
        let phi_clone = Arc::clone(&phi);
        let user_msg = messages.iter()
            .filter(|m| m.role == "user")
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let task_type = phi_brain::profile_db::UserProfile::classify_task(&user_msg).to_string();
        let response_for_learner = final_response.clone();
        tokio::spawn(async move {
            phi_clone.learner.learn_from_interaction(
                &phi_brain::learner::Interaction {
                    user_message: user_msg,
                    assistant_response: response_for_learner,
                    model_used: final_model.clone(),
                    task_type,
                    response_time_ms: 0,
                    user_feedback: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                }
            ).await;
        });
    }

    // Reset activities to inactive
    let _ = app.emit("agent-activity", serde_json::json!({ "tab": "code", "active": false }));
    let _ = app.emit("agent-activity", serde_json::json!({ "tab": "browser", "active": false }));
    let _ = app.emit("agent-activity", serde_json::json!({ "tab": "payments", "active": false }));
    let _ = app.emit("agent-activity", serde_json::json!({ "tab": "chat", "active": false }));

    Ok(final_response)
}

#[tauri::command]
async fn list_mcp_servers(
    mcp: tauri::State<'_, Arc<mcp::ServerManager>>,
) -> Result<Vec<mcp::ServerInfo>, AppError> {
    Ok(mcp.list_servers().await)
}

#[tauri::command]
async fn list_models(
    router: tauri::State<'_, Arc<ai::Router>>,
) -> Result<String, AppError> {
    let configs = router.list_models();
    let models: Vec<serde_json::Value> = configs.into_iter().map(|cfg| {
        serde_json::json!({
            "id": cfg.id,
            "provider": cfg.provider,
            "model": cfg.model,
            "status": "online",
            "max_tokens": cfg.max_tokens,
            "temperature": cfg.temperature,
            "streaming": cfg.streaming,
        })
    }).collect();
    serde_json::to_string(&models).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn greet(name: String) -> Result<String, AppError> {
    Ok(format!("Hello !! Im Prime, {}! 👋", name))
}

// =============================================================================
// Plugin Commands
// =============================================================================

#[tauri::command]
async fn list_plugins(
    skills: tauri::State<'_, Arc<skills::System>>,
) -> Result<String, AppError> {
    tracing::info!("list_plugins called");
    let entries = skills.registry.list().await;
    let plugins: Vec<serde_json::Value> = entries.into_iter().map(|e| {
        serde_json::json!({
            "id": e.id,
            "name": e.name,
            "version": e.version,
            "description": e.description,
            "author": e.author,
            "status": if e.enabled { "active" } else { "inactive" },
            "enabled": e.enabled,
            "permissions": [],
            "entry": "",
            "type": "skill"
        })
    }).collect();
    let result = serde_json::to_string(&plugins).map_err(|e| AppError::Workspace(e.to_string()));
    tracing::info!("list_plugins returning {} plugins", plugins.len());
    result
}

#[tauri::command]
async fn plugin_enable(
    skills: tauri::State<'_, Arc<skills::System>>,
    id: String,
) -> Result<(), AppError> {
    skills.registry.set_enabled(&id, true).await.map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn plugin_disable(
    skills: tauri::State<'_, Arc<skills::System>>,
    id: String,
) -> Result<(), AppError> {
    skills.registry.set_enabled(&id, false).await.map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn plugin_install(
    skills: tauri::State<'_, Arc<skills::System>>,
    path: String,
) -> Result<(), AppError> {
    skills.load_skill(&path).await.map_err(|e| AppError::Workspace(e.to_string()))?;
    Ok(())
}

#[tauri::command]
async fn plugin_uninstall(
    skills: tauri::State<'_, Arc<skills::System>>,
    id: String,
) -> Result<(), AppError> {
    skills.registry.unregister(&id).await.map_err(|e| AppError::Workspace(e.to_string()))
}

// =============================================================================
// Workflow Commands
// =============================================================================

#[tauri::command]
async fn list_workflows(
    event_bus: tauri::State<'_, Arc<arch::EventBus>>,
) -> Result<String, AppError> {
    tracing::info!("list_workflows called");
    let workflows = event_bus.workflows.list_workflows().await;
    let result = serde_json::to_string(&workflows).map_err(|e| AppError::Workspace(e.to_string()));
    tracing::info!("list_workflows done");
    result
}

#[tauri::command]
async fn workflow_start(
    event_bus: tauri::State<'_, Arc<arch::EventBus>>,
    id: String,
) -> Result<(), AppError> {
    event_bus.workflows.start_workflow(&id).await.map_err(AppError::Workspace)
}

#[tauri::command]
async fn workflow_cancel(
    event_bus: tauri::State<'_, Arc<arch::EventBus>>,
    id: String,
) -> Result<(), AppError> {
    event_bus.workflows.cancel_workflow(&id).await.map_err(AppError::Workspace)
}

#[tauri::command]
async fn workflow_pause(
    event_bus: tauri::State<'_, Arc<arch::EventBus>>,
    id: String,
) -> Result<(), AppError> {
    event_bus.workflows.pause_workflow(&id).await.map_err(AppError::Workspace)
}

#[tauri::command]
async fn workflow_resume(
    event_bus: tauri::State<'_, Arc<arch::EventBus>>,
    id: String,
) -> Result<(), AppError> {
    event_bus.workflows.resume_workflow(&id).await.map_err(AppError::Workspace)
}

// =============================================================================
// MCP Management Commands
// =============================================================================

#[tauri::command]
async fn mcp_start_server(
    mcp: tauri::State<'_, Arc<mcp::ServerManager>>,
    id: String,
) -> Result<(), AppError> {
    mcp.start_server(&id).await.map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn mcp_stop_server(
    mcp: tauri::State<'_, Arc<mcp::ServerManager>>,
    id: String,
) -> Result<(), AppError> {
    mcp.stop_server(&id).await.map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn mcp_restart_server(
    mcp: tauri::State<'_, Arc<mcp::ServerManager>>,
    id: String,
) -> Result<(), AppError> {
    mcp.stop_server(&id).await.map_err(|e| AppError::Workspace(e.to_string()))?;
    mcp.start_server(&id).await.map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn mcp_toggle_server(
    mcp: tauri::State<'_, Arc<mcp::ServerManager>>,
    id: String,
    enabled: bool,
) -> Result<(), AppError> {
    if enabled {
        mcp.start_server(&id).await.map_err(|e| AppError::Workspace(e.to_string()))
    } else {
        mcp.stop_server(&id).await.map_err(|e| AppError::Workspace(e.to_string()))
    }
}

#[tauri::command]
async fn mcp_add_config(
    mcp: tauri::State<'_, Arc<mcp::ServerManager>>,
    id: String,
    config: String,
) -> Result<(), AppError> {
    mcp.add_config(id.clone(), config).await;
    tracing::info!("Stored MCP config for server: {}", id);
    Ok(())
}

#[tauri::command]
async fn mcp_remove_config(
    mcp: tauri::State<'_, Arc<mcp::ServerManager>>,
    id: String,
) -> Result<(), AppError> {
    if mcp.remove_config(&id).await.is_some() {
        tracing::info!("Removed MCP config for server: {}", id);
    } else {
        tracing::warn!("No config found for server: {}", id);
    }
    Ok(())
}

// =============================================================================
// Memory Management Commands
// =============================================================================

#[tauri::command]
async fn get_memory_stats(
    memory: tauri::State<'_, Arc<memory::System>>,
) -> Result<String, AppError> {
    let stats = memory.get_stats().await.map_err(AppError::Database)?;
    serde_json::to_string(&stats).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn delete_memory_entry(
    memory: tauri::State<'_, Arc<memory::System>>,
    id: String,
) -> Result<(), AppError> {
    memory.delete_entry(&id).await.map_err(AppError::Database)
}

#[tauri::command]
async fn clear_memory(
    memory: tauri::State<'_, Arc<memory::System>>,
    memory_type: String,
) -> Result<(), AppError> {
    memory.clear_type(&memory_type).await.map_err(AppError::Database)
}

// =============================================================================
// Agent Commands
// =============================================================================

#[tauri::command]
async fn list_agents(
    dev: tauri::State<'_, Arc<dev::Engine>>,
    ai: tauri::State<'_, Arc<ai::Router>>,
) -> Result<String, AppError> {
    let agents = dev.agents.available_agents().await;
    let result: Vec<serde_json::Value> = agents.into_iter().map(|a| {
        let model = ai.resolve_agent_model(&a.capabilities, &[]);
        serde_json::json!({
            "id": a.id,
            "name": a.name,
            "role": a.role,
            "model": model,
            "capabilities": a.capabilities,
        })
    }).collect();
    serde_json::to_string(&result).map_err(|e| AppError::Workspace(e.to_string()))
}

// =============================================================================
// Settings Commands
// =============================================================================

#[tauri::command]
async fn get_settings(
    runtime: tauri::State<'_, Arc<core::Runtime>>,
) -> Result<String, AppError> {
    let storage = runtime.storage.clone();
    let settings = tokio::task::spawn_blocking(move || {
        storage
            .load_settings()
            .map_err(|e| AppError::Workspace(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Workspace(e.to_string()))??;
    serde_json::to_string(&settings).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn save_settings(
    runtime: tauri::State<'_, Arc<core::Runtime>>,
    settings: String,
) -> Result<(), AppError> {
    let parsed: core::storage::AppSettings = serde_json::from_str(&settings)
        .map_err(|e| AppError::Workspace(format!("Invalid settings JSON: {}", e)))?;
    let storage = runtime.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .save_settings(&parsed)
            .map_err(|e| AppError::Workspace(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Workspace(e.to_string()))??;
    Ok(())
}

// =============================================================================
// User Config Commands — persisted as JSON in app data directory
// =============================================================================

/// All providers the user can configure
const ALL_PROVIDERS: &[&str] = &[
    "openai", "anthropic", "deepseek", "google", "ollama", "openrouter",
    "groq", "together", "mistral", "cohere", "perplexity", "azure",
    "aws", "replicate", "huggingface", "fireworks", "anyscale", "lmstudio",
    "localai", "groqcloud", "xai", "meta", "zhipu", "baidu", "alibaba",
    "tencent", "custom_openai", "sambanova", "writer", "ai21",
];

/// All connections the user can configure
const ALL_CONNECTIONS: &[&str] = &[
    "telegram", "telegram_bot", "whatsapp", "discord", "slack",
    "email", "wechat", "signal", "matrix", "irc",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub api_keys: HashMap<String, String>,
    pub connection_configs: HashMap<String, ConnectionConfig>,
    pub enabled_providers: Vec<String>,
    pub enabled_connections: Vec<String>,
    pub verified_providers: Vec<String>,
    pub system_settings: SystemSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemSettings {
    #[serde(default = "default_true")]
    pub sandbox: bool,
    #[serde(default = "default_true")]
    pub network: bool,
    #[serde(default)]
    pub filesystem: bool,
    #[serde(default = "default_true")]
    pub permission_prompts: bool,
    #[serde(default = "default_true")]
    pub audit_logging: bool,
    #[serde(default)]
    pub headless_enable: bool,
    #[serde(default = "default_ethereum")]
    pub payment_default_chain: String,
    #[serde(default = "default_true")]
    pub payment_audit: bool,
}

fn default_true() -> bool { true }
fn default_ethereum() -> String { "Ethereum".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionConfig {
    pub enabled: bool,
    pub label: String,
    #[serde(default)]
    pub fields: HashMap<String, String>,
}


impl Default for UserConfig {
    fn default() -> Self {
        let all_providers: Vec<String> = ALL_PROVIDERS.iter().map(|s| s.to_string()).collect();
        let all_connections: Vec<String> = ALL_CONNECTIONS.iter().map(|s| s.to_string()).collect();
        Self {
            api_keys: HashMap::new(),
            connection_configs: HashMap::new(),
            enabled_providers: all_providers,
            enabled_connections: all_connections,
            verified_providers: Vec::new(),
            system_settings: SystemSettings::default(),
        }
    }
}

fn config_path() -> Result<std::path::PathBuf, AppError> {
    let data_dir = dirs_next::data_dir()
        .ok_or_else(|| AppError::Workspace("Cannot determine data directory".into()))?;
    let prime_dir = data_dir.join("prime");
    std::fs::create_dir_all(&prime_dir).map_err(|e| AppError::Workspace(e.to_string()))?;
    Ok(prime_dir.join("config.json"))
}

pub(crate) fn load_config_inner() -> Result<UserConfig, AppError> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = UserConfig::default();
        let json = serde_json::to_string_pretty(&cfg)
            .map_err(|e| AppError::Workspace(e.to_string()))?;
        std::fs::write(&path, &json).map_err(|e| AppError::Workspace(e.to_string()))?;
        return Ok(cfg);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| AppError::Workspace(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| AppError::Workspace(e.to_string()))
}

/// Maps provider IDs to env var names (lowercase id → UPPERCASE_ENV_KEY)
fn provider_id_to_env_key(id: &str) -> String {
    match id {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "google" => "GOOGLE_API_KEY",
        "groq" => "GROQ_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "xai" => "XAI_API_KEY",
        "fireworks" => "FIREWORKS_API_KEY",
        "together" => "TOGETHER_API_KEY",
        "perplexity" => "PERPLEXITY_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "cohere" => "COHERE_API_KEY",
        "azure" => "AZURE_OPENAI_API_KEY",
        "aws" => "AWS_ACCESS_KEY_ID",
        "replicate" => "REPLICATE_API_TOKEN",
        "huggingface" => "HUGGINGFACE_API_KEY",
        "moonshot" => "MOONSHOT_API_KEY",
        "qwen" => "QWEN_API_KEY",
        "minimax" => "MINIMAX_API_KEY",
        "ollama" => "",
        "vllm" => "VLLM_API_KEY",
        "byteplus" => "BYTEPLUS_API_KEY",
        "venice" => "VENICE_API_KEY",
        "nous" => "NOUS_API_KEY",
        "glm" => "GLM_API_KEY",
        _ => "",
    }
    .to_string()
}

/// Tries credential pool first, then env var, then config JSON file.
/// The credential pool provides multi-key failover per provider.
pub(crate) fn get_api_key(env_key: &str, provider_id: &str) -> Result<String, String> {
    // 1. Try global credential pool (multi-key, failover)
    if let Some(key) = CREDENTIAL_POOL.get(provider_id) {
        return Ok(key.expose().to_string());
    }

    // 2. Try environment variable
    if let Ok(key) = std::env::var(env_key) {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // 3. Try config file
    if let Ok(config) = load_config_inner() {
        // Try by provider ID (lowercase)
        if let Some(key) = config.api_keys.get(provider_id) {
            if !key.is_empty() {
                return Ok(key.clone());
            }
        }
        // Try by env key name (backward compat)
        let provider_id_from_env = provider_id_to_env_key(provider_id);
        if !provider_id_from_env.is_empty() {
            if let Some(key) = config.api_keys.get(&provider_id_from_env) {
                if !key.is_empty() {
                    return Ok(key.clone());
                }
            }
        }
    }

    Err(format!(
        "{} not set. Set the {} environment variable or add the key in Settings → Model Providers",
        env_key, env_key
    ))
}

/// Report an API call failure for a provider's credential.
/// The pool automatically deactivates keys after 3 failures, triggering
/// failover to the next available key.
pub(crate) fn report_api_failure(provider_id: &str, key: &str) {
    CREDENTIAL_POOL.report_failure(provider_id, key);
}

/// Report an API call success for a provider's credential.
/// Resets the failure count, keeping the key active.
pub(crate) fn report_api_success(provider_id: &str, key: &str) {
    CREDENTIAL_POOL.report_success(provider_id, key);
}

#[tauri::command]
async fn get_config() -> Result<String, AppError> {
    let config = tokio::task::spawn_blocking(load_config_inner)
        .await
        .map_err(|e| AppError::Workspace(e.to_string()))??;
    serde_json::to_string(&config).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn save_api_key(provider: String, key: String) -> Result<(), AppError> {
    let provider_clone = provider.clone();
    let key_clone = key.clone();
    tokio::task::spawn_blocking(move || {
        let mut config = load_config_inner()?;
        if key.is_empty() {
            config.api_keys.remove(&provider);
        } else {
            config.api_keys.insert(provider, key);
        }
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| AppError::Workspace(e.to_string()))?;
        std::fs::write(config_path()?, &json).map_err(|e| AppError::Workspace(e.to_string()))?;
        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::Workspace(e.to_string()))??;

    // Register with global credential pool so it's immediately available
    if !key_clone.is_empty() {
        let env_key = provider_id_to_env_key(&provider_clone);
        let env_val = std::env::var(&env_key).ok();
        CREDENTIAL_POOL.register(&provider_clone, ai::SecretKey::new(key_clone), Some(0));
        if let Some(env_k) = env_val {
            CREDENTIAL_POOL.register(&provider_clone, ai::SecretKey::new(env_k), Some(1));
        }
    }

    Ok(())
}

#[tauri::command]
async fn save_connection_config(id: String, config_json: String) -> Result<(), AppError> {
    let conn_cfg: ConnectionConfig = serde_json::from_str(&config_json)
        .map_err(|e| AppError::Workspace(format!("Invalid connection config: {}", e)))?;
    tokio::task::spawn_blocking(move || {
        let mut config = load_config_inner()?;
        config.connection_configs.insert(id, conn_cfg);
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| AppError::Workspace(e.to_string()))?;
        std::fs::write(config_path()?, &json).map_err(|e| AppError::Workspace(e.to_string()))?;
        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::Workspace(e.to_string()))??;
    Ok(())
}

#[tauri::command]
async fn save_verified_providers(providers: Vec<String>) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        let mut config = load_config_inner()?;
        config.verified_providers = providers;
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| AppError::Workspace(e.to_string()))?;
        std::fs::write(config_path()?, &json).map_err(|e| AppError::Workspace(e.to_string()))?;
        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::Workspace(e.to_string()))??;
    Ok(())
}

#[tauri::command]
async fn save_system_settings(settings_json: String) -> Result<(), AppError> {
    let settings: SystemSettings = serde_json::from_str(&settings_json)
        .map_err(|e| AppError::Workspace(format!("Invalid system settings: {}", e)))?;
    tokio::task::spawn_blocking(move || {
        let mut config = load_config_inner()?;
        config.system_settings = settings;
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| AppError::Workspace(e.to_string()))?;
        std::fs::write(config_path()?, &json).map_err(|e| AppError::Workspace(e.to_string()))?;
        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::Workspace(e.to_string()))??;
    Ok(())
}

// =============================================================================
// Model Management Commands
// =============================================================================

#[tauri::command]
async fn model_test_connection(
    router: tauri::State<'_, Arc<ai::Router>>,
    id: String,
) -> Result<String, AppError> {
    router.test_connection(&id).await
        .map_err(AppError::AiEngine)
}

#[tauri::command]
async fn model_add(
    router: tauri::State<'_, Arc<ai::Router>>,
    config: String,
) -> Result<(), AppError> {
    let config: ai::ModelConfig = serde_json::from_str(&config)
        .map_err(|e| AppError::AiEngine(format!("Invalid model config: {}", e)))?;
    router.add_model(config).await
        .map_err(AppError::AiEngine)
}

#[tauri::command]
async fn model_remove(
    router: tauri::State<'_, Arc<ai::Router>>,
    id: String,
) -> Result<(), AppError> {
    router.remove_model(&id).await
        .map_err(AppError::AiEngine)
}

#[tauri::command]
async fn list_provider_registry(
    router: tauri::State<'_, Arc<ai::Router>>,
) -> Result<String, AppError> {
    let ids = router.registry.list_ids();
    serde_json::to_string(&ids).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn list_provider_details(
    router: tauri::State<'_, Arc<ai::Router>>,
) -> Result<String, AppError> {
    let providers: Vec<serde_json::Value> = router.registry.list_all().iter().map(|p| {
        serde_json::json!({
            "id": p.id,
            "name": p.name,
            "auth_type": p.auth_type,
            "base_url": p.inference_base_url,
            "api_mode": p.api_mode,
            "env_vars": p.api_key_env_vars,
        })
    }).collect();
    serde_json::to_string(&providers).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn get_credential_status(
    router: tauri::State<'_, Arc<ai::Router>>,
) -> Result<String, AppError> {
    let providers: Vec<serde_json::Value> = router.registry.list_ids().iter().map(|id| {
        let active = router.cred_pool.active_count(id);
        let total = router.cred_pool.total_count(id);
        serde_json::json!({
            "provider": id,
            "active_keys": active,
            "total_keys": total,
            "has_keys": active > 0,
        })
    }).collect();
    serde_json::to_string(&providers).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn race_models(
    router: tauri::State<'_, Arc<ai::Router>>,
    messages: Vec<ai::ChatMessage>,
    model_ids: Vec<String>,
) -> Result<String, AppError> {
    let result = router.race_models(messages, &model_ids).await;
    result.map_err(|e| AppError::AiEngine(ai::redact_sensitive(&e.to_string())))
}

#[tauri::command]
async fn parallel_chat(
    router: tauri::State<'_, Arc<ai::Router>>,
    task_type: String,
    messages: Vec<ai::ChatMessage>,
    count: usize,
) -> Result<String, AppError> {
    let result = router.parallel_chat(&task_type, messages, count).await;
    result.map_err(|e| AppError::AiEngine(ai::redact_sensitive(&e.to_string())))
}

#[tauri::command]
async fn broadcast_to_all(
    router: tauri::State<'_, Arc<ai::Router>>,
    messages: Vec<ai::ChatMessage>,
) -> Result<String, AppError> {
    let results = router.broadcast_to_all(messages).await;
    serde_json::to_string(&results.iter().map(|(k, v)| {
        serde_json::json!({
            "model": k,
            "success": v.is_ok(),
            "content": v.as_ref().ok(),
        })
    }).collect::<Vec<_>>())
    .map_err(|e| AppError::Workspace(e.to_string()))
}

// =============================================================================
// Debug Commands
// =============================================================================

#[tauri::command]
async fn get_logs(
    event_bus: tauri::State<'_, Arc<arch::EventBus>>,
) -> Result<String, AppError> {
    event_bus.logger.get_logs_json().map_err(AppError::Workspace)
}

#[tauri::command]
async fn get_events(
    event_bus: tauri::State<'_, Arc<arch::EventBus>>,
) -> Result<String, AppError> {
    let events = event_bus.core.recent_events(100).await;
    serde_json::to_string(&events).map_err(|e| AppError::Workspace(e.to_string()))
}

// =============================================================================
// Proxy Commands
// =============================================================================

#[tauri::command]
async fn proxy_list(
    proxy: tauri::State<'_, Arc<proxy::ProxyRotator>>,
) -> Result<String, AppError> {
    let entries = proxy.list_proxies();
    serde_json::to_string(&entries).map_err(|e| AppError::Workspace(e.to_string()))
}

#[tauri::command]
async fn proxy_add(
    proxy: tauri::State<'_, Arc<proxy::ProxyRotator>>,
    url: String,
    username: Option<String>,
    password: Option<String>,
    region: Option<String>,
) -> Result<(), AppError> {
    proxy.add_proxy(url, username, password, region);
    Ok(())
}

#[tauri::command]
async fn proxy_remove(
    proxy: tauri::State<'_, Arc<proxy::ProxyRotator>>,
    url: String,
) -> Result<bool, AppError> {
    Ok(proxy.remove_proxy(&url))
}

#[tauri::command]
async fn proxy_next(
    proxy: tauri::State<'_, Arc<proxy::ProxyRotator>>,
) -> Result<Option<String>, AppError> {
    Ok(proxy.next())
}

#[tauri::command]
async fn proxy_mark_success(
    proxy: tauri::State<'_, Arc<proxy::ProxyRotator>>,
    url: String,
) -> Result<(), AppError> {
    proxy.mark_success(&url);
    Ok(())
}

#[tauri::command]
async fn proxy_mark_failed(
    proxy: tauri::State<'_, Arc<proxy::ProxyRotator>>,
    url: String,
) -> Result<(), AppError> {
    proxy.mark_failed(&url);
    Ok(())
}

#[tauri::command]
async fn proxy_active_count(
    proxy: tauri::State<'_, Arc<proxy::ProxyRotator>>,
) -> Result<usize, AppError> {
    Ok(proxy.active_count())
}

// =============================================================================
// Browser Commands
// =============================================================================

#[tauri::command]
async fn browser_connect(
    browser: tauri::State<'_, Arc<browser::System>>,
    endpoint: Option<String>,
) -> Result<(), AppError> {
    let ep = endpoint.unwrap_or_else(|| "ws://localhost".to_string());
    browser.playwright.connect(&ep).await
        .map_err(|e| AppError::Execution(e.to_string()))
}

#[tauri::command]
async fn browser_disconnect(
    browser: tauri::State<'_, Arc<browser::System>>,
) -> Result<(), AppError> {
    browser.playwright.disconnect().await;
    Ok(())
}

#[tauri::command]
async fn browser_navigate(
    browser: tauri::State<'_, Arc<browser::System>>,
    url: String,
) -> Result<browser::PageSnapshot, AppError> {
    browser.navigate(&url).await
        .map_err(|e| AppError::Execution(e.to_string()))
}

#[tauri::command]
async fn browser_snapshot(
    browser: tauri::State<'_, Arc<browser::System>>,
) -> Result<browser::PageSnapshot, AppError> {
    browser.snapshot().await
        .map_err(|e| AppError::Execution(e.to_string()))
}

#[tauri::command]
async fn browser_is_connected(
    browser: tauri::State<'_, Arc<browser::System>>,
) -> Result<bool, AppError> {
    Ok(browser.playwright.is_connected().await)
}

#[tauri::command]
async fn browser_get_text(
    browser: tauri::State<'_, Arc<browser::System>>,
) -> Result<String, AppError> {
    browser.playwright.get_text().await
        .map_err(|e| AppError::Execution(e.to_string()))
}

#[tauri::command]
async fn fetch_web_page(
    browser: tauri::State<'_, Arc<browser::System>>,
    url: String,
) -> Result<String, AppError> {
    // 1. Try real browser (playwright) first for full JS execution and stealth
    if !browser.playwright.is_connected().await {
        let _ = browser.playwright.connect("ws://localhost").await;
    }
    if browser.playwright.is_connected().await {
        if let Ok(snapshot) = browser.navigate(&url).await {
            return Ok(snapshot.html);
        }
    }

    // 2. Fallback to robust HTTP reqwest
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Execution(format!("HTTP client build failed: {}", e)))?;

    let res = client.get(&url)
        .send()
        .await
        .map_err(|e| AppError::Execution(format!("Network request failed: {}", e)))?;

    let body = res.text()
        .await
        .map_err(|e| AppError::Execution(format!("Failed to read response body: {}", e)))?;

    Ok(body)
}

// =============================================================================
// Task & Observability Commands
// =============================================================================

#[tauri::command]
async fn task_list(
    obs: tauri::State<'_, Arc<observability::ObservabilitySystem>>,
) -> Result<Vec<observability::task_monitor::TaskInfo>, AppError> {
    Ok(obs.task_monitor.list_tasks(None))
}

#[tauri::command]
async fn task_summary(
    obs: tauri::State<'_, Arc<observability::ObservabilitySystem>>,
) -> Result<observability::task_monitor::TaskSummary, AppError> {
    Ok(obs.task_monitor.get_task_summary())
}

#[tauri::command]
async fn obs_telemetry(
    obs: tauri::State<'_, Arc<observability::ObservabilitySystem>>,
) -> Result<observability::SystemTelemetry, AppError> {
    Ok(obs.snapshot())
}

#[tauri::command]
async fn obs_metrics(
    obs: tauri::State<'_, Arc<observability::ObservabilitySystem>>,
) -> Result<observability::metrics::MetricsSnapshot, AppError> {
    Ok(obs.metrics.snapshot())
}

#[tauri::command]
async fn obs_timeline(
    obs: tauri::State<'_, Arc<observability::ObservabilitySystem>>,
) -> Result<Vec<observability::timeline::TimelineEntry>, AppError> {
    Ok(obs.timeline.all())
}

#[tauri::command]
async fn crash_list(
    obs: tauri::State<'_, Arc<observability::ObservabilitySystem>>,
) -> Result<observability::crash_reporter::CrashReport, AppError> {
    Ok(obs.crash_reporter.generate_report())
}

// =============================================================================
// Security Commands
// =============================================================================

#[tauri::command]
async fn security_get_policy(
    security: tauri::State<'_, Arc<security::System>>,
) -> Result<security::SecurityPolicy, AppError> {
    let policy = security.policy();
    Ok(policy.clone())
}

#[tauri::command]
async fn security_set_policy(
    security: tauri::State<'_, Arc<security::System>>,
    policy: security::SecurityPolicy,
) -> Result<(), AppError> {
    security.set_policy(policy);
    Ok(())
}

#[tauri::command]
async fn permission_check(
    security: tauri::State<'_, Arc<security::System>>,
    subject: String,
    resource: String,
    action: String,
) -> Result<bool, AppError> {
    Ok(security.permissions.check(&subject, &resource, &action))
}

#[tauri::command]
async fn permission_grant(
    security: tauri::State<'_, Arc<security::System>>,
    subject: String,
    resource: String,
    action: String,
) -> Result<(), AppError> {
    security.permissions.grant(subject, resource, action);
    Ok(())
}

#[tauri::command]
async fn permission_revoke(
    security: tauri::State<'_, Arc<security::System>>,
    subject: String,
    resource: String,
    action: String,
) -> Result<(), AppError> {
    security.permissions.revoke(&subject, &resource, &action);
    Ok(())
}

#[tauri::command]
async fn encryption_hash(
    data: Vec<u8>,
) -> Result<String, AppError> {
    Ok(security::encryption::EncryptionEngine::hash(&data))
}

#[tauri::command]
async fn encryption_verify(
    data: Vec<u8>,
    hash: String,
) -> Result<bool, AppError> {
    Ok(security::encryption::EncryptionEngine::verify(&data, &hash))
}

#[tauri::command]
async fn sandbox_is_isolated(
    security: tauri::State<'_, Arc<security::System>>,
) -> Result<bool, AppError> {
    Ok(security.sandbox.is_isolated().await)
}

#[tauri::command]
async fn sandbox_set_isolated(
    security: tauri::State<'_, Arc<security::System>>,
    isolated: bool,
) -> Result<(), AppError> {
    security.sandbox.set_isolated(isolated).await;
    Ok(())
}

#[tauri::command]
async fn audit_query(
    security: tauri::State<'_, Arc<security::System>>,
    action: Option<String>,
    subject: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<security::audit::AuditLogEntry>, AppError> {
    let since = None;
    let until = None;
    Ok(security.audit.query_log(action.as_deref(), subject.as_deref(), since, until, limit.unwrap_or(100)))
}

#[tauri::command]
async fn audit_export(
    security: tauri::State<'_, Arc<security::System>>,
    format: String,
) -> Result<String, AppError> {
    match format.as_str() {
        "json" => Ok(security.audit.export_json()),
        "csv" => Ok(security.audit.export_csv()),
        _ => Err(AppError::Workspace("Unsupported format. Use 'json' or 'csv'.".to_string())),
    }
}

#[tauri::command]
async fn resource_usage(
    security: tauri::State<'_, Arc<security::System>>,
) -> Result<security::resource_limits::ResourceUsage, AppError> {
    Ok(security.resource_limits.usage().await)

}

#[tauri::command]
async fn resource_check_enforce(
    security: tauri::State<'_, Arc<security::System>>,
) -> Result<security::resource_limits::ResourceUsage, AppError> {
    security.resource_limits.check_and_enforce().await
        .map_err(AppError::Workspace)
}

#[tauri::command]
async fn integrity_check_file(
    security: tauri::State<'_, Arc<security::System>>,
    path: String,
) -> Result<security::integrity::IntegrityStatus, AppError> {
    Ok(security.integrity.check_tampered(std::path::Path::new(&path)))
}

#[tauri::command]
async fn integrity_verify_all(
    security: tauri::State<'_, Arc<security::System>>,
) -> Result<Vec<security::integrity::IntegrityAlert>, AppError> {
    Ok(security.integrity.verify_all())
}

#[tauri::command]
async fn integrity_record_file(
    security: tauri::State<'_, Arc<security::System>>,
    path: String,
) -> Result<String, AppError> {
    security.integrity.hash_and_record(std::path::PathBuf::from(path))
        .map_err(AppError::Workspace)
}

#[tauri::command]
async fn rate_limiter_stats(
    security: tauri::State<'_, Arc<security::System>>,
) -> Result<security::rate_limiter::RateLimiterStats, AppError> {
    Ok(security.rate_limiter.stats())
}

// =============================================================================
// Supervisor Commands — Heartbeat-based AI Agent Observer
// =============================================================================

/// Managed state for the Supervisor Heartbeat System.
pub struct SupervisorState {
    pub supervisor: Arc<core::supervisor::Supervisor>,
    pub sender: Mutex<Option<tokio::sync::mpsc::Sender<core::supervisor::Heartbeat>>>,
}

#[tauri::command]
async fn supervisor_start(
    state: tauri::State<'_, Arc<SupervisorState>>,
) -> Result<(), AppError> {
    let (tx, rx) = core::supervisor::Supervisor::channel();
    let sup = Arc::clone(&state.supervisor);
    tokio::spawn(async move {
        sup.run(rx).await;
    });
    *state.sender.lock().await = Some(tx);
    tracing::info!("Supervisor started");
    Ok(())
}

#[tauri::command]
async fn supervisor_stats(
    state: tauri::State<'_, Arc<SupervisorState>>,
) -> Result<core::supervisor::SupervisorStats, AppError> {
    Ok(state.supervisor.stats().await)
}

#[tauri::command]
async fn supervisor_stop(
    state: tauri::State<'_, Arc<SupervisorState>>,
) -> Result<(), AppError> {
    state.supervisor.shutdown();
    *state.sender.lock().await = None;
    tracing::info!("Supervisor stopped");
    Ok(())
}

// =============================================================================
// System Guardian Commands
// =============================================================================

#[tauri::command]
async fn system_health(
    monitor: tauri::State<'_, Arc<system::SystemMonitor>>,
) -> Result<system::SystemMetrics, AppError> {
    Ok(monitor.collect_metrics().await)
}

#[tauri::command]
async fn system_threat_level(
    dev: tauri::State<'_, Arc<dev::Engine>>,
    monitor: tauri::State<'_, Arc<system::SystemMonitor>>,
) -> Result<system::ThreatReport, AppError> {
    let report = monitor.assess().await;
    monitor.enforce(&report, &dev.governance).await;
    Ok(report)
}

// =============================================================================
// Governance Commands
// =============================================================================

#[tauri::command]
async fn governance_audit_log(
    dev: tauri::State<'_, Arc<dev::Engine>>,
    limit: Option<usize>,
) -> Result<Vec<dev::governance::AuditEntry>, AppError> {
    Ok(dev.governance.audit_log(limit.unwrap_or(50)).await)
}

#[tauri::command]
async fn governance_agent_summary(
    dev: tauri::State<'_, Arc<dev::Engine>>,
    agent_id: String,
) -> Result<Option<dev::governance::AgentAuditSummary>, AppError> {
    Ok(dev.governance.agent_summary(&agent_id).await)
}

#[tauri::command]
async fn governance_validate(
    dev: tauri::State<'_, Arc<dev::Engine>>,
) -> Result<dev::governance::ValidationReport, AppError> {
    let agents = dev.agents.available_agents().await;
    let workflows = dev.agents.list_workflow_defs().await;
    let mut report = dev.governance.validate_agents(&agents);
    let wf_report = dev.governance.validate_workflows(&workflows, &agents);
    let wf_passed = wf_report.workflow_errors.is_empty();
    report.workflow_errors = wf_report.workflow_errors;
    report.total_workflows = wf_report.total_workflows;
    report.passed = report.agent_errors.is_empty() && wf_passed;
    Ok(report)
}

// =============================================================================
// Auto-Update Commands
// =============================================================================

use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub should_update: bool,
    pub manifest: Option<UpdateManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub date: String,
    pub body: String,
}

#[tauri::command]
async fn update_check(app: tauri::AppHandle) -> Result<UpdateCheckResult, AppError> {
    let updater = app.updater().map_err(|e| AppError::Workspace(e.to_string()))?;
    match updater.check().await {
        Ok(Some(update)) => {
            Ok(UpdateCheckResult {
                should_update: true,
                manifest: Some(UpdateManifest {
                    version: update.version.clone(),
                    date: update.date.map(|d| d.to_string()).unwrap_or_default(),
                    body: update.body.unwrap_or_default(),
                }),
            })
        }
        Ok(None) => Ok(UpdateCheckResult { should_update: false, manifest: None }),
        Err(e) => Err(AppError::Workspace(e.to_string())),
    }
}

#[tauri::command]
async fn update_install(app: tauri::AppHandle) -> Result<(), AppError> {
    let updater = app.updater().map_err(|e| AppError::Workspace(e.to_string()))?;
    if let Some(update) = updater.check().await.map_err(|e| AppError::Workspace(e.to_string()))? {
        update.download_and_install(|_, _| {}, || {}).await.map_err(|e| AppError::Workspace(e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
async fn update_relaunch(app: tauri::AppHandle) {
    app.restart();
}

// =============================================================================
// Chat Persistence Commands
// =============================================================================

#[tauri::command]
async fn save_chat_data(data: String) -> Result<(), AppError> {
    let path = dirs_data_dir().join("chats.json");
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await
            .map_err(|e| AppError::Workspace(format!("Failed to create chats dir: {}", e)))?;
    }
    tokio::fs::write(&path, &data).await
        .map_err(|e| AppError::Workspace(format!("Failed to save chats: {}", e)))?;
    Ok(())
}

#[tauri::command]
async fn load_chat_data() -> Result<Option<String>, AppError> {
    let path = dirs_data_dir().join("chats.json");
    match tokio::fs::read_to_string(&path).await {
        Ok(data) => Ok(Some(data)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError::Workspace(format!("Failed to load chats: {}", e))),
    }
}

fn dirs_data_dir() -> std::path::PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            std::path::PathBuf::from(home).join(".local").join("share")
        });
    base.join("prime")
}

// =============================================================================
// Application Entrypoint
// =============================================================================

use std::sync::OnceLock;
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

#[tauri::command]
async fn ping() -> Result<String, AppError> {
    Ok("pong".to_string())
}

pub struct PrimeApp;

impl PrimeApp {
    pub fn run() {

        let env_filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(
                "prime=debug"
                    .parse()
                    .expect("invalid tracing filter directive"),
            )
            // Suppress HTTP client debug logs that could contain Authorization headers
            .add_directive(
                "reqwest=warn"
                    .parse()
                    .expect("invalid filter directive"),
            )
            .add_directive(
                "hyper=warn"
                    .parse()
                    .expect("invalid filter directive"),
            );

        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .init();

        tracing::info!("🚀 {} v{}", crate::core::BRANDING, env!("CARGO_PKG_VERSION"));
        tracing::info!("🔑 Licensed under Prime AI Public License v1.0 — © Aly Ghaly");

        let rt = RUNTIME.get_or_init(|| {
            tokio::runtime::Runtime::new().expect("failed to create tokio runtime")
        });

        // Allow runtime to be used in all crates that call tokio::spawn from setup context
        let _guard = rt.enter();

        tauri::Builder::default()
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_clipboard_manager::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .invoke_handler(tauri::generate_handler![
                get_system_state,
                execute_code,
                search_code,
                query_memory,
                invoke_skill,
                ai_chat,
                list_mcp_servers,
                list_models,
                greet,
                list_plugins,
                plugin_enable,
                plugin_disable,
                plugin_install,
                plugin_uninstall,
                list_workflows,
                workflow_start,
                workflow_cancel,
                workflow_pause,
                workflow_resume,
                mcp_start_server,
                mcp_stop_server,
                mcp_restart_server,
                mcp_toggle_server,
                mcp_add_config,
                mcp_remove_config,
                get_memory_stats,
                delete_memory_entry,
                clear_memory,
                model_test_connection,
                model_add,
                model_remove,
                list_provider_registry,
                list_provider_details,
                get_credential_status,
                race_models,
                parallel_chat,
                broadcast_to_all,
                get_logs,
                get_events,
                ping,
                list_agents,
                get_settings,
                save_settings,
                // Config commands
                get_config,
                save_api_key,
                save_connection_config,
                save_verified_providers,
                save_system_settings,
                // Proxy commands
                proxy_list,
                proxy_add,
                proxy_remove,
                proxy_next,
                proxy_mark_success,
                proxy_mark_failed,
                proxy_active_count,
                // Tools registry commands
                tools::commands::list_all_tools,
                tools::commands::get_tool,
                tools::commands::search_tools,
                tools::commands::toggle_tool,
                tools::commands::enable_tool_category,
                // Security commands
                security_get_policy,
                security_set_policy,
                permission_check,
                permission_grant,
                permission_revoke,
                encryption_hash,
                encryption_verify,
                sandbox_is_isolated,
                sandbox_set_isolated,
                audit_query,
                audit_export,
                resource_usage,
                resource_check_enforce,
                integrity_check_file,
                integrity_verify_all,
                integrity_record_file,
                rate_limiter_stats,
                // Browser automation commands
                browser_connect,
                browser_disconnect,
                browser_navigate,
                browser_snapshot,
                browser_is_connected,
                browser_get_text,
                fetch_web_page,
                // Observability commands
                task_list,
                task_summary,
                obs_telemetry,
                obs_metrics,
                obs_timeline,
                crash_list,
                // Supervisor commands
                supervisor_start,
                supervisor_stats,
                supervisor_stop,
                // Governance commands
                governance_audit_log,
                governance_agent_summary,
                governance_validate,
                // System Guardian commands
                system_health,
                system_threat_level,
                // Payment/Wallet commands
                tools::payments::list_payment_methods,
                tools::payments::get_active_payment_method,
                tools::payments::list_all_wallets,
                tools::payments::connect_wallet,
                tools::payments::disconnect_wallet,
                tools::payments::set_active_payment_method,
                tools::payments::toggle_payment_mode,
                tools::payments::get_payment_mode,
                tools::payments::agent_create_wallet,
                tools::payments::agent_connect_wallet,
                tools::payments::get_connection_info,
                // Payment execution commands
                tools::payments::execute_payment,
                tools::payments::check_balance,
                tools::payments::get_tx_history,
                tools::payments::estimate_gas_fee,
                tools::payments::validate_crypto_address,
                tools::payments::get_token_balance,
                tools::payments::get_swap_estimate,
                tools::payments::get_network_status,
                tools::gateway::list_gateways,
                update_check,
                update_install,
                update_relaunch,
                save_chat_data,
                load_chat_data,
                // IDE commands
                ide::files::list_dir,
                ide::files::read_file,
                ide::files::write_file,
                ide::files::create_dir,
                ide::files::delete_file,
                ide::files::rename_file,
                ide::files::search_files,
                ide::workspace::list_workspaces,
                ide::workspace::add_workspace,
                ide::workspace::remove_workspace,
                ide::workspace::open_workspace,
                ide::import::import_ide_history,
            ])
            .setup(|app| {
                // Initialize core systems — wrap in Arc early for shared ownership
                let runtime = Arc::new(core::Runtime::new());
                let memory = Arc::new(memory::System::new());
                let skills = Arc::new(skills::System::new());
                let execution = Arc::new(execution::Engine::new());
                let verification = Arc::new(verification::System::new());
                let code_intel = Arc::new(code_intel::Engine::new());
                let browser = Arc::new(browser::System::new());
                let dev = Arc::new(dev::Engine::new());
                dev.init_retrieval(
                    memory.vector.clone(),
                    code_intel.symbols.clone(),
                    code_intel.deps.clone(),
                );
                // Seed all agent definitions into the agent registry
                let dev_agents = Arc::clone(&dev);
                tauri::async_runtime::spawn(async move {
                    dev_agents.seed_agents().await;
                });
                // System Guardian — monitors CPU, RAM, disk, temp
                let system_monitor = Arc::new(system::SystemMonitor::new());
                let sm = Arc::clone(&system_monitor);
                let gov = dev.governance.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                    loop {
                        interval.tick().await;
                        let report = sm.assess().await;
                        if !report.level.is_safe() {
                            sm.enforce(&report, &gov).await;
                        }
                    }
                });
                let mcp = Arc::new(mcp::ServerManager::new());

                // Load connection configs for MCP servers that need them
                let telegram_fields: HashMap<String, String> = load_config_inner()
                    .ok()
                    .and_then(|c| c.connection_configs.get("telegram_bot").cloned())
                    .map(|cc| cc.fields)
                    .unwrap_or_default();
                let whatsapp_fields: HashMap<String, String> = load_config_inner()
                    .ok()
                    .and_then(|c| c.connection_configs.get("whatsapp").cloned())
                    .map(|cc| cc.fields)
                    .unwrap_or_default();

                // Register MCP servers — clone Arcs for the registered servers that need them
                {
                    let mc = mcp.clone();
                    let br = browser.clone();
                    let mem = memory.clone();
                    let ci = code_intel.clone();
                    let dv = dev.clone();
                    let st = runtime.storage.clone();
                    tauri::async_runtime::spawn(async move {
                        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                        mc.register(Arc::new(mcp::filesystem::FilesystemMcp::new(vec![home_dir])))
                            .await;
                        mc.register(Arc::new(mcp::git::GitMcp::new())).await;
                        mc.register(Arc::new(mcp::terminal::TerminalMcp::new()))
                            .await;
                        mc.register(Arc::new(mcp::browser::BrowserMcp::new(br)))
                            .await;
                        mc.register(Arc::new(mcp::memory::MemoryMcp::new(mem)))
                            .await;
                        mc.register(Arc::new(mcp::search::SearchMcp::new(ci, dv)))
                            .await;
                        mc.register(Arc::new(mcp::docs::DocsMcp::new(st.clone())))
                            .await;
                        mc.register(Arc::new(mcp::database::DatabaseMcp::new(st)))
                            .await;
                        mc.register(Arc::new(mcp::os::OsMcp::new())).await;
                        mc.register(Arc::new(mcp::telegram::TelegramMcp::with_config(&telegram_fields))).await;
                        mc.register(Arc::new(mcp::discord::DiscordMcp::new())).await;
                        mc.register(Arc::new(mcp::whatsapp::WhatsAppMcp::with_config(&whatsapp_fields))).await;
                    });
                }

                let ai = Arc::new(ai::Router::new());

                // Auto-populate global credential pool from env + config
                {
                    let config_keys = load_config_inner()
                        .map(|c| c.api_keys)
                        .unwrap_or_default();
                    let registry = ai::provider_registry::ProviderRegistry::new();
                    CREDENTIAL_POOL.auto_discover(&registry, &config_keys);
                }

                let event_bus = Arc::new(arch::EventBus::new());
                let security = Arc::new(security::System::new());

                let obs = Arc::new(observability::ObservabilitySystem::new());

                let tools_registry = Arc::new(tools::ToolRegistry::new());

                // Create Supervisor Heartbeat System
                let supervisor_state = Arc::new(SupervisorState {
                    supervisor: Arc::new(core::supervisor::Supervisor::new(
                        core::supervisor::SupervisorConfig::default(),
                    )),
                    sender: Mutex::new(None),
                });

                // Store as managed state
                let payments_manager = Arc::new(tools::PaymentsManager::new());
                app.manage(Arc::clone(&payments_manager));

                app.manage(Arc::clone(&tools_registry));
                app.manage(Arc::clone(&supervisor_state));
                app.manage(Arc::clone(&runtime));
                app.manage(Arc::clone(&memory));
                app.manage(Arc::clone(&skills));
                app.manage(Arc::clone(&execution));
                app.manage(Arc::clone(&verification));
                app.manage(Arc::clone(&code_intel));
                app.manage(Arc::clone(&browser));
                app.manage(Arc::clone(&ai));
                app.manage(Arc::clone(&event_bus));
                app.manage(Arc::clone(&security));
                app.manage(Arc::clone(&obs));
                app.manage(Arc::clone(&mcp));
                app.manage(Arc::clone(&dev));
                app.manage(Arc::clone(&system_monitor));

                // Initialize Phi Brain (local AI intelligence layer)
                let phi_brain = Arc::new(phi_brain::PhiBrain::new());
                app.manage(Arc::clone(&phi_brain));

                // Start Guardian background health check loop
                let guardian_app = app.handle().clone();
                let guardian_phi = Arc::clone(&phi_brain);
                tokio::spawn(async move {
                    guardian_phi.guardian.start_loop(guardian_app).await;
                });

                // Start MCP server manager + health check background task
                let mcp_handle = Arc::clone(&mcp);
                tokio::spawn(async move {
                    if let Err(e) = mcp_handle.start().await {
                        tracing::error!("MCP server manager failed to start: {:?}", e);
                    }
                });
                mcp.start_health_checks();

                Ok(())
            })
            .run(tauri::generate_context!())
            .expect(
                "Prime application crashed during startup — Tauri runtime failed to initialize",
            );
    }
}

// =============================================================================
// Shared Error Type
// =============================================================================

#[derive(Debug, Error, serde::Serialize)]
#[serde(tag = "type", content = "message")]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Network failure during payment: {0}")]
    PaymentNetwork(String),
    #[error("AI Engine error: {0}")]
    AiEngine(String),
    #[error("Execution failed: {0}")]
    Execution(String),
    #[error("Search failed: {0}")]
    Search(String),
    #[error("Workspace error: {0}")]
    Workspace(String),
    #[error("Unauthorized access to resource")]
    Unauthorized,
}
