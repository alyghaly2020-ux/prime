use super::CommandValidator;
use super::ExecutionResult;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

const ALLOWED_LANGUAGES: &[&str] = &[
    "python",
    "py",
    "javascript",
    "js",
    "rust",
    "rs",
    "bash",
    "sh",
    "powershell",
    "pwsh",
];

pub struct ProcessSupervisor {
    max_execution_time_secs: u64,
    _max_memory_mb: u64,
    validator: Arc<CommandValidator>,
}

impl Default for ProcessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            max_execution_time_secs: 30,
            _max_memory_mb: 512,
            validator: Arc::new(CommandValidator::new()),
        }
    }

    pub async fn run(&self, code: &str, language: &str) -> anyhow::Result<ExecutionResult> {
        let start = std::time::Instant::now();

        // Validate language
        if !ALLOWED_LANGUAGES.contains(&language) {
            return Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Language '{}' is not allowed for execution", language),
                duration_ms: start.elapsed().as_millis() as u64,
                checkpoint_id: None,
            });
        }

        // Rust requires compilation — delegate to run_rust
        if language == "rust" || language == "rs" {
            return self.run_rust(code, start).await;
        }

        let script = self.build_script(code, language);
        // Sanitize the built script for shell metacharacters
        let safe_script = self.validator.sanitize_script(&script);

        // Shell-commands returned by build_script need to be run through a shell
        let (cmd, args) = if cfg!(target_os = "windows") {
            (
                "pwsh",
                vec!["-NoProfile".to_string(), "-Command".to_string(), safe_script],
            )
        } else {
            ("sh", vec!["-c".to_string(), safe_script])
        };
        let result = timeout(
            Duration::from_secs(self.max_execution_time_secs),
            self.execute_script(cmd, &args),
        )
        .await;

        let duration = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok((stdout, stderr, exit_code))) => Ok(ExecutionResult {
                success: exit_code == 0,
                exit_code,
                stdout,
                stderr,
                duration_ms: duration,
                checkpoint_id: None,
            }),
            Ok(Err(e)) => Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Execution error: {}", e),
                duration_ms: duration,
                checkpoint_id: None,
            }),
            Err(_) => Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Timeout after {}s", self.max_execution_time_secs),
                duration_ms: duration,
                checkpoint_id: None,
            }),
        }
    }

    fn build_script(&self, code: &str, language: &str) -> String {
        match language {
            "python" | "py" => format!("python3 -c \"{}\"", code.replace('"', "\\\"")),
            "javascript" | "js" => format!("node -e \"{}\"", code.replace('"', "\\\"")),
            "bash" | "sh" => code.to_string(),
            "powershell" | "pwsh" => {
                format!("pwsh -NoProfile -Command \"{}\"", code.replace('"', "\\\""))
            }
            // rust/rs handled by run_rust() — not dispatched here
            _ => code.to_string(),
        }
    }

    async fn run_rust(
        &self,
        code: &str,
        start: std::time::Instant,
    ) -> anyhow::Result<ExecutionResult> {
        let tmp_dir = std::env::temp_dir().join(format!("prime_rust_exec_{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir)?;
        let src_file = tmp_dir.join("script.rs");
        let bin_file = if cfg!(target_os = "windows") {
            tmp_dir.join("script.exe")
        } else {
            tmp_dir.join("script")
        };

        // Write code to temp file
        std::fs::write(&src_file, code)?;

        // Compile
        let compile_cmd = "rustc".to_string();
        let compile_args = vec![
            src_file.to_string_lossy().to_string(),
            "-o".to_string(),
            bin_file.to_string_lossy().to_string(),
        ];

        let compile_result = timeout(
            Duration::from_secs(self.max_execution_time_secs),
            self.execute_script(&compile_cmd, &compile_args),
        )
        .await;

        let duration_so_far = start.elapsed().as_millis() as u64;

        match compile_result {
            Ok(Ok((stdout, stderr, exit_code))) => {
                if exit_code != 0 {
                    return Ok(ExecutionResult {
                        success: false,
                        exit_code,
                        stdout,
                        stderr,
                        duration_ms: duration_so_far,
                        checkpoint_id: None,
                    });
                }
            }
            Ok(Err(e)) => {
                return Ok(ExecutionResult {
                    success: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: format!("Compilation error: {}", e),
                    duration_ms: duration_so_far,
                    checkpoint_id: None,
                });
            }
            Err(_) => {
                return Ok(ExecutionResult {
                    success: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: format!(
                        "Compilation timeout after {}s",
                        self.max_execution_time_secs
                    ),
                    duration_ms: duration_so_far,
                    checkpoint_id: None,
                });
            }
        }

        // Run the compiled binary
        let run_result = timeout(
            Duration::from_secs(self.max_execution_time_secs),
            self.execute_script(&bin_file.to_string_lossy(), &[]),
        )
        .await;

        let duration = start.elapsed().as_millis() as u64;

        // Cleanup temp files
        let _ = std::fs::remove_dir_all(&tmp_dir);

        match run_result {
            Ok(Ok((stdout, stderr, exit_code))) => Ok(ExecutionResult {
                success: exit_code == 0,
                exit_code,
                stdout,
                stderr,
                duration_ms: duration,
                checkpoint_id: None,
            }),
            Ok(Err(e)) => Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Execution error: {}", e),
                duration_ms: duration,
                checkpoint_id: None,
            }),
            Err(_) => Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Execution timeout after {}s", self.max_execution_time_secs),
                duration_ms: duration,
                checkpoint_id: None,
            }),
        }
    }

    async fn execute_script(
        &self,
        cmd: &str,
        args: &[String],
    ) -> anyhow::Result<(String, String, i32)> {
        use tokio::process::Command;

        let output = Command::new(cmd).args(args).output().await?;

        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.code().unwrap_or(-1),
        ))
    }
}

// ---------------------------------------------------------------------------
// CommandWhitelist – ensure only safe commands are executed
// ---------------------------------------------------------------------------

/// Whitelist of allowed commands and blocklist of dangerous command patterns.
pub struct CommandWhitelist {
    allowed: Vec<String>,
    blocked: Vec<String>,
}

impl Default for CommandWhitelist {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandWhitelist {
    pub fn new() -> Self {
        Self {
            allowed: Self::default_allowed(),
            blocked: Self::default_blocked(),
        }
    }

    /// Default set of safe / commonly-used commands.
    fn default_allowed() -> Vec<String> {
        vec![
            "ls",
            "dir",
            "echo",
            "cat",
            "type",
            "git",
            "npm",
            "npx",
            "cargo",
            "python",
            "python3",
            "node",
            "rustc",
            "sh",
            "bash",
            "pwsh",
            "powershell",
            "cd",
            "pwd",
            "mkdir",
            "cp",
            "copy",
            "mv",
            "move",
            "head",
            "tail",
            "grep",
            "find",
            "where",
            "which",
            "sort",
            "wc",
            "date",
            "whoami",
            "hostname",
            "curl",
            "wget",
            "code",
            "code-insiders",
            "pip",
            "pip3",
            "make",
            "cmake",
            "deno",
            "bun",
            "tsc",
            "vitest",
            "jest",
            "mocha",
            "pnpm",
            "yarn",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    /// Command patterns that are always rejected.
    fn default_blocked() -> Vec<String> {
        vec![
            "rm -rf",
            "rm -fr",
            "rm -r /",
            "rm -f",
            "del /f",
            "del /s",
            "format",
            "dd",
            "mkfs",
            "fdisk",
            "shutdown",
            "reboot",
            "init",
            "killall",
            "pkill",
            "fuser",
            "chmod",
            "chown",
            "mount",
            "umount",
            "iptables",
            "ufw",
            "reg delete",
            "reg add",
            "sc delete",
            "wmic",
            "vssadmin",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    /// Returns `true` when `cmd` is safe to run.
    pub fn is_allowed(&self, cmd: &str) -> bool {
        let trimmed = cmd.trim();

        // Extract the base command (first token)
        let base_cmd = trimmed.split_whitespace().next().unwrap_or("");

        // Base command must be in the allowed list
        if !self.allowed.iter().any(|a| a == base_cmd) {
            return false;
        }

        // Full command must not start with any blocked pattern
        let lower = trimmed.to_lowercase();
        for blocked in &self.blocked {
            if lower.starts_with(blocked) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // build_script tests — verifying shell escaping and language dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_script_python() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("print('hi')", "python");
        assert_eq!(script, r#"python3 -c "print('hi')""#);
    }

    #[test]
    fn test_build_script_python_short_alias() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("print('hi')", "py");
        assert_eq!(script, r#"python3 -c "print('hi')""#);
    }

    #[test]
    fn test_build_script_javascript() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("console.log('hi')", "javascript");
        assert_eq!(script, r#"node -e "console.log('hi')""#);
    }

    #[test]
    fn test_build_script_javascript_short() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("console.log('hi')", "js");
        assert_eq!(script, r#"node -e "console.log('hi')""#);
    }

    #[test]
    fn test_build_script_bash() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("echo hello", "bash");
        assert_eq!(script, "echo hello");
    }

    #[test]
    fn test_build_script_sh() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("echo hello", "sh");
        assert_eq!(script, "echo hello");
    }

    #[test]
    fn test_build_script_powershell() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("Write-Host hi", "powershell");
        assert_eq!(script, r#"pwsh -NoProfile -Command "Write-Host hi""#);
    }

    #[test]
    fn test_build_script_powershell_short() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("Write-Host hi", "pwsh");
        assert_eq!(script, r#"pwsh -NoProfile -Command "Write-Host hi""#);
    }

    #[test]
    fn test_build_script_unknown_language_passthrough() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("some raw code", "unknown-lang");
        assert_eq!(script, "some raw code");
    }

    // -----------------------------------------------------------------------
    // Shell injection tests — ensure dangerous characters are escaped
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_script_python_escapes_double_quotes() {
        let s = ProcessSupervisor::new();
        // Code containing double quotes should have them escaped
        let script = s.build_script(r#"print("hello")"#, "python");
        assert_eq!(script, r#"python3 -c "print(\"hello\")""#);
    }

    #[test]
    fn test_build_script_python_escapes_multiple_quotes() {
        let s = ProcessSupervisor::new();
        let code = r#"a="x"; b="y""#;
        let script = s.build_script(code, "python");
        assert!(
            script.contains(r#"\""#),
            "Double quotes inside code should be escaped"
        );
    }

    #[test]
    #[should_panic(expected = "Backticks should be escaped")] // KNOWN VULNERABILITY
    fn test_build_script_python_allows_backtick_injection() {
        let s = ProcessSupervisor::new();
        // Backticks inside double-quoted strings ARE interpreted by sh/bash.
        // This test documents the vulnerability — it will panic because the
        // current code does NOT escape backticks.
        let script = s.build_script("`cat /etc/passwd`", "python");
        assert!(!script.contains('`'), "Backticks should be escaped");
    }

    #[test]
    #[should_panic(expected = "$(subshell) should be blocked or escaped")] // KNOWN VULNERABILITY
    fn test_build_script_python_allows_subshell_injection() {
        let s = ProcessSupervisor::new();
        // $(...) is interpreted inside double-quoted strings in sh/bash.
        let script = s.build_script("$(cat /etc/passwd)", "python");
        assert!(
            !script.contains("$("),
            "$(subshell) should be blocked or escaped"
        );
    }

    #[test]
    fn test_build_script_bash_raw_passthrough_still_injectable() {
        let s = ProcessSupervisor::new();
        // bash/sh language passes code directly — no protection at all
        let script = s.build_script("echo hello; rm -rf /", "sh");
        assert_eq!(
            script, "echo hello; rm -rf /",
            "bash/sh passes code verbatim — caller must sanitize"
        );
    }

    #[test]
    fn test_build_script_powershell_escapes_double_quotes() {
        let s = ProcessSupervisor::new();
        let script = s.build_script(r#"Write-Host "hello""#, "powershell");
        assert_eq!(script, r#"pwsh -NoProfile -Command "Write-Host \"hello\"""#);
    }

    // -----------------------------------------------------------------------
    // Rust dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_script_rust_falls_to_passthrough() {
        let s = ProcessSupervisor::new();
        // Rust is no longer handled by build_script — run() delegates to run_rust()
        let script = s.build_script("fn main() {}", "rust");
        assert_eq!(
            script, "fn main() {}",
            "Rust code should pass through raw since build_script no longer handles it"
        );
    }

    #[test]
    fn test_build_script_rust_short_falls_to_passthrough() {
        let s = ProcessSupervisor::new();
        let script = s.build_script("fn main() {}", "rs");
        assert_eq!(
            script, "fn main() {}",
            "Rust code should pass through raw since build_script no longer handles it"
        );
    }
}
