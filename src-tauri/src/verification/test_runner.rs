use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Detailed information about a single test case.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TestDetail {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u64,
}

/// Overall test result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TestResult {
    pub passed: bool,
    pub failures: Option<Vec<String>>,
    pub total: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub duration_ms: u64,
    pub test_details: Vec<TestDetail>,
    pub coverage_pct: Option<f64>,
}

pub struct TestRunner;

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRunner {
    pub fn new() -> Self {
        Self
    }

    /// Run tests for the given language.
    pub async fn run_tests(&self, code: &str, language: &str) -> TestResult {
        match language {
            "python" | "py" => self.run_python_tests(code).await,
            "rust" | "rs" => self.run_rust_tests(code).await,
            "javascript" | "js" | "ts" => self.run_js_tests(code).await,
            _ => TestResult {
                passed: true,
                failures: None,
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: 0,
                test_details: Vec::new(),
                coverage_pct: None,
            },
        }
    }

    // ── Python tests ────────────────────────────────────────────────────

    async fn run_python_tests(&self, code: &str) -> TestResult {
        let start = std::time::Instant::now();

        // Quick detection: no test framework usage → skip
        let has_tests = code.contains("def test")
            || code.contains("unittest")
            || code.contains("pytest")
            || code.contains("import pytest");
        if !has_tests {
            return TestResult {
                passed: true,
                failures: None,
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                test_details: Vec::new(),
                coverage_pct: None,
            };
        }

        // Write code to a temp file and run pytest
        let tmp_dir = std::env::temp_dir().join(format!("prime_test_py_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp_dir);
        let test_file = tmp_dir.join("test_code.py");
        std::fs::write(&test_file, code).ok();

        // Detect pytest marker (conftest.py, pytest.ini etc.) – otherwise
        // pytest will skip a file without `test_` prefix.  Use a proper name.
        let renamed = tmp_dir.join("test_snippet.py");
        let _ = std::fs::copy(&test_file, &renamed);

        let cmd = if cfg!(target_os = "windows") {
            "python"
        } else {
            "python3"
        };

        let output = timeout(
            Duration::from_secs(60),
            Command::new(cmd)
                .args([
                    "-m",
                    "pytest",
                    &renamed.to_string_lossy(),
                    "-v",
                    "--tb=short",
                ])
                .current_dir(&tmp_dir)
                .output(),
        )
        .await;

        let _ = std::fs::remove_dir_all(&tmp_dir);
        let elapsed = start.elapsed().as_millis() as u64;

        Self::parse_pytest_output(output, elapsed)
    }

    fn parse_pytest_output(
        result: Result<Result<std::process::Output, std::io::Error>, tokio::time::error::Elapsed>,
        duration_ms: u64,
    ) -> TestResult {
        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                Self::parse_test_output(&combined, "python", duration_ms)
            }
            Ok(Err(e)) => TestResult {
                passed: false,
                failures: Some(vec![format!("Execution error: {}", e)]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms,
                test_details: Vec::new(),
                coverage_pct: None,
            },
            Err(_) => TestResult {
                passed: false,
                failures: Some(vec!["Timeout (60s)".to_string()]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms,
                test_details: Vec::new(),
                coverage_pct: None,
            },
        }
    }

    // ── Rust tests ──────────────────────────────────────────────────────

    async fn run_rust_tests(&self, _code: &str) -> TestResult {
        let start = std::time::Instant::now();

        // Find the workspace Cargo.toml by searching upward from the current dir
        let workspace = Self::find_cargo_workspace()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let output = timeout(
            Duration::from_secs(120),
            Command::new("cargo")
                .args(["test", "--no-fail-fast"])
                .current_dir(&workspace)
                .output(),
        )
        .await;

        let elapsed = start.elapsed().as_millis() as u64;

        match output {
            Ok(Ok(out)) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                Self::parse_test_output(&combined, "rust", elapsed)
            }
            Ok(Err(e)) => TestResult {
                passed: false,
                failures: Some(vec![format!("Execution error: {}", e)]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: elapsed,
                test_details: Vec::new(),
                coverage_pct: None,
            },
            Err(_) => TestResult {
                passed: false,
                failures: Some(vec!["Timeout (120s)".to_string()]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: elapsed,
                test_details: Vec::new(),
                coverage_pct: None,
            },
        }
    }

    /// Walk up from the current directory looking for a Cargo.toml.
    fn find_cargo_workspace() -> Option<std::path::PathBuf> {
        let mut dir = std::env::current_dir().ok()?;
        loop {
            if dir.join("Cargo.toml").exists() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    // ── JavaScript / TypeScript tests ───────────────────────────────────

    async fn run_js_tests(&self, _code: &str) -> TestResult {
        let start = std::time::Instant::now();

        let cwd = std::env::current_dir().unwrap_or_default();
        let has_vitest = cwd.join("vitest.config.ts").exists()
            || cwd.join("vitest.config.js").exists()
            || cwd.join("vite.config.ts").exists();

        let has_jest = cwd.join("jest.config.ts").exists()
            || cwd.join("jest.config.js").exists()
            || Self::check_package_json_dep(&cwd.join("package.json"), "jest");

        let has_package_json = cwd.join("package.json").exists();

        // Try vitest first, then jest, then npm test
        let output = if has_vitest {
            timeout(
                Duration::from_secs(60),
                Command::new("npx")
                    .args(["vitest", "run"])
                    .current_dir(&cwd)
                    .output(),
            )
            .await
        } else if has_jest {
            timeout(
                Duration::from_secs(60),
                Command::new("npx")
                    .args(["jest"])
                    .current_dir(&cwd)
                    .output(),
            )
            .await
        } else if has_package_json {
            timeout(
                Duration::from_secs(60),
                Command::new("npm")
                    .args(["test"])
                    .current_dir(&cwd)
                    .output(),
            )
            .await
        } else {
            return TestResult {
                passed: true,
                failures: None,
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                test_details: Vec::new(),
                coverage_pct: None,
            };
        };

        let elapsed = start.elapsed().as_millis() as u64;

        match output {
            Ok(Ok(out)) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                Self::parse_test_output(&combined, "javascript", elapsed)
            }
            Ok(Err(e)) => TestResult {
                passed: false,
                failures: Some(vec![format!("Execution error: {}", e)]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: elapsed,
                test_details: Vec::new(),
                coverage_pct: None,
            },
            Err(_) => TestResult {
                passed: false,
                failures: Some(vec!["Timeout (60s)".to_string()]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: elapsed,
                test_details: Vec::new(),
                coverage_pct: None,
            },
        }
    }

    fn check_package_json_dep(path: &std::path::Path, dep: &str) -> bool {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return false,
        };
        json.get("devDependencies")
            .and_then(|v| v.as_object())
            .map(|obj| obj.contains_key(dep))
            .unwrap_or(false)
    }

    // ── Unified output parser ───────────────────────────────────────────

    /// Parse test output and extract test names, status, and duration.
    /// Supports Rust (`cargo test`), JavaScript (`vitest` / `jest`) and
    /// Python (`pytest`) output formats.
    pub fn parse_test_output(output: &str, language: &str, duration_ms: u64) -> TestResult {
        use regex::Regex;

        let mut test_details = Vec::new();
        let mut failures = Vec::new();
        let mut passed_count = 0;
        let mut failed_count = 0;
        let mut total_from_summary: Option<usize> = None;
        let mut passed_from_summary: Option<usize> = None;

        match language {
            "rust" | "rs" => {
                // Rust: `test test_name ... ok` or `test test_name ... FAILED`
                let re_test = Regex::new(r"^test\s+(\S+)\s+\.\.\.\s+(ok|FAILED)").unwrap();
                // Summary: `test result: ok. 7 passed; 0 failed; ...`
                let re_summary =
                    Regex::new(r"test result:\s+(ok|FAILED)\.\s*(\d+)\s+passed;\s*(\d+)\s+failed")
                        .unwrap();

                for line in output.lines() {
                    if let Some(caps) = re_test.captures(line) {
                        let name = caps[1].to_string();
                        let passed = &caps[2] == "ok";
                        if passed {
                            passed_count += 1;
                        } else {
                            failed_count += 1;
                            failures.push(format!("{} ({})", line, name));
                        }
                        test_details.push(TestDetail {
                            name,
                            passed,
                            duration_ms: 0,
                        });
                    }
                    if let Some(caps) = re_summary.captures(line) {
                        passed_from_summary = caps[2].parse().ok();
                        total_from_summary = Some(
                            passed_from_summary.unwrap_or(0)
                                + caps[3].parse::<usize>().unwrap_or(0),
                        );
                    }
                }
            }

            "javascript" | "js" | "ts" => {
                // Vitest style: ` ✓ test_name` or ` × test_name`
                let re_vitest = Regex::new(r"^\s*(✓|×|✗|✘)\s+(.+)").unwrap();
                // Summary: `Tests  \d+ passed | \d+ failed | \d+ total`
                let _re_summary =
                    Regex::new(r"Tests\s+(?:(\d+)\s+passed)?.*?(?:(\d+)\s+failed)?").unwrap();
                // Jest summary: `Tests:       \d+ passed, \d+ total`
                let re_jest_summary =
                    Regex::new(r"Tests:\s+(\d+)\s+passed,\s*(\d+)\s+total").unwrap();

                for line in output.lines() {
                    if let Some(caps) = re_vitest.captures(line) {
                        let name = caps[2].trim().to_string();
                        let passed = caps[1].contains('✓');
                        if passed {
                            passed_count += 1;
                        } else {
                            failed_count += 1;
                            failures.push(name.clone());
                        }
                        test_details.push(TestDetail {
                            name,
                            passed,
                            duration_ms: 0,
                        });
                    }
                    if let Some(caps) = re_jest_summary.captures(line) {
                        passed_from_summary = caps[1].parse().ok();
                        total_from_summary = caps[2].parse().ok();
                    }
                }
            }

            "python" | "py" => {
                // pytest: `test_snippet.py::test_name PASSED` or `FAILED`
                let re_pytest = Regex::new(r"::(\S+)\s+(PASSED|FAILED|ERROR|SKIPPED)").unwrap();
                // Pytest summary: `== \d+ passed in ... ==`
                let re_summary = Regex::new(r"===\s*(\d+)\s+passed").unwrap();

                for line in output.lines() {
                    if let Some(caps) = re_pytest.captures(line) {
                        let name = caps[1].to_string();
                        let passed = &caps[2] == "PASSED" || &caps[2] == "SKIPPED";
                        if passed {
                            passed_count += 1;
                        } else {
                            failed_count += 1;
                            failures.push(format!("{} ({})", name, line));
                        }
                        test_details.push(TestDetail {
                            name,
                            passed,
                            duration_ms: 0,
                        });
                    }
                    if let Some(caps) = re_summary.captures(line) {
                        passed_from_summary = caps[1].parse().ok();
                    }
                }
            }

            _ => {}
        }

        // Fall back to parsed counts; prefer summary line counts
        let total = total_from_summary.unwrap_or(test_details.len());
        let passed = passed_from_summary.unwrap_or(passed_count);

        TestResult {
            passed: failures.is_empty(),
            failures: if failures.is_empty() {
                None
            } else {
                Some(failures)
            },
            total,
            passed_count: passed,
            failed_count,
            duration_ms,
            test_details,
            coverage_pct: None,
        }
    }

    /// Attempt to estimate line coverage from test output.
    /// This looks for coverage reports embedded in the output (e.g.
    /// `cargo tarpaulin`, `pytest-cov`, `vitest --coverage`).
    pub fn get_test_coverage(output: &str) -> Option<f64> {
        use regex::Regex;

        // cargo-tarpaulin: `Coverage: xx.x%`
        let re_tarpaulin = Regex::new(r"Coverage:\s+(\d+\.?\d*)%").ok()?;
        if let Some(caps) = re_tarpaulin.captures(output) {
            return caps[1].parse::<f64>().ok();
        }

        // pytest-cov: `TOTAL\s+\d+\s+\d+\s+(\d+)%`
        let re_pytest_cov = Regex::new(r"TOTAL\s+\d+\s+\d+\s+(\d+)%").ok()?;
        if let Some(caps) = re_pytest_cov.captures(output) {
            return caps[1].parse::<f64>().ok();
        }

        // vitest --coverage: `Statements  : xx.xx%`
        let re_vitest = Regex::new(r"Statements\s*:\s*(\d+\.?\d*)%").ok()?;
        if let Some(caps) = re_vitest.captures(output) {
            return caps[1].parse::<f64>().ok();
        }

        None
    }

    /// Run specific tests by name.  Delegates to the appropriate test runner
    /// with a filter.
    pub async fn run_selected_tests(&self, test_names: &[&str], language: &str) -> TestResult {
        if test_names.is_empty() {
            return TestResult {
                passed: true,
                failures: None,
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: 0,
                test_details: Vec::new(),
                coverage_pct: None,
            };
        }

        let start = std::time::Instant::now();

        let result = match language {
            "rust" | "rs" => {
                let workspace = Self::find_cargo_workspace()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                let mut args = vec!["test".to_string(), "--no-fail-fast".to_string()];
                args.extend(test_names.iter().map(|n| n.to_string()));

                timeout(
                    Duration::from_secs(120),
                    Command::new("cargo")
                        .args(&args)
                        .current_dir(&workspace)
                        .output(),
                )
                .await
            }
            "javascript" | "js" | "ts" => {
                let cwd = std::env::current_dir().unwrap_or_default();
                let args: Vec<String> = test_names
                    .iter()
                    .flat_map(|n| vec!["-t".to_string(), n.to_string()])
                    .collect();

                let mut cmd_args = vec!["vitest".to_string(), "run".to_string()];
                cmd_args.extend(args);

                timeout(
                    Duration::from_secs(60),
                    Command::new("npx")
                        .args(&cmd_args)
                        .current_dir(&cwd)
                        .output(),
                )
                .await
            }
            "python" | "py" => {
                let tmp_dir =
                    std::env::temp_dir().join(format!("prime_test_py_{}", std::process::id()));
                let _ = std::fs::create_dir_all(&tmp_dir);
                let renamed = tmp_dir.join("test_snippet.py");
                std::fs::write(&renamed, "").ok();

                let filter: Vec<String> = test_names.iter().map(|n| format!("-k {}", n)).collect();
                let cmd = if cfg!(target_os = "windows") {
                    "python"
                } else {
                    "python3"
                };
                let mut args = vec![
                    "-m".to_string(),
                    "pytest".to_string(),
                    renamed.to_string_lossy().to_string(),
                    "-v".to_string(),
                    "--tb=short".to_string(),
                ];
                args.extend(filter);

                let result = timeout(
                    Duration::from_secs(60),
                    Command::new(cmd).args(&args).current_dir(&tmp_dir).output(),
                )
                .await;

                let _ = std::fs::remove_dir_all(&tmp_dir);
                result
            }
            _ => {
                return TestResult {
                    passed: true,
                    failures: None,
                    total: 0,
                    passed_count: 0,
                    failed_count: 0,
                    duration_ms: 0,
                    test_details: Vec::new(),
                    coverage_pct: None,
                };
            }
        };

        let elapsed = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(out)) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                Self::parse_test_output(&combined, language, elapsed)
            }
            Ok(Err(e)) => TestResult {
                passed: false,
                failures: Some(vec![format!("Execution error: {}", e)]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: elapsed,
                test_details: Vec::new(),
                coverage_pct: None,
            },
            Err(_) => TestResult {
                passed: false,
                failures: Some(vec!["Timeout".to_string()]),
                total: 0,
                passed_count: 0,
                failed_count: 0,
                duration_ms: elapsed,
                test_details: Vec::new(),
                coverage_pct: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_test_output tests ─────────────────────────────────────────

    #[test]
    fn test_parse_rust_ok() {
        let output = "\
running 1 test
test my_test ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";
        let result = TestRunner::parse_test_output(output, "rust", 100);
        assert!(result.passed);
        assert_eq!(result.total, 1);
        assert_eq!(result.passed_count, 1);
        assert_eq!(result.test_details.len(), 1);
        assert_eq!(result.test_details[0].name, "my_test");
    }

    #[test]
    fn test_parse_rust_failure() {
        let output = "\
running 1 test
test failing_test ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

failures:
---- failing_test stdout ----
assertion failed
";
        let result = TestRunner::parse_test_output(output, "rust", 100);
        assert!(!result.passed);
        assert_eq!(result.failed_count, 1);
        assert_eq!(result.failures.as_ref().map(|f| f.len()), Some(1));
    }

    #[test]
    fn test_parse_vitest_ok() {
        let output = "\n ✓ adds 1 + 2 = 3\n ✓ subtracts 2 - 1 = 1\n\n Test Files 1 passed (1)\n Tests  2 passed (2)\n";
        let result = TestRunner::parse_test_output(output, "javascript", 50);
        assert!(result.passed);
        assert_eq!(result.test_details.len(), 2);
    }

    #[test]
    fn test_parse_pytest_ok() {
        let output = "\
test_snippet.py::test_add PASSED
test_snippet.py::test_sub PASSED
=== 2 passed in 0.10s ===
";
        let result = TestRunner::parse_test_output(output, "python", 100);
        assert!(result.passed);
        assert_eq!(result.test_details.len(), 2);
        assert_eq!(result.passed_count, 2);
    }

    // ── get_test_coverage tests ─────────────────────────────────────────

    #[test]
    fn test_coverage_tarpaulin() {
        let output = "Coverage: 87.5%";
        assert!((TestRunner::get_test_coverage(output).unwrap() - 87.5).abs() < 0.01);
    }

    #[test]
    fn test_coverage_pytest_cov() {
        let output = "TOTAL                               123     45     63%";
        assert!((TestRunner::get_test_coverage(output).unwrap() - 63.0).abs() < 0.01);
    }

    #[test]
    fn test_coverage_vitest() {
        let output = "Statements  : 92.31%";
        assert!((TestRunner::get_test_coverage(output).unwrap() - 92.31).abs() < 0.01);
    }

    #[test]
    fn test_coverage_none() {
        assert!(TestRunner::get_test_coverage("no coverage here").is_none());
    }
}
