use super::{Issue, IssueSeverity};
use std::sync::atomic::{AtomicU64, Ordering};

/// Statistics tracked across self-heal attempts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealStats {
    pub total_attempted: u64,
    pub total_fixed: u64,
    pub common_errors: Vec<(String, u64)>,
}

pub struct SelfHealingLoop {
    max_iterations: u32,
    // Statistics
    total_attempted: AtomicU64,
    total_fixed: AtomicU64,
    error_counts: parking_lot::Mutex<std::collections::HashMap<String, u64>>,
}

impl Default for SelfHealingLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfHealingLoop {
    pub fn new() -> Self {
        Self {
            max_iterations: 5,
            total_attempted: AtomicU64::new(0),
            total_fixed: AtomicU64::new(0),
            error_counts: parking_lot::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Iteratively apply `fixer` to the code for each error issue, up to
    /// `max_iterations` rounds.
    pub async fn heal<F>(&self, code: &str, issues: &[Issue], fixer: F) -> String
    where
        F: Fn(&str, &Issue) -> Option<String>,
    {
        let mut current = code.to_string();

        for _ in 0..self.max_iterations {
            let mut fixed = false;

            for issue in issues {
                if matches!(issue.severity, IssueSeverity::Error) {
                    self.total_attempted.fetch_add(1, Ordering::Relaxed);
                    if let Some(fixed_code) = fixer(&current, issue) {
                        self.total_fixed.fetch_add(1, Ordering::Relaxed);
                        current = fixed_code;
                        fixed = true;
                        tracing::info!("Self-healed: {}", issue.message);
                    }
                }
            }

            if !fixed {
                break;
            }
        }

        current
    }

    /// Generate a suggested fix string for a given error issue.
    /// This produces a human-readable fix description, optionally with
    /// replacement code.
    pub fn generate_fix(&self, error: &Issue) -> Option<String> {
        error.suggestion.as_ref().map(|suggestion| format!(
            "Fix for {} (line {}): {}",
            error.code.as_deref().unwrap_or("issue"),
            error.line.map_or_else(|| "?".to_string(), |l| l.to_string()),
            suggestion
        ))
    }

    /// Auto-fix simple, well-known issues in code without needing a callback.
    /// Currently supports:
    /// - Missing semicolons in Rust/JavaScript
    /// - `print ` without parentheses in Python
    /// - Trailing whitespace
    pub fn auto_fix_simple(&self, code: &str, language: &str) -> String {
        let mut result = code.to_string();

        match language {
            "rust" | "rs" => {
                // Add missing semicolons to non-block, non-control-flow statements
                result = Self::fix_missing_semicolons_rust(&result);
            }
            "javascript" | "js" => {
                result = Self::fix_missing_semicolons_js(&result);
            }
            "python" | "py" => {
                result = Self::fix_print_without_parens(&result);
            }
            _ => {}
        }

        // Remove trailing whitespace from each line
        result = result
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<&str>>()
            .join("\n");

        result
    }

    /// Return a snapshot of self-heal statistics.
    pub fn get_heal_stats(&self) -> HealStats {
        let counts = self.error_counts.lock();
        let mut common: Vec<(String, u64)> = counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        common.sort_by_key(|b| std::cmp::Reverse(b.1));
        common.truncate(10); // top 10

        HealStats {
            total_attempted: self.total_attempted.load(Ordering::Relaxed),
            total_fixed: self.total_fixed.load(Ordering::Relaxed),
            common_errors: common,
        }
    }

    // ── Internal fix helpers ────────────────────────────────────────────

    fn fix_missing_semicolons_rust(code: &str) -> String {
        let mut result = String::new();
        // Statements that should NOT get a semicolon
        let no_semi = [
            "fn ", "pub ", "struct ", "enum ", "trait ", "impl ", "if ", "else", "for ", "while ",
            "loop ", "match ", "const ", "static ", "use ", "mod ", "pub(", "#[", "//", "/*", "*/",
            "}", "{", "type ", "where ",
        ];

        for line in code.lines() {
            let trimmed = line.trim();
            let needs_semi = !trimmed.is_empty()
                && !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
                && !trimmed.ends_with('}')
                && !trimmed.ends_with("=>")
                && !trimmed.ends_with("->")
                && !trimmed.ends_with(',')
                && !no_semi.iter().any(|p| trimmed.starts_with(p));

            if needs_semi {
                result.push_str(line);
                result.push_str(";\n");
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }

    fn fix_missing_semicolons_js(code: &str) -> String {
        let mut result = String::new();
        let no_semi = [
            "function ",
            "if ",
            "else",
            "for ",
            "while ",
            "switch ",
            "try ",
            "catch ",
            "finally ",
            "class ",
            "import ",
            "export ",
            "const ",
            "let ",
            "var ",
            "//",
            "/*",
            "#[",
            "}",
            "{",
        ];

        for line in code.lines() {
            let trimmed = line.trim();
            let needs_semi = !trimmed.is_empty()
                && !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
                && !trimmed.ends_with('}')
                && !trimmed.ends_with("=>")
                && !trimmed.ends_with(',')
                && !trimmed.ends_with('(')
                && !trimmed.ends_with(')')
                && !no_semi.iter().any(|p| trimmed.starts_with(p));

            if needs_semi {
                result.push_str(line);
                result.push_str(";\n");
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }

    fn fix_print_without_parens(code: &str) -> String {
        code.lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("print ") && !trimmed.contains('(') {
                    let content = trimmed.strip_prefix("print ").unwrap_or("");
                    format!(
                        "{}{}({})",
                        &line[..line.len() - trimmed.len()],
                        "print",
                        content
                    )
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_fix_missing_semicolon_rust() {
        let loop_ = SelfHealingLoop::new();
        let fixed = loop_.auto_fix_simple("fn main() {\n    let x = 5\n}", "rust");
        assert!(
            fixed.contains("let x = 5;"),
            "Missing semicolon should be added"
        );
    }

    #[test]
    fn test_auto_fix_print_without_parens_python() {
        let loop_ = SelfHealingLoop::new();
        let fixed = loop_.auto_fix_simple("print hello", "python");
        assert_eq!(fixed.trim(), "print(hello)");
    }

    #[test]
    fn test_generate_fix() {
        let loop_ = SelfHealingLoop::new();
        let issue = Issue {
            severity: IssueSeverity::Error,
            message: "Test error".to_string(),
            file: None,
            line: Some(10),
            column: None,
            code: Some("test-code".to_string()),
            suggestion: Some("Do X instead".to_string()),
        };
        let fix = loop_.generate_fix(&issue);
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("Do X instead"));
    }

    #[test]
    fn test_get_heal_stats() {
        let loop_ = SelfHealingLoop::new();
        let stats = loop_.get_heal_stats();
        assert_eq!(stats.total_attempted, 0);
        assert_eq!(stats.total_fixed, 0);
    }
}
