use super::{Issue, IssueSeverity};

pub struct LintResult {
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
    pub suggestions: Vec<String>,
}

pub struct LintEngine;

impl Default for LintEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LintEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn lint(&self, code: &str, language: &str) -> LintResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        match language {
            "python" | "py" => self.lint_python(code, &mut errors, &mut warnings, &mut suggestions),
            "javascript" | "js" | "ts" => {
                self.lint_javascript(code, &mut errors, &mut warnings, &mut suggestions)
            }
            "rust" | "rs" => self.lint_rust(code, &mut errors, &mut warnings, &mut suggestions),
            _ => {}
        }

        LintResult {
            errors,
            warnings,
            suggestions,
        }
    }

    // ── Rust lints ──────────────────────────────────────────────────────

    fn lint_rust(
        &self,
        code: &str,
        errors: &mut Vec<Issue>,
        warnings: &mut Vec<Issue>,
        suggestions: &mut Vec<String>,
    ) {
        for (i, line) in code.lines().enumerate() {
            // Line length (same as before)
            if line.len() > 100 {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Line too long (> 100 characters)".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("rust-101".to_string()),
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }

            // unsafe blocks – Error
            if line.contains("unsafe {") || line.trim().starts_with("unsafe ") {
                errors.push(Issue {
                    severity: IssueSeverity::Error,
                    message: "Unsafe block detected".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("rust-unsafe".to_string()),
                    suggestion: Some(
                        "Avoid unsafe code; use safe abstractions instead".to_string(),
                    ),
                });
            }

            // .unwrap() calls – Warning
            if line.contains(".unwrap()") && !line.trim().starts_with("//") {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Uses .unwrap() which may panic".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("rust-unwrap".to_string()),
                    suggestion: Some("Use pattern matching or ? operator instead".to_string()),
                });
            }

            // TODO / FIXME – Warning
            if line.contains("TODO") || line.contains("FIXME") {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Contains TODO or FIXME comment".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("rust-todo".to_string()),
                    suggestion: Some("Complete the implementation before committing".to_string()),
                });
            }
        }

        if !code.contains("fn main") && code.contains("use ") {
            suggestions.push("Consider adding a main function stub".to_string());
        }

        if code.contains("dbg!") {
            warnings.push(Issue {
                severity: IssueSeverity::Warning,
                message: "dbg! macro used – likely debugging leftover".to_string(),
                file: None,
                line: None,
                column: None,
                code: Some("rust-dbg".to_string()),
                suggestion: Some("Remove dbg! calls before committing".to_string()),
            });
        }
    }

    // ── Python lints ────────────────────────────────────────────────────

    fn lint_python(
        &self,
        code: &str,
        errors: &mut Vec<Issue>,
        warnings: &mut Vec<Issue>,
        _suggestions: &mut Vec<String>,
    ) {
        for (i, line) in code.lines().enumerate() {
            // Line length
            if line.len() > 100 {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Line too long (> 100 characters)".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("E501".to_string()),
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }

            // Tabs
            if line.contains('\t') {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Tab found, use spaces instead".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("W191".to_string()),
                    suggestion: Some("Replace tabs with 4 spaces".to_string()),
                });
            }

            // Print without parentheses (Python 2 style)
            if line.trim().starts_with("print ") && !line.contains('(') {
                errors.push(Issue {
                    severity: IssueSeverity::Error,
                    message: "Print statement without parentheses".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("E1601".to_string()),
                    suggestion: Some("Use print() function instead".to_string()),
                });
            }

            // Bare except: – Warning
            if line.trim().starts_with("except:") {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Bare except clause catches all exceptions".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("E0702".to_string()),
                    suggestion: Some(
                        "Use 'except Exception:' to avoid catching KeyboardInterrupt etc."
                            .to_string(),
                    ),
                });
            }

            // Wildcard import – Warning
            if line.contains("from ") && line.contains("import *") {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Wildcard import (import *) pollutes namespace".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("W0401".to_string()),
                    suggestion: Some("Import specific names instead".to_string()),
                });
            }

            // Mutable default argument – Error
            if line.contains("def ") && line.contains("=[]")
                || line.contains("={}")
                || line.contains("=set()")
            {
                warnings.push(Issue {
                    severity: IssueSeverity::Error,
                    message: "Mutable default argument detected".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("W0102".to_string()),
                    suggestion: Some(
                        "Use None as default and initialize inside the function".to_string(),
                    ),
                });
            }
        }
    }

    // ── JavaScript / TypeScript lints ───────────────────────────────────

    fn lint_javascript(
        &self,
        code: &str,
        errors: &mut Vec<Issue>,
        warnings: &mut Vec<Issue>,
        _suggestions: &mut Vec<String>,
    ) {
        for (i, line) in code.lines().enumerate() {
            // Line length
            if line.len() > 120 {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Line too long (> 120 characters)".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("max-len".to_string()),
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }

            // 'var' usage
            if line.contains("var ") {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "'var' used instead of 'let' or 'const'".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("no-var".to_string()),
                    suggestion: Some(
                        "Use 'let' for mutable or 'const' for immutable variables".to_string(),
                    ),
                });
            }

            // == vs === – Warning
            if line.contains("==")
                && !line.contains("===")
                && !line.contains("!==")
                && !line.contains("=>")
            {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Uses loose equality (==) instead of strict equality (===)"
                        .to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("eqeqeq".to_string()),
                    suggestion: Some("Use === for strict equality comparison".to_string()),
                });
            }

            // console.log – Warning
            if line.contains("console.log") && !line.trim().starts_with("//") {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "console.log() left in code".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("no-console".to_string()),
                    suggestion: Some("Remove or replace with proper logging".to_string()),
                });
            }

            // debugger statement – Error
            if line.trim() == "debugger" || line.trim().starts_with("debugger;") {
                errors.push(Issue {
                    severity: IssueSeverity::Error,
                    message: "debugger statement present".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("no-debugger".to_string()),
                    suggestion: Some("Remove debugger statement before committing".to_string()),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Rust lint tests ─────────────────────────────────────────────────

    #[test]
    fn test_lint_rust_unsafe() {
        let engine = LintEngine::new();
        let result = engine.lint("fn main() { unsafe { let x = 1; } }", "rust");
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == Some("rust-unsafe".to_string())));
    }

    #[test]
    fn test_lint_rust_unwrap() {
        let engine = LintEngine::new();
        let result = engine.lint("fn main() { let x = foo().unwrap(); }", "rust");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("rust-unwrap".to_string())));
    }

    #[test]
    fn test_lint_rust_todo() {
        let engine = LintEngine::new();
        let result = engine.lint("// TODO: implement this", "rust");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("rust-todo".to_string())));
    }

    #[test]
    fn test_lint_rust_dbg() {
        let engine = LintEngine::new();
        let result = engine.lint("fn main() { dbg!(x); }", "rust");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("rust-dbg".to_string())));
    }

    // ── Python lint tests ───────────────────────────────────────────────

    #[test]
    fn test_lint_python_bare_except() {
        let engine = LintEngine::new();
        let result = engine.lint("try:\n    pass\nexcept:\n    pass", "python");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("E0702".to_string())));
    }

    #[test]
    fn test_lint_python_wildcard_import() {
        let engine = LintEngine::new();
        let result = engine.lint("from os import *", "python");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("W0401".to_string())));
    }

    #[test]
    fn test_lint_python_mutable_default() {
        let engine = LintEngine::new();
        let result = engine.lint("def foo(x=[]): pass", "python");
        // Check by message content since the detection regex may match
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.message.contains("Mutable default"))
                || result
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Mutable default"))
        );
    }

    // ── JavaScript lint tests ───────────────────────────────────────────

    #[test]
    fn test_lint_javascript_loose_equals() {
        let engine = LintEngine::new();
        let result = engine.lint("if (x == '5') {}", "javascript");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("eqeqeq".to_string())));
    }

    #[test]
    fn test_lint_javascript_no_var() {
        let engine = LintEngine::new();
        let result = engine.lint("var x = 1;", "javascript");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("no-var".to_string())));
    }

    #[test]
    fn test_lint_javascript_console_log() {
        let engine = LintEngine::new();
        let result = engine.lint("console.log('test');", "javascript");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("no-console".to_string())));
    }

    #[test]
    fn test_lint_javascript_debugger() {
        let engine = LintEngine::new();
        let result = engine.lint("debugger;", "javascript");
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == Some("no-debugger".to_string())));
    }
}
