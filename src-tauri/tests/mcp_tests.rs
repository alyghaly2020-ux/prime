//! MCP (Model Context Protocol) tests for Prime.
//!
//! Covers: FilesystemMcp (path allowlist validation, read/write/list_dir),
//! ServerManager (registration, dispatch, error handling),
//! SearchMcp (handler dispatch), OsMcp (handler dispatch).

use prime::mcp::{
    filesystem::FilesystemMcp, os::OsMcp, search::SearchMcp, McpServer, ServerManager,
};
use prime::{code_intel, dev};
use serde_json::json;
use std::sync::Arc;

fn make_search() -> SearchMcp {
    let ci = Arc::new(code_intel::Engine::new());
    let dv = Arc::new(dev::Engine::new());
    SearchMcp::new(ci, dv)
}

// ===========================================================================
// FilesystemMcp — path allowlist validation
// ===========================================================================

#[tokio::test]
async fn test_filesystem_read_allowed_path_succeeds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file_path = dir.path().join("test.txt");
    std::fs::write(&file_path, "hello from allowed file").unwrap();

    let allowed = vec![dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    let params = json!({ "path": file_path.to_string_lossy().to_string() });
    let result = fs.handle_request("read_file", params).await;
    assert!(
        result.is_ok(),
        "read_file on allowed path should succeed: {:?}",
        result.err()
    );
    let val = result.unwrap();
    assert_eq!(val["content"], "hello from allowed file");
}

#[tokio::test]
async fn test_filesystem_read_denied_path_fails() {
    let allowed_dir = tempfile::tempdir().expect("temp dir");
    let secret_dir = tempfile::tempdir().expect("temp dir");
    let secret_file = secret_dir.path().join("secret.txt");
    std::fs::write(&secret_file, "sensitive data").unwrap();

    // Only allowed_dir is in the allowlist
    let allowed = vec![allowed_dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    let params = json!({ "path": secret_file.to_string_lossy().to_string() });
    let result = fs.handle_request("read_file", params).await;
    assert!(result.is_err(), "read_file on denied path should fail");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not allowed") || err.contains("denied"),
        "error message should indicate path is not allowed, got: {}",
        err
    );
}

#[tokio::test]
async fn test_filesystem_write_allowed_path_succeeds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file_path = dir.path().join("new_file.txt");

    let allowed = vec![dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    let params = json!({
        "path": file_path.to_string_lossy().to_string(),
        "content": "written content"
    });
    let result = fs.handle_request("write_file", params).await;
    assert!(
        result.is_ok(),
        "write_file on allowed path should succeed: {:?}",
        result.err()
    );

    // Verify file was actually written
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "written content");
}

#[tokio::test]
async fn test_filesystem_write_denied_path_fails() {
    let allowed_dir = tempfile::tempdir().expect("temp dir");
    let outside_dir = tempfile::tempdir().expect("temp dir");
    let file_path = outside_dir.path().join("malicious.txt");

    let allowed = vec![allowed_dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    let params = json!({
        "path": file_path.to_string_lossy().to_string(),
        "content": "should not be written"
    });
    let result = fs.handle_request("write_file", params).await;
    assert!(
        result.is_err(),
        "write_file on denied path must be rejected"
    );
}

#[tokio::test]
async fn test_filesystem_read_missing_path_param() {
    let fs = FilesystemMcp::new(vec![]);
    let params = json!({}); // no "path"
    let result = fs.handle_request("read_file", params).await;
    assert!(result.is_err(), "missing path param should fail");
    assert!(result.unwrap_err().to_string().contains("Missing path"));
}

#[tokio::test]
async fn test_filesystem_write_missing_params() {
    let fs = FilesystemMcp::new(vec![]);
    // Missing content
    let params = json!({ "path": "/tmp/x" });
    let r1 = fs.handle_request("write_file", params).await;
    assert!(r1.is_err(), "missing content should fail");

    // Missing path
    let params = json!({ "content": "hi" });
    let r2 = fs.handle_request("write_file", params).await;
    assert!(r2.is_err(), "missing path should fail");
}

#[tokio::test]
async fn test_filesystem_read_nonexistent_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let allowed = vec![dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    // Path does not exist but is within the allowed directory
    let params =
        json!({ "path": dir.path().join("nonexistent.txt").to_string_lossy().to_string() });
    let result = fs.handle_request("read_file", params).await;
    // The canonicalize will fail since the file doesn't exist,
    // which means is_path_allowed returns false!
    assert!(
        result.is_err(),
        "non-existent file should fail (canonicalize fails → not allowed)"
    );
}

// ===========================================================================
// FilesystemMcp — list_dir path validation gap
// ===========================================================================
//
// NOTE: `list_dir` does NOT call `is_path_allowed()` — it directly passes
// the user-supplied path to `tokio::fs::read_dir`.  This is a **security
// gap** compared to `read_file` and `write_file` which do validate.
// The tests below document this inconsistency.

#[tokio::test]
async fn test_filesystem_list_dir_respects_path_validation_regression() {
    let allowed_dir = tempfile::tempdir().expect("temp dir");
    let outside_dir = tempfile::tempdir().expect("temp dir");
    let secret_file = outside_dir.path().join("secret.txt");
    std::fs::write(&secret_file, "pwned").unwrap();

    let allowed = vec![allowed_dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    let params = json!({ "path": outside_dir.path().to_string_lossy().to_string() });
    let result = fs.handle_request("list_dir", params).await;

    assert!(
        result.is_err(),
        "list_dir should reject paths outside allowlist"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not allowed"),
        "error should indicate path not allowed"
    );
}

#[tokio::test]
async fn test_filesystem_list_dir_allowed_path_works() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("a.rs"), "").unwrap();
    std::fs::write(dir.path().join("b.py"), "").unwrap();

    let allowed = vec![dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    let params = json!({ "path": dir.path().to_string_lossy().to_string() });
    let result = fs.handle_request("list_dir", params).await;
    assert!(result.is_ok(), "list_dir on allowed path should work");
    let val = result.unwrap();
    let entries = val["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e.as_str() == Some("a.rs")));
    assert!(entries.iter().any(|e| e.as_str() == Some("b.py")));
}

#[tokio::test]
async fn test_filesystem_list_dir_nonexistent_path() {
    let fs = FilesystemMcp::new(vec![]);
    let params = json!({ "path": "/definitely/does/not/exist/xyz789" });
    let result = fs.handle_request("list_dir", params).await;
    assert!(result.is_err(), "list_dir on non-existent path should fail");
}

#[tokio::test]
async fn test_filesystem_list_dir_missing_param() {
    let fs = FilesystemMcp::new(vec![]);
    let result = fs.handle_request("list_dir", json!({})).await;
    assert!(result.is_err(), "list_dir without path param should fail");
    assert!(result.unwrap_err().to_string().contains("Missing path"));
}

// ===========================================================================
// FilesystemMcp — path traversal attempts
// ===========================================================================

#[tokio::test]
async fn test_filesystem_path_traversal_blocked_by_allowlist() {
    let dir = tempfile::tempdir().expect("temp dir");
    let allowed = vec![dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    // Path traversal attempt: ../
    let traversal = dir.path().join("../../etc/passwd");
    let params = json!({ "path": traversal.to_string_lossy().to_string() });
    let result = fs.handle_request("read_file", params).await;
    // The canonicalize on the traversal path will resolve to /etc/passwd (if exists)
    // or fail if the path doesn't exist. Either way, it should not be allowed.
    assert!(
        result.is_err(),
        "path traversal with '..' should be blocked"
    );
}

/// Symbolic link traversal — confirm symlinks outside the allowlist
/// (that resolve into allowed dirs) are still subject to canonicalize check.
#[tokio::test]
#[cfg(not(target_os = "windows"))] // symlinks require special permissions on Windows
async fn test_filesystem_symlink_outside_allowed_dir() {
    let allowed_dir = tempfile::tempdir().expect("temp dir");
    let outside_dir = tempfile::tempdir().expect("temp dir");
    let secret = outside_dir.path().join("secret.txt");
    std::fs::write(&secret, "leaked").unwrap();

    // Symlink inside allowed_dir -> outside_dir/secret.txt
    let link = allowed_dir.path().join("link.txt");
    std::os::unix::fs::symlink(&secret, &link).unwrap();

    let allowed = vec![allowed_dir.path().to_string_lossy().to_string()];
    let fs = FilesystemMcp::new(allowed);

    // canonicalize resolves the symlink to outside_dir/secret.txt
    // which is NOT in the allowlist -> should be denied
    let params = json!({ "path": link.to_string_lossy().to_string() });
    let result = fs.handle_request("read_file", params).await;
    assert!(result.is_err(), "symlink to outside dir should be blocked");
}

// ===========================================================================
// FilesystemMcp — unknown method
// ===========================================================================

#[tokio::test]
async fn test_filesystem_unknown_method() {
    let fs = FilesystemMcp::new(vec![]);
    let result = fs.handle_request("nonexistent_method", json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown method"));
}

// ===========================================================================
// ServerManager
// ===========================================================================

#[tokio::test]
async fn test_server_manager_register_and_list() {
    let mgr = ServerManager::new();
    assert!(
        mgr.list_servers().await.is_empty(),
        "new manager should have no servers"
    );

    let fs = Arc::new(FilesystemMcp::new(vec![]));
    mgr.register(fs).await;

    let servers = mgr.list_servers().await;
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].id, "filesystem");
    assert!(servers[0].running);
}

#[tokio::test]
async fn test_server_manager_register_multiple() {
    let mgr = ServerManager::new();
    mgr.register(Arc::new(FilesystemMcp::new(vec![]))).await;
    mgr.register(Arc::new(make_search())).await;
    mgr.register(Arc::new(OsMcp::new())).await;

    let servers = mgr.list_servers().await;
    assert_eq!(servers.len(), 3);
    let ids: std::collections::HashSet<String> = servers.into_iter().map(|s| s.id).collect();
    assert!(ids.contains("filesystem"));
    assert!(ids.contains("search"));
    assert!(ids.contains("os"));
}

#[tokio::test]
async fn test_server_manager_call_nonexistent() {
    let mgr = ServerManager::new();
    let result = mgr.call("ghost_server", "ping", json!({})).await;
    assert!(result.is_err(), "calling a non-existent server should fail");
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_server_manager_call_dispatches_to_correct_server() {
    let mgr = ServerManager::new();
    let dir = tempfile::tempdir().expect("temp dir");
    let file_path = dir.path().join("hello.txt");
    std::fs::write(&file_path, "world").unwrap();

    mgr.register(Arc::new(FilesystemMcp::new(vec![dir
        .path()
        .to_string_lossy()
        .to_string()])))
        .await;

    let result = mgr
        .call(
            "filesystem",
            "read_file",
            json!({
                "path": file_path.to_string_lossy().to_string()
            }),
        )
        .await;

    assert!(
        result.is_ok(),
        "dispatch to filesystem server should succeed"
    );
    assert_eq!(result.unwrap()["content"], "world");
}

#[tokio::test]
async fn test_server_manager_call_with_server_error() {
    let mgr = ServerManager::new();
    let fs = Arc::new(FilesystemMcp::new(vec![]));
    mgr.register(fs).await;

    // Denied path -> server returns an error, which should propagate
    let result = mgr
        .call(
            "filesystem",
            "read_file",
            json!({
                "path": "/etc/shadow"
            }),
        )
        .await;

    assert!(
        result.is_err(),
        "server errors should propagate through the manager"
    );
}

// ===========================================================================
// SearchMcp
// ===========================================================================

#[tokio::test]
async fn test_search_mcp_web_search_handler() {
    let search = make_search();
    let result = search
        .handle_request(
            "web_search",
            json!({
                "query": "rust programming"
            }),
        )
        .await;
    assert!(result.is_ok(), "web_search handler should not fail");
    let val = result.unwrap();
    assert!(
        val["results"].is_array(),
        "web_search should return a results array"
    );
}

#[tokio::test]
async fn test_search_mcp_code_search_handler() {
    let search = make_search();
    let result = search
        .handle_request(
            "code_search",
            json!({
                "query": "fn main",
                "path": "/src"
            }),
        )
        .await;
    assert!(result.is_ok(), "code_search handler should not fail");
    let val = result.unwrap();
    assert!(
        val["results"].is_array(),
        "code_search should return a results array"
    );
}

#[tokio::test]
async fn test_search_mcp_unknown_method() {
    let search = make_search();
    let result = search.handle_request("delete_all", json!({})).await;
    assert!(result.is_err(), "unknown method should fail");
    assert!(result.unwrap_err().to_string().contains("Unknown method"));
}

#[tokio::test]
async fn test_search_mcp_method_requires_string_queries() {
    let search = make_search();
    // Query is missing (not a string), but SearchMcp does not validate — it uses
    // as_str() which returns None, and then just returns empty results.
    let result = search
        .handle_request(
            "web_search",
            json!({
                "query": 12345
            }),
        )
        .await;
    assert!(
        result.is_ok(),
        "non-string query should still be handled gracefully"
    );
}

// ===========================================================================
// OsMcp
// ===========================================================================

#[tokio::test]
async fn test_os_mcp_info_handler() {
    let os = OsMcp::new();
    let result = os.handle_request("info", json!({})).await;
    assert!(result.is_ok(), "os info handler should succeed");
    let val = result.unwrap();
    assert!(val["os"].is_string(), "os field should be a string");
    assert!(val["arch"].is_string(), "arch field should be a string");
    assert!(val["cpus"].is_number(), "cpus field should be a number");
}

#[tokio::test]
async fn test_os_mcp_env_handler() {
    let os = OsMcp::new();
    // PATH should exist on all platforms
    let result = os.handle_request("env", json!({ "key": "PATH" })).await;
    assert!(result.is_ok(), "env handler should succeed");
    let val = result.unwrap();
    assert_eq!(val["key"], "PATH");
    assert!(val["value"].is_string(), "PATH should have a string value");
}

#[tokio::test]
async fn test_os_mcp_env_missing_key() {
    let os = OsMcp::new();
    let result = os.handle_request("env", json!({})).await;
    assert!(result.is_ok(), "env without key should still succeed");
    let val = result.unwrap();
    assert_eq!(val["key"], "", "missing key should default to empty string");
    assert!(val["value"].is_null(), "missing key should have null value");
}

#[tokio::test]
async fn test_os_mcp_notify_handler() {
    if std::env::var("GITHUB_ACTIONS").is_ok() || std::env::var("CI").is_ok() {
        return;
    }
    let os = OsMcp::new();
    let result = os
        .handle_request(
            "notify",
            json!({
                "title": "Test",
                "body": "Hello from test"
            }),
        )
        .await;
    assert!(result.is_ok(), "notify handler should succeed");
    assert_eq!(result.unwrap()["sent"], true);
}

#[tokio::test]
async fn test_os_mcp_clipboard_handler() {
    if std::env::var("GITHUB_ACTIONS").is_ok() || std::env::var("CI").is_ok() {
        return;
    }
    let os = OsMcp::new();
    let result = os.handle_request("clipboard", json!({})).await;
    assert!(result.is_ok(), "clipboard handler should succeed");
}

#[tokio::test]
async fn test_os_mcp_unknown_method() {
    let os = OsMcp::new();
    let result = os.handle_request("reboot", json!({})).await;
    assert!(result.is_err(), "unknown method should fail");
    assert!(result.unwrap_err().to_string().contains("Unknown method"));
}

// ===========================================================================
// ServerManager — lifecycle (start / stop)
// ===========================================================================

#[tokio::test]
async fn test_server_manager_start_stop() {
    let mgr = ServerManager::new();
    // start() and stop_all() should not fail with no registered servers
    assert!(
        mgr.start().await.is_ok(),
        "start with no servers should succeed"
    );
    assert!(
        mgr.stop_all().await.is_ok(),
        "stop_all with no servers should succeed"
    );

    // With a registered server
    mgr.register(Arc::new(make_search())).await;
    assert!(mgr.start().await.is_ok());
    assert!(mgr.stop_all().await.is_ok());
}

// ===========================================================================
// McpServer trait — id / name
// ===========================================================================

#[test]
fn test_filesystem_mcp_identity() {
    let fs = FilesystemMcp::new(vec![]);
    assert_eq!(fs.id(), "filesystem");
    assert_eq!(fs.name(), "Filesystem MCP");
}

#[test]
fn test_search_mcp_identity() {
    let search = make_search();
    assert_eq!(search.id(), "search");
    assert_eq!(search.name(), "Search MCP");
}

#[test]
fn test_os_mcp_identity() {
    let os = OsMcp::new();
    assert_eq!(os.id(), "os");
    assert_eq!(os.name(), "OS MCP");
}
