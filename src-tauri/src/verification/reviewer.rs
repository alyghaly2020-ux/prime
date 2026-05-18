use super::{Issue, IssueSeverity, VerificationResult};

pub struct CodeReviewer;

// ── Helper types used by CodeReviewer methods ──────────────────────────

struct FuncInfo {
    name: String,
    start_line: usize,
    end_line: usize,
}

struct DuplicateBlock {
    block1_start: usize,
    block1_end: usize,
    block2_start: usize,
    block2_end: usize,
    line_count: usize,
}

impl Default for CodeReviewer {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeReviewer {
    pub fn new() -> Self {
        Self
    }

    /// Review a complete code string.
    pub fn review(&self, code: &str, language: &str) -> VerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let suggestions = Vec::new();

        // ── Generic checks (language-agnostic) ──────────────────────────

        if code.trim().is_empty() {
            errors.push(Issue {
                severity: IssueSeverity::Error,
                message: "Empty code".to_string(),
                file: None,
                line: None,
                column: None,
                code: Some("empty".to_string()),
                suggestion: Some("Provide code to review".to_string()),
            });
            return VerificationResult {
                passed: false,
                score: 0.0,
                errors,
                warnings,
                suggestions,
            };
        }

        // Comment density
        let total_lines = code.lines().count();
        let comment_lines: Vec<&str> = code
            .lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("//")
                    || t.starts_with('#')
                    || t.starts_with("/*")
                    || t.starts_with("*")
            })
            .collect();

        if total_lines > 0 && comment_lines.len() > total_lines / 2 {
            warnings.push(Issue {
                severity: IssueSeverity::Warning,
                message: "More than 50% of code is commented out".to_string(),
                file: None,
                line: None,
                column: None,
                code: Some("comment-density".to_string()),
                suggestion: Some("Remove dead commented code".to_string()),
            });
        }

        // Magic numbers
        let number_pattern = regex::Regex::new(r"\b\d{4,}\b").unwrap();
        for (i, line) in code.lines().enumerate() {
            if number_pattern.is_match(line)
                && !line.contains("const")
                && !line.contains("let")
                && !line.contains("var")
            {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: "Possible magic number".to_string(),
                    file: None,
                    line: Some(i + 1),
                    column: None,
                    code: Some("magic-number".to_string()),
                    suggestion: Some("Extract to a named constant".to_string()),
                });
            }
        }

        // ── Function length checks ──────────────────────────────────────

        let functions = Self::extract_functions(code, language);
        for func in &functions {
            let line_count = func.end_line - func.start_line + 1;
            if line_count > 100 {
                errors.push(Issue {
                    severity: IssueSeverity::Error,
                    message: format!(
                        "Function '{}' is {} lines long (> 100)",
                        func.name, line_count
                    ),
                    file: None,
                    line: Some(func.start_line),
                    column: None,
                    code: Some("func-length".to_string()),
                    suggestion: Some("Break function into smaller functions".to_string()),
                });
            } else if line_count > 50 {
                warnings.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: format!(
                        "Function '{}' is {} lines long (> 50)",
                        func.name, line_count
                    ),
                    file: None,
                    line: Some(func.start_line),
                    column: None,
                    code: Some("func-length".to_string()),
                    suggestion: Some("Consider refactoring into smaller functions".to_string()),
                });
            }
        }

        // ── Duplicate code detection ────────────────────────────────────

        let duplicates = Self::find_duplicate_blocks(code, 4); // 4+ identical lines
        for dup in &duplicates {
            warnings.push(Issue {
                severity: IssueSeverity::Warning,
                message: format!(
                    "Duplicate code block ({} lines) found at lines {}-{} and {}-{}",
                    dup.line_count,
                    dup.block1_start,
                    dup.block1_end,
                    dup.block2_start,
                    dup.block2_end
                ),
                file: None,
                line: Some(dup.block1_start),
                column: None,
                code: Some("duplicate-code".to_string()),
                suggestion: Some("Extract shared logic into a function".to_string()),
            });
        }

        // ── Error handling check ────────────────────────────────────────

        self.check_error_handling(code, language, &mut errors, &mut warnings);

        // ── Naming conventions ──────────────────────────────────────────

        self.check_naming_conventions(code, language, &mut errors, &mut warnings);

        let passed = errors.is_empty();
        VerificationResult {
            passed,
            score: if passed { 1.0 } else { 0.0 },
            errors,
            warnings,
            suggestions,
        }
    }

    /// Review a complete file by path.
    pub fn review_file(&self, path: &str) -> anyhow::Result<VerificationResult> {
        let content = std::fs::read_to_string(path)?;
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let language = match ext {
            "py" => "python",
            "rs" => "rust",
            "js" | "jsx" => "javascript",
            "ts" | "tsx" => "ts",
            _ => "unknown",
        };
        Ok(self.review(&content, language))
    }

    /// Review only changed lines between `old` and `new` code.
    /// Runs review logic specifically on the differences.
    pub fn review_diff(&self, old: &str, new: &str, language: &str) -> VerificationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let suggestions = Vec::new();

        let diff = similar::TextDiff::from_lines(old, new);
        let mut changed_lines = Vec::new();

        // Collect new/inserted lines
        for change in diff.iter_all_changes() {
            if matches!(change.tag(), similar::ChangeTag::Insert) {
                changed_lines.push(change.value().to_string());
            }
        }

        let changed_code = changed_lines.join("\n");

        // Run lint rules on the diff only
        // (Simplified – runs the full review on just the changed content)
        let full_review = self.review(&changed_code, language);

        // But also check for issues that span old+new (e.g. removed code)
        let mut deleted_line_count = 0usize;
        for change in diff.iter_all_changes() {
            if matches!(change.tag(), similar::ChangeTag::Delete) {
                deleted_line_count += change.value().lines().count();
            }
        }
        if deleted_line_count > 20 {
            warnings.push(Issue {
                severity: IssueSeverity::Warning,
                message: format!(
                    "Large deletion ({} lines) – verify it's intentional",
                    deleted_line_count
                ),
                file: None,
                line: None,
                column: None,
                code: Some("large-deletion".to_string()),
                suggestion: Some("Consider incremental deletion with review".to_string()),
            });
        }

        errors.extend(full_review.errors);
        warnings.extend(full_review.warnings);

        let passed = errors.is_empty();
        VerificationResult {
            passed,
            score: if passed { 1.0 } else { 0.0 },
            errors,
            warnings,
            suggestions,
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Extract function definitions and their line ranges.
    fn extract_functions(code: &str, language: &str) -> Vec<FuncInfo> {
        let lines: Vec<&str> = code.lines().collect();
        let mut functions = Vec::new();
        let mut i = 0;

        let def_pattern: &[&str] = match language {
            "rust" | "rs" => &["fn "],
            "python" | "py" => &["def ", "async def "],
            "javascript" | "js" | "ts" => &["function ", "async function "],
            _ => return functions,
        };

        while i < lines.len() {
            let trimmed = lines[i].trim();

            // Check if this line starts a function definition
            let is_def = def_pattern.iter().any(|p| {
                if trimmed.starts_with(p) {
                    // Avoid false positives like `if fn `
                    true
                } else {
                    false
                }
            });

            // Also check for Rust-like `fn name(` (not just "fn " prefix)
            let is_rust_fn =
                language == "rust" && trimmed.starts_with("fn ") && trimmed.contains('(');

            if is_def || is_rust_fn {
                let name = Self::extract_func_name(trimmed, language);
                let start = i;

                // Find the end of the function based on brace counting
                // or Python indentation
                let end = if language == "python" || language == "py" {
                    // Python: next line at same or lesser indentation
                    let base_indent = lines[i].len().saturating_sub(lines[i].trim_start().len());
                    let mut j = i + 1;
                    while j < lines.len() {
                        let cur_indent = lines[j].len().saturating_sub(lines[j].trim_start().len());
                        if lines[j].trim().is_empty() {
                            j += 1;
                            continue;
                        }
                        if cur_indent <= base_indent && !lines[j].trim().is_empty() {
                            break;
                        }
                        j += 1;
                    }
                    j.saturating_sub(1)
                } else {
                    // Brace-based: find matching }
                    let mut depth = 0;
                    let mut started = false;
                    let mut j = i;
                    while j < lines.len() {
                        for ch in lines[j].chars() {
                            if ch == '{' {
                                depth += 1;
                                started = true;
                            } else if ch == '}' {
                                depth -= 1;
                            }
                        }
                        if started && depth <= 0 {
                            break;
                        }
                        j += 1;
                    }
                    j.min(lines.len() - 1)
                };

                if end > start {
                    functions.push(FuncInfo {
                        name,
                        start_line: start + 1, // 1-indexed
                        end_line: end + 1,
                    });
                }
            }
            i += 1;
        }

        functions
    }

    fn extract_func_name(line: &str, language: &str) -> String {
        match language {
            "python" | "py" => {
                // def foo( or async def foo(
                let after_def = line
                    .trim()
                    .strip_prefix("async def ")
                    .or_else(|| line.trim().strip_prefix("def "))
                    .unwrap_or("");
                after_def
                    .split('(')
                    .next()
                    .unwrap_or("unknown")
                    .trim()
                    .to_string()
            }
            "rust" | "rs" => {
                let after_fn = line.trim().strip_prefix("pub ").unwrap_or(line.trim());
                let after_fn = after_fn.strip_prefix("fn ").unwrap_or("");
                // Handle generics: fn foo<T>(
                let without_generics = after_fn.split('<').next().unwrap_or(after_fn);
                without_generics
                    .split('(')
                    .next()
                    .unwrap_or("unknown")
                    .trim()
                    .to_string()
            }
            "javascript" | "js" | "ts" => {
                let after_func = line
                    .trim()
                    .strip_prefix("async function ")
                    .or_else(|| line.trim().strip_prefix("function "))
                    .or_else(|| {
                        // Arrow functions assigned to const/let: `const foo = (`
                        if line.contains("= (") || line.contains("=>") {
                            line.split('=').next().map(|s| s.trim())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("");
                after_func
                    .split('(')
                    .next()
                    .unwrap_or("unknown")
                    .trim()
                    .to_string()
            }
            _ => "unknown".to_string(),
        }
    }

    /// Simple duplicate code detection: find non-overlapping identical line
    /// blocks of at least `min_lines` consecutive lines.
    fn find_duplicate_blocks(code: &str, min_lines: usize) -> Vec<DuplicateBlock> {
        let lines: Vec<&str> = code.lines().collect();
        let mut duplicates = Vec::new();

        if lines.len() < min_lines * 2 {
            return duplicates;
        }

        for i in 0..lines.len() {
            for j in (i + min_lines)..lines.len() {
                let mut count = 0;
                while i + count < lines.len()
                    && j + count < lines.len()
                    && lines[i + count].trim() == lines[j + count].trim()
                {
                    count += 1;
                }
                if count >= min_lines {
                    // Avoid overlapping duplicates
                    let overlaps = duplicates.iter().any(|d: &DuplicateBlock| {
                        (i >= d.block1_start - 1 && i < d.block1_end)
                            || (j >= d.block2_start - 1 && j < d.block2_end)
                    });
                    if !overlaps {
                        duplicates.push(DuplicateBlock {
                            block1_start: i + 1,
                            block1_end: i + count,
                            block2_start: j + 1,
                            block2_end: j + count,
                            line_count: count,
                        });
                    }
                    // Skip past this block for outer loop
                    break;
                }
            }
        }

        duplicates
    }

    /// Check that functions which should return Result/Option actually do.
    fn check_error_handling(
        &self,
        code: &str,
        language: &str,
        _errors: &mut Vec<Issue>,
        warnings: &mut Vec<Issue>,
    ) {
        // Rust: check if a function returns `Result` or uses `?`
        for (i, line) in code.lines().enumerate() {
            if language == "rust" {
                // Functions that might need error handling
                if line.contains("fn ") && line.contains("-> ") {
                    // Check if the return type includes Result
                    let has_result = line.contains("Result<");
                    let has_question = line.contains("?");

                    // If the function body has potential error sources but no Result return
                    if !has_result && !has_question {
                        // Look ahead in function body for error-related operations
                        // This is a simple heuristic check
                        let contains_fallible = code.lines().skip(i + 1).take(30).any(|l| {
                            l.contains("std::fs::")
                                || l.contains("std::io::")
                                || l.contains("File::")
                                || l.contains("read_to_string")
                                || l.contains("unwrap_or")
                        });

                        if contains_fallible {
                            warnings.push(Issue {
                                severity: IssueSeverity::Warning,
                                message:
                                    "Function uses fallible operations but does not return Result"
                                        .to_string(),
                                file: None,
                                line: Some(i + 1),
                                column: None,
                                code: Some("error-handling".to_string()),
                                suggestion: Some(
                                    "Consider returning Result<T, E> and using ?".to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Check naming conventions per language.
    fn check_naming_conventions(
        &self,
        code: &str,
        language: &str,
        _errors: &mut Vec<Issue>,
        warnings: &mut Vec<Issue>,
    ) {
        match language {
            "rust" | "rs" => {
                // Rust: functions should be snake_case
                for (i, line) in code.lines().enumerate() {
                    let trimmed = line.trim();
                    if let Some(fn_name) = trimmed
                        .strip_prefix("fn ")
                        .or_else(|| trimmed.strip_prefix("pub fn "))
                        .and_then(|s| {
                            let name = s.split('(').next().unwrap_or("");
                            let name = name.split('<').next().unwrap_or("");
                            let name = name.split_whitespace().next().unwrap_or("");
                            if name.is_empty() {
                                None
                            } else {
                                Some(name)
                            }
                        })
                    {
                        // Check for any uppercase letters (camelCase or PascalCase)
                        if fn_name.chars().any(|c| c.is_uppercase()) {
                            warnings.push(Issue {
                                severity: IssueSeverity::Warning,
                                message: format!(
                                    "Function '{}' uses camelCase; Rust convention is snake_case",
                                    fn_name
                                ),
                                file: None,
                                line: Some(i + 1),
                                column: None,
                                code: Some("naming-convention".to_string()),
                                suggestion: Some("Use snake_case for function names".to_string()),
                            });
                        }
                    }
                }
            }

            "python" | "py" => {
                // Python: functions should be snake_case
                for (i, line) in code.lines().enumerate() {
                    let trimmed = line.trim();
                    if let Some(after_def) = trimmed
                        .strip_prefix("def ")
                        .or_else(|| trimmed.strip_prefix("async def "))
                    {
                        let fn_name = after_def.split('(').next().unwrap_or("").trim();
                        if !fn_name.is_empty()
                            && fn_name.chars().any(|c| c.is_uppercase())
                            && !fn_name.starts_with("__")
                        {
                            warnings.push(Issue {
                                severity: IssueSeverity::Warning,
                                message: format!(
                                    "Function '{}' uses CamelCase; Python convention is snake_case",
                                    fn_name
                                ),
                                file: None,
                                line: Some(i + 1),
                                column: None,
                                code: Some("naming-convention".to_string()),
                                suggestion: Some("Use snake_case for function names".to_string()),
                            });
                        }
                    }
                }
            }

            "javascript" | "js" | "ts" => {
                // JS: functions should be camelCase (not PascalCase for non-components)
                for (i, line) in code.lines().enumerate() {
                    let trimmed = line.trim();
                    if let Some(after_func) = trimmed
                        .strip_prefix("function ")
                        .or_else(|| trimmed.strip_prefix("async function "))
                    {
                        let fn_name = after_func.split('(').next().unwrap_or("").trim();
                        if !fn_name.is_empty()
                            && fn_name.chars().next().is_some_and(|c| c.is_uppercase())
                            && !fn_name.starts_with("React")
                            && !fn_name.starts_with("use")
                        {
                            warnings.push(Issue {
                                severity: IssueSeverity::Warning,
                                message: format!(
                                    "Function '{}' uses PascalCase; JS convention is camelCase",
                                    fn_name
                                ),
                                file: None,
                                line: Some(i + 1),
                                column: None,
                                code: Some("naming-convention".to_string()),
                                suggestion: Some("Use camelCase for regular functions".to_string()),
                            });
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Empty code ──────────────────────────────────────────────────────

    #[test]
    fn test_empty_code() {
        let reviewer = CodeReviewer::new();
        let result = reviewer.review("", "rust");
        assert!(!result.passed);
    }

    // ── Function length ─────────────────────────────────────────────────

    #[test]
    fn test_function_length_warning() {
        let code = "fn long_function() {\n".to_string()
            + &(0..55)
                .map(|i| format!("    let x_{} = {};\n", i, i))
                .collect::<String>()
            + "}";
        let reviewer = CodeReviewer::new();
        let result = reviewer.review(&code, "rust");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("func-length".to_string())));
    }

    // ── Duplicate code ──────────────────────────────────────────────────

    #[test]
    fn test_duplicate_code_detection() {
        let code = "\
fn foo() {
    let x = 1;
    let y = 2;
    let z = 3;
    println!(\"hello\");
}

fn bar() {
    let x = 1;
    let y = 2;
    let z = 3;
    println!(\"hello\");
}
";
        let reviewer = CodeReviewer::new();
        let result = reviewer.review(code, "rust");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("duplicate-code".to_string())));
    }

    // ── Naming conventions ──────────────────────────────────────────────

    #[test]
    fn test_snake_case_violation_rust() {
        let reviewer = CodeReviewer::new();
        let result = reviewer.review("pub fn CamelCaseFunc() {}", "rust");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("naming-convention".to_string())));
    }

    #[test]
    fn test_snake_case_violation_python() {
        let reviewer = CodeReviewer::new();
        let result = reviewer.review("def CamelCaseFunc(): pass", "python");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("naming-convention".to_string())));
    }

    #[test]
    fn test_camel_case_violation_js() {
        let reviewer = CodeReviewer::new();
        let result = reviewer.review("function PascalCaseFunc() {}", "javascript");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("naming-convention".to_string())));
    }

    // ── review_file / review_diff ───────────────────────────────────────

    #[test]
    fn test_review_diff_large_deletion() {
        let old_code = (0..30).map(|i| format!("line {}\n", i)).collect::<String>();
        let new_code = "";
        let reviewer = CodeReviewer::new();
        let result = reviewer.review_diff(&old_code, new_code, "rust");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == Some("large-deletion".to_string())));
    }
}
