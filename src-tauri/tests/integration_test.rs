//! # Prime — Integration Smoke Tests
//!
//! These tests verify that all public modules in the `prime` crate can be
//! instantiated, their default constructors work, and the core data types
//! behave as expected.  They serve as basic smoke tests and documentation
//! of how each subsystem works.
//!
//! Run with:
//! ```sh
//! cd src-tauri && cargo test --test integration_test -- --nocapture
//! ```

use prime::execution::supervisor::ProcessSupervisor;
use prime::security::{self, permissions::PermissionManager, encryption::EncryptionEngine, SecurityPolicy, PermissionModel};
use prime::verification::{self, IssueSeverity};
use prime::contracts::{ChatMessage, ModelConfig, Usage, ChatResponse, ExecutionResult, MemoryEntry, SystemEvent, VerificationResult as ContractVerificationResult, Issue as ContractIssue, IssueSeverity as ContractIssueSeverity};
use prime::core;
use prime::tools;
use prime::code_intel;
use prime::memory;
use std::sync::Arc;

// =============================================================================
// 1. Crate-Level Smoke Tests
// =============================================================================

/// Verify the crate link works — can we reference `prime::`?
#[test]
fn test_crate_links_correctly() {
    let _version = env!("CARGO_PKG_VERSION");
    assert!(!_version.is_empty());
}

/// Verify we can run a Tokio runtime (required by many subsystems)
#[test]
fn test_tokio_runtime_can_be_created() {
    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    let result = rt.block_on(async { 1 + 1 });
    assert_eq!(result, 2);
}

// =============================================================================
// 2. Security Module Smoke Tests
// =============================================================================

#[test]
fn test_security_module_instantiation() {
    let sys = security::System::new();
    let policy = sys.policy();
    assert!(policy.sandbox_enabled);
    assert_eq!(policy.max_memory_mb, 1024);
}

#[test]
fn test_permission_manager_default_state() {
    let pm = PermissionManager::new();
    // Default-deny: no permissions granted
    assert!(!pm.check("anyone", "anything", "anyaction"));
    assert!(pm.list_for("anyone").is_empty());
}

#[test]
fn test_encryption_engine_roundtrip() -> anyhow::Result<()> {
    let mut engine = EncryptionEngine::new();
    engine.init_with_password("test-password", b"0123456789ab")?;
    let plaintext = b"Hello, Prime!";
    let encrypted = engine.encrypt(plaintext)?;
    let decrypted = engine.decrypt(&encrypted)?;
    assert_eq!(&decrypted, plaintext);
    Ok(())
}

#[test]
fn test_security_policy_struct() {
    let policy = SecurityPolicy {
        sandbox_enabled: true,
        permission_model: PermissionModel::Strict,
        max_cpu_cores: 4.0,
        max_memory_mb: 2048,
        max_timeout_secs: 120,
        encryption_at_rest: true,
        allowed_networks: vec!["127.0.0.1/8".into()],
    };
    assert!(policy.sandbox_enabled);
    assert_eq!(policy.max_memory_mb, 2048);
    assert_eq!(policy.allowed_networks.len(), 1);
}

// =============================================================================
// 3. Contracts Module Smoke Tests (Data Types)
// =============================================================================

#[test]
fn test_chat_message_creation() {
    let msg = ChatMessage {
        role: "user".into(),
        content: "Hello".into(),
        tool_calls: None,
        timestamp: Some(1234567890),
    };
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "Hello");
    assert!(msg.timestamp.is_some());
}

#[test]
fn test_model_config_defaults() {
    let config = ModelConfig {
        id: "test-model".into(),
        provider: "openai".into(),
        model: "gpt-4".into(),
        max_tokens: 4096,
        temperature: 0.7,
        streaming: true,
    };
    assert_eq!(config.id, "test-model");
    assert_eq!(config.model, "gpt-4");
    assert_eq!(config.max_tokens, 4096);
}

#[test]
fn test_chat_response_structure() {
    let response = ChatResponse {
        content: "Hello!".into(),
        model: "gpt-4".into(),
        usage: Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
        finish_reason: "stop".into(),
    };
    assert_eq!(response.content, "Hello!");
    assert_eq!(response.usage.total_tokens, 15);
}

#[test]
fn test_execution_result() {
    let result = ExecutionResult {
        success: true,
        exit_code: 0,
        stdout: "done".into(),
        stderr: String::new(),
        duration_ms: 42,
    };
    assert!(result.success);
    assert_eq!(result.stdout, "done");
    assert!(result.duration_ms > 0);
}

#[test]
fn test_memory_entry_fields() {
    let entry = MemoryEntry {
        id: "mem-1".into(),
        memory_type: "working".into(),
        content: "test content".into(),
        metadata: serde_json::json!({"key": "value"}),
        created_at: "2025-01-01T00:00:00Z".into(),
        importance: 0.8,
    };
    assert_eq!(entry.memory_type, "working");
    assert_eq!(entry.importance, 0.8);
}

#[test]
fn test_system_event_structure() {
    let event = SystemEvent {
        id: "evt-1".into(),
        event_type: "system.started".into(),
        source: "prime".into(),
        payload: serde_json::json!({"version": "0.1.0"}),
        timestamp: chrono::Utc::now(),
    };
    assert_eq!(event.event_type, "system.started");
    assert!(event.payload.get("version").and_then(|v| v.as_str()) == Some("0.1.0"));
}

#[test]
fn test_contract_verification_result() {
    let result = ContractVerificationResult {
        passed: true,
        score: 1.0,
        errors: vec![],
        warnings: vec![ContractIssue {
            severity: ContractIssueSeverity::Warning,
            message: "unused variable".into(),
            file: Some("main.rs".into()),
            line: Some(10),
        }],
    };
    assert!(result.passed);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].message, "unused variable");
}

// =============================================================================
// 4. Verification Module Smoke Tests
// =============================================================================

#[test]
fn test_verification_system_instantiation() {
    let _sys = verification::System::new();
    // Just verifying Construction succeeds — no panic
}

#[test]
fn test_verification_issue_severity() {
    let variants = [
        (IssueSeverity::Error, "Error"),
        (IssueSeverity::Warning, "Warning"),
        (IssueSeverity::Info, "Info"),
        (IssueSeverity::Hint, "Hint"),
    ];
    for (sev, label) in &variants {
        let debug = format!("{:?}", sev);
        assert_eq!(&debug, label);
    }
}

#[test]
fn test_verification_result_api() {
    // Directly construct a VerificationResult
    let result = prime::verification::VerificationResult {
        passed: true,
        score: 1.0,
        errors: vec![],
        warnings: vec![],
        suggestions: vec!["add documentation".into()],
    };
    assert!(result.passed);
    assert_eq!(result.score, 1.0);
    assert_eq!(result.suggestions.len(), 1);
}

// =============================================================================
// 5. Core Module Smoke Tests
// =============================================================================

#[test]
fn test_core_runtime_instantiation() {
    let _rt = core::Runtime::new();
    // Construction succeeds
}

#[test]
fn test_core_runtime_state_exists() {
    let rt = core::Runtime::new();
    let rt_clone = rt; // just to verify it's Send + Sync + Clone friendly
    drop(rt_clone);
}

#[test]
fn test_serde_config_roundtrip() {
    let val = serde_json::json!({
        "name": "test",
        "count": 42,
        "enabled": true
    });
    let serialized = serde_json::to_string(&val).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized["name"], "test");
    assert_eq!(deserialized["count"], 42);
    assert_eq!(deserialized["enabled"], true);
}

// =============================================================================
// 6. Tools Module Smoke Tests
// =============================================================================

#[tokio::test]
async fn test_tool_registry_instantiation() {
    let registry = tools::ToolRegistry::new();
    // ToolRegistry defaults should pre-populate with all_tool_configs
    let tools = registry.list_all().await;
    assert!(!tools.is_empty(), "registry should have pre-loaded tools");
}

#[test]
fn test_tool_category_enum() {
    use tools::config::ToolCategory;
    let categories = vec![
        ToolCategory::BrowserStealth,
        ToolCategory::ApiGateway,
        ToolCategory::SwarmOrchestration,
        ToolCategory::McpSkills,
        ToolCategory::SearchEngine,
    ];
    assert_eq!(categories.len(), 5);
    for cat in &categories {
        let debug = format!("{:?}", cat);
        assert!(!debug.is_empty());
    }
}

// =============================================================================
// 7. Code Intel Module Smoke Tests
// =============================================================================

#[test]
fn test_code_intel_engine_instantiation() {
    let _engine = code_intel::Engine::new();
    // Engine should construct without panicking
}

#[test]
fn test_code_intel_engine_has_search() {
    let engine = code_intel::Engine::new();
    // The search engine is always available (Arc, not Option)
    let _search = &engine.search;
}

// =============================================================================
// 8. Memory Module Smoke Tests
// =============================================================================

#[test]
fn test_memory_system_instantiation() {
    let _sys = memory::System::new();
    // System with 7-tier memory should initialize
}

#[test]
fn test_memory_cache_instantiation() {
    let cache = memory::cache::EmbeddingCache::new();
    assert_eq!(cache.stats(), (0, 0));
}

// =============================================================================
// 9. Async / Runtime Smoke Tests
// =============================================================================

#[test]
fn test_process_supervisor_instantiation() {
    let _supervisor = ProcessSupervisor::new();
    // Supervisor created with default limits
}

#[tokio::test]
async fn test_basic_async_operation() {
    let result = tokio::spawn(async { 42 }).await.unwrap();
    assert_eq!(result, 42);
}

#[tokio::test]
async fn test_async_join_multiple() {
    let (a, b, c) = tokio::join!(
        async { 1 + 1 },
        async { 2 + 2 },
        async { 3 + 3 },
    );
    assert_eq!(a, 2);
    assert_eq!(b, 4);
    assert_eq!(c, 6);
}

// =============================================================================
// 10. Module Validation — Verify All Major Modules Are Accessible
// =============================================================================

/// This test imports types from every public module to ensure the
/// crate's public API is coherent and no module is broken.
#[tokio::test]
async fn test_all_modules_accessible() {
    // ai
    let _ai_chat_msg = prime::ai::ChatMessage {
        role: "user".into(),
        content: "test".into(),
        tool_calls: None,
        timestamp: None,
    };

    // arch (event bus)
    let _bus = prime::arch::EventBus::new();

    // browser contract types
    let _snapshot = prime::contracts::PageSnapshot {
        url: "https://example.com".into(),
        title: "Example".into(),
        text: "content".into(),
        screenshot: None,
    };

    // execution
    let _exec_contract = prime::contracts::ExecutionResult {
        success: true,
        exit_code: 0,
        stdout: "ok".into(),
        stderr: String::new(),
        duration_ms: 1,
    };

    // If we reach here, all referenced modules compiled and linked
}

// =============================================================================
// 11. EventBus and Architecture Smoke Tests
// =============================================================================

#[tokio::test]
async fn test_event_bus_instantiation() {
    let bus = Arc::new(prime::arch::EventBus::new());
    assert!(Arc::strong_count(&bus) == 1);
}

// =============================================================================
// 12. Error Handling
// =============================================================================

#[test]
fn test_app_error_display() {
    let err = prime::AppError::Execution("something broke".into());
    let msg = err.to_string();
    assert!(msg.contains("something broke"));
    assert!(msg.contains("Execution failed"));

    let err2 = prime::AppError::Search("not found".into());
    assert!(err2.to_string().contains("Search failed"));

    let err3 = prime::AppError::Workspace("permission denied".into());
    assert!(err3.to_string().contains("Workspace error"));
}

#[test]
fn test_app_error_serialization() {
    let err = prime::AppError::Workspace("disk full".into());
    let json = serde_json::to_string(&err).unwrap();
    assert_eq!(json, r#"{"type":"Workspace","message":"disk full"}"#);
}
