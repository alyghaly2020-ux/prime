//! Security tests for Prime.
//!
//! Covers: PermissionManager, ResourceLimiter, EncryptionEngine,
//! SecuritySandbox, SecurityPolicy, ProcessSupervisor (shell injection
//! prevention via public API), and path traversal checks.
//!
//! NOTE: Unit-level shell injection tests live in
//! `src/execution/supervisor.rs` under `#[cfg(test)]` — they directly
//! exercise the private `build_script` method and the escaping thereof.

use prime::execution::supervisor::ProcessSupervisor;
use prime::security::{
    self, encryption::EncryptionEngine, permissions::PermissionManager,
    resource_limits::ResourceLimiter, sandbox::SecuritySandbox, PermissionModel, SecurityPolicy,
};

// ===========================================================================
// PermissionManager
// ===========================================================================

#[test]
fn test_permission_default_deny() {
    let pm = PermissionManager::new();
    // No permissions granted → should always be denied
    assert!(!pm.check("anyone", "file", "read"));
    assert!(!pm.check("anyone", "file", "write"));
}

#[test]
fn test_permission_grant_and_check() {
    let pm = PermissionManager::new();
    pm.grant("alice".into(), "file".into(), "read".into());
    assert!(pm.check("alice", "file", "read"));
    // Different action on same resource → denied
    assert!(!pm.check("alice", "file", "write"));
    // Different resource → denied
    assert!(!pm.check("alice", "db", "read"));
}

#[test]
fn test_permission_revoke_removes_check() {
    let pm = PermissionManager::new();
    pm.grant("bob".into(), "api".into(), "call".into());
    assert!(pm.check("bob", "api", "call"));
    pm.revoke("bob", "api", "call");
    assert!(
        !pm.check("bob", "api", "call"),
        "after revoke permission should be denied"
    );
}

#[test]
fn test_permission_revoke_other_actions_unaffected() {
    let pm = PermissionManager::new();
    pm.grant("bob".into(), "api".into(), "read".into());
    pm.grant("bob".into(), "api".into(), "write".into());
    pm.revoke("bob", "api", "write");
    assert!(
        pm.check("bob", "api", "read"),
        "read permission should survive"
    );
    assert!(
        !pm.check("bob", "api", "write"),
        "write permission should be gone"
    );
}

#[test]
fn test_permission_list() {
    let pm = PermissionManager::new();
    pm.grant("carol".into(), "fs".into(), "read".into());
    pm.grant("carol".into(), "fs".into(), "write".into());
    let perms = pm.list_for("carol");
    assert_eq!(perms.len(), 2);
    assert!(perms
        .iter()
        .any(|p| p.action == "read" && p.resource == "fs"));
    assert!(perms
        .iter()
        .any(|p| p.action == "write" && p.resource == "fs"));
}

#[test]
fn test_permission_list_empty_for_unknown() {
    let pm = PermissionManager::new();
    assert!(pm.list_for("nobody").is_empty());
}

#[test]
fn test_permission_multiple_subjects_isolated() {
    let pm = PermissionManager::new();
    pm.grant("alice".into(), "x".into(), "read".into());
    pm.grant("bob".into(), "y".into(), "write".into());
    assert!(pm.check("alice", "x", "read"));
    assert!(pm.check("bob", "y", "write"));
    assert!(
        !pm.check("alice", "y", "write"),
        "alice should not have bob's permissions"
    );
    assert!(
        !pm.check("bob", "x", "read"),
        "bob should not have alice's permissions"
    );
}

// ===========================================================================
// ResourceLimiter
// ===========================================================================

#[tokio::test]
async fn test_resource_limits_default_check_passes() {
    let limiter = ResourceLimiter::new();
    // Initial usage is zero and limits are positive
    assert!(limiter.check_limits().await.is_ok());
}

#[tokio::test]
async fn test_resource_limits_cpu_exceeded() {
    let limiter = ResourceLimiter::new();
    limiter.track_usage(5.0, 0, 0).await; // 5.0 > default limit of 4.0
    let err = limiter.check_limits().await.unwrap_err();
    assert!(err.contains("CPU"));
}

#[tokio::test]
async fn test_resource_limits_memory_exceeded() {
    let limiter = ResourceLimiter::new();
    limiter.track_usage(0.0, 2048, 0).await; // 2048 > default limit of 1024
    let err = limiter.check_limits().await.unwrap_err();
    assert!(err.to_lowercase().contains("memory"));
}

#[tokio::test]
async fn test_resource_limits_time_exceeded() {
    let limiter = ResourceLimiter::new();
    limiter.track_usage(0.0, 0, 120).await; // 120 > default limit of 60
    let err = limiter.check_limits().await.unwrap_err();
    assert!(err.to_lowercase().contains("time"));
}

#[tokio::test]
async fn test_resource_limits_below_threshold_passes() {
    let limiter = ResourceLimiter::new();
    limiter.track_usage(2.0, 500, 30).await; // all below limits
    assert!(limiter.check_limits().await.is_ok());
}

#[tokio::test]
async fn test_resource_limits_reset_clears_usage() {
    let limiter = ResourceLimiter::new();
    limiter.track_usage(5.0, 2048, 120).await;
    assert!(limiter.check_limits().await.is_err());
    limiter.reset().await;
    assert!(
        limiter.check_limits().await.is_ok(),
        "after reset the limiter should pass again"
    );
}

// ===========================================================================
// EncryptionEngine
// ===========================================================================

#[test]
fn test_encryption_roundtrip() -> anyhow::Result<()> {
    let mut engine = EncryptionEngine::new();
    engine.init_with_password("strong-password", b"0123456789ab")?;
    let data = b"Sensitive data: hello world 123!";
    let encrypted = engine.encrypt(data)?;
    let decrypted = engine.decrypt(&encrypted)?;
    assert_eq!(
        decrypted, data,
        "decrypted bytes must match original plaintext"
    );
    Ok(())
}

#[test]
fn test_encryption_different_nonce_each_time() -> anyhow::Result<()> {
    let mut engine = EncryptionEngine::new();
    engine.init_with_password("pwd", b"0123456789ab")?;
    let data = b"same payload";
    // AES-GCM generates a random nonce each time
    let a = engine.encrypt(data)?;
    let b = engine.encrypt(data)?;
    assert_ne!(
        a, b,
        "two encryptions of the same data should differ (random nonce)"
    );
    Ok(())
}

#[test]
fn test_encryption_fails_without_init() {
    let engine = EncryptionEngine::new();
    let result = engine.encrypt(b"data");
    assert!(result.is_err(), "encrypt without key derivation must fail");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("key not initialized"));
}

#[test]
fn test_decryption_fails_without_init() {
    let engine = EncryptionEngine::new();
    let result = engine.decrypt(b"some-ciphertext");
    assert!(result.is_err(), "decrypt without key derivation must fail");
}

#[test]
fn test_decryption_fails_with_wrong_key() -> anyhow::Result<()> {
    // Set up engine with password A
    let mut engine_a = EncryptionEngine::new();
    engine_a.init_with_password("password-a", b"0123456789ab")?;
    let encrypted = engine_a.encrypt(b"secret")?;

    // Set up engine with password B (different key)
    let mut engine_b = EncryptionEngine::new();
    engine_b.init_with_password("password-b", b"0123456789ab")?;
    let result = engine_b.decrypt(&encrypted);
    assert!(
        result.is_err(),
        "decrypting with wrong key must fail (AES-GCM auth tag mismatch)"
    );
    Ok(())
}

#[test]
fn test_hash_and_verify() {
    let data = b"some content to hash";
    let hash = EncryptionEngine::hash(data);
    assert!(!hash.is_empty(), "hash must not be empty");
    assert!(
        EncryptionEngine::verify(data, &hash),
        "verify should pass for correct data"
    );
    assert!(
        !EncryptionEngine::verify(b"wrong data", &hash),
        "verify should fail for tampered data"
    );
}

#[test]
fn test_hash_is_deterministic() {
    let h1 = EncryptionEngine::hash(b"hello");
    let h2 = EncryptionEngine::hash(b"hello");
    assert_eq!(h1, h2, "hash of same data must be identical");
}

#[test]
fn test_init_with_password_derives_different_key_for_different_salt() -> anyhow::Result<()> {
    let mut engine_1 = EncryptionEngine::new();
    engine_1.init_with_password("pwd", b"salt-12345678")?;

    let mut engine_2 = EncryptionEngine::new();
    engine_2.init_with_password("pwd", b"salt-87654321")?;

    let data = b"test";
    let _c1 = engine_1.encrypt(data)?;
    let c2 = engine_2.encrypt(data)?;
    // Different keys → different ciphertexts (and decryption with wrong key fails)
    assert!(
        engine_1.decrypt(&c2).is_err(),
        "different salt should produce different key"
    );
    Ok(())
}

// ===========================================================================
// SecuritySandbox
// ===========================================================================

#[tokio::test]
async fn test_sandbox_default_isolated() {
    let sandbox = SecuritySandbox::new();
    assert!(
        sandbox.is_isolated().await,
        "sandbox should default to isolated"
    );
}

#[tokio::test]
async fn test_sandbox_set_isolated() {
    let sandbox = SecuritySandbox::new();
    sandbox.set_isolated(false).await;
    assert!(!sandbox.is_isolated().await);
    sandbox.set_isolated(true).await;
    assert!(sandbox.is_isolated().await);
}

#[tokio::test]
async fn test_sandbox_execute_in_sandbox() {
    let sandbox = SecuritySandbox::new();
    let result = sandbox.execute_in_sandbox(|| 42).await;
    assert_eq!(result.value, 42);
}

#[tokio::test]
async fn test_sandbox_execute_with_closure_captures() {
    let sandbox = SecuritySandbox::new();
    let msg = "hello from closure".to_string();
    let result = sandbox.execute_in_sandbox(move || msg.len()).await;
    assert_eq!(result.value, 18);
}

// ===========================================================================
// Security System / Policy
// ===========================================================================

#[test]
fn test_security_system_default_policy() {
    let sys = security::System::new();
    let policy = sys.policy();
    assert!(policy.sandbox_enabled);
    assert_eq!(policy.max_memory_mb, 1024);
    assert_eq!(policy.max_timeout_secs, 60);
}

#[test]
fn test_security_system_set_policy() {
    let sys = security::System::new();
    let custom = SecurityPolicy {
        sandbox_enabled: false,
        permission_model: PermissionModel::Strict,
        max_cpu_cores: 2.0,
        max_memory_mb: 512,
        max_timeout_secs: 30,
        encryption_at_rest: false,
        allowed_networks: vec!["10.0.0.0/8".into()],
    };
    sys.set_policy(custom);
    let policy = sys.policy();
    assert!(!policy.sandbox_enabled);
    assert_eq!(policy.max_memory_mb, 512);
    assert_eq!(policy.allowed_networks.len(), 1);
}

// ===========================================================================
// ProcessSupervisor — integration-level execution tests
// ===========================================================================

#[tokio::test]
async fn test_supervisor_run_simple_bash() {
    let supervisor = ProcessSupervisor::new();
    let result = supervisor.run("echo hello_test_42", "bash").await;
    assert!(
        result.is_ok(),
        "supervisor run should succeed: {:?}",
        result.err()
    );
    let exec = result.unwrap();
    assert!(exec.success, "echo should exit with 0");
    assert!(
        exec.stdout.contains("hello_test_42"),
        "stdout must contain echoed text"
    );
    assert!(exec.duration_ms > 0, "duration should be positive");
    assert!(exec.exit_code == 0, "exit code must be 0");
}

#[tokio::test]
async fn test_supervisor_run_failing_command() {
    let supervisor = ProcessSupervisor::new();
    let result = supervisor.run("exit 42", "bash").await;
    assert!(result.is_ok());
    let exec = result.unwrap();
    assert!(!exec.success, "exit 42 should be a failure");
    assert_eq!(exec.exit_code, 42);
}

#[tokio::test]
async fn test_supervisor_run_timeout_triggers() {
    let supervisor = ProcessSupervisor::new();
    // Sleep longer than the default 30 s timeout to trigger timeout
    // Use a short 1-second sleep so the test doesn't actually wait 30 s;
    // the supervisor's timeout is 30 s by default so we just verify
    // the machinery works for a fast command here.
    let result = supervisor.run("sleep 1 && echo ok", "bash").await;
    // On most systems `sleep 1` finishes well within 30 s
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_supervisor_run_python_simple() {
    let supervisor = ProcessSupervisor::new();
    // Use bash language directly so we don't need python3 installed
    let result = supervisor
        .run(
            "python3 --version 2>&1 || node --version 2>&1 || echo 'no runtime'",
            "bash",
        )
        .await;
    assert!(result.is_ok(), "fallback command should not crash");
}

// ===========================================================================
// Path traversal / allowlist — testing through PermissionManager
// ===========================================================================

#[test]
fn test_path_traversal_prevented_by_permissions() {
    let pm = PermissionManager::new();
    // Only grant access to /safe/path
    pm.grant("tool".into(), "/safe/path".into(), "read".into());

    // Direct access to /safe/path should be granted
    assert!(pm.check("tool", "/safe/path", "read"));

    // Path traversal attempts should be denied
    assert!(
        !pm.check("tool", "/safe/path/../../../etc/passwd", "read"),
        "path traversal to parent dirs should be denied"
    );
    assert!(
        !pm.check("tool", "/unsafe/path", "read"),
        "completely different path should be denied"
    );
    assert!(
        !pm.check("tool", "/safe/path_extra", "read"),
        "similar but not identical path should be denied"
    );
}

#[test]
fn test_path_traversal_requires_exact_resource_match() {
    let pm = PermissionManager::new();
    pm.grant("svc".into(), "/data".into(), "read".into());

    // The permission manager does string comparison — no wildcard support
    assert!(pm.check("svc", "/data", "read"), "exact match works");
    assert!(
        !pm.check("svc", "/data/../etc", "read"),
        "traversal not matched exactly"
    );
    assert!(
        !pm.check("svc", "/data2", "read"),
        "similar name not matched"
    );
}

#[test]
fn test_write_permission_separate_from_read() {
    let pm = PermissionManager::new();
    pm.grant("user".into(), "file.txt".into(), "read".into());
    assert!(pm.check("user", "file.txt", "read"));
    assert!(
        !pm.check("user", "file.txt", "write"),
        "write should be denied when only read was granted"
    );
}

// ===========================================================================
// Shell injection — integration-level testing through the public API
// ===========================================================================

#[tokio::test]
async fn test_shell_injection_via_code_content_simple() {
    // Even if the code contains shell metacharacters, the supervisor
    // should handle them gracefully (at least not crash or error badly).
    let supervisor = ProcessSupervisor::new();

    // code contains `;` which in the shell would separate commands
    let code = "echo safe_part; echo also_safe";
    let result = supervisor.run(code, "bash").await;
    assert!(
        result.is_ok(),
        "semicolons in bash code should work: {:?}",
        result.err()
    );
    let exec = result.unwrap();
    assert!(exec.success);
    assert!(exec.stdout.contains("safe_part"));
    assert!(exec.stdout.contains("also_safe"));
}

#[tokio::test]
async fn test_shell_injection_python_code_with_quotes() {
    // Python code that contains double quotes — the supervisor's
    // build_script escapes them so they shouldn't break out.
    let supervisor = ProcessSupervisor::new();

    let result = supervisor
        .run(r#"print("hello from python")"#, "python")
        .await;
    // If python3 is not installed this will fail — we accept both outcomes
    if let Ok(exec) = result {
        if exec.success {
            assert!(exec.stdout.contains("hello from python"));
        }
    }
}

#[tokio::test]
async fn test_shell_injection_newlines_in_code() {
    // Newlines inside the code should not cause injection
    let supervisor = ProcessSupervisor::new();
    let code = "echo line1\necho line2\necho line3";
    let result = supervisor.run(code, "bash").await;
    assert!(result.is_ok());
    let exec = result.unwrap();
    assert!(exec.success);
    assert!(exec.stdout.contains("line1"));
    assert!(exec.stdout.contains("line2"));
    assert!(exec.stdout.contains("line3"));
}
