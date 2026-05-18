use std::collections::HashMap;

type ErrorPattern = Vec<(String, Vec<String>, Vec<String>, Option<String>)>;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorAnalysis {
    pub error_type: String,
    pub message: String,
    pub possible_causes: Vec<String>,
    pub suggestions: Vec<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// A suggested fix string, generated from known patterns.
    pub suggested_fix: Option<String>,
}

pub struct ErrorAnalyzer {
    /// Map of language -> list of (error-substring, causes, suggestions, suggested-fix)
    patterns: HashMap<String, ErrorPattern>,
}

impl Default for ErrorAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorAnalyzer {
    pub fn new() -> Self {
        let mut patterns: HashMap<String, ErrorPattern> =
            HashMap::new();

        // ── Python ──────────────────────────────────────────────────────
        patterns.insert(
            "python".to_string(),
            vec![
                (
                    "SyntaxError".to_string(),
                    vec!["Missing parenthesis".to_string(), "Invalid indentation".to_string()],
                    vec![
                        "Check line endings and parentheses".to_string(),
                        "Verify proper indentation (4 spaces)".to_string(),
                    ],
                    Some("Fix indentation or add missing closing parenthesis/token".to_string()),
                ),
                (
                    "NameError".to_string(),
                    vec!["Variable not defined".to_string(), "Typo in variable name".to_string()],
                    vec![
                        "Check variable spelling".to_string(),
                        "Verify variable is defined before use".to_string(),
                    ],
                    Some("Check the variable name for typos or import the missing module".to_string()),
                ),
                (
                    "TypeError".to_string(),
                    vec!["Wrong type used".to_string(), "Incompatible operation".to_string()],
                    vec![
                        "Use type() to check variable types".to_string(),
                        "Add type hints for clarity".to_string(),
                    ],
                    Some("Ensure the operand types are compatible; use str()/int()/float() to convert if needed"
                        .to_string()),
                ),
                (
                    "IndentationError".to_string(),
                    vec![
                        "Mixed tabs and spaces".to_string(),
                        "Inconsistent indentation level".to_string(),
                    ],
                    vec![
                        "Use 4 spaces consistently".to_string(),
                        "Check for mixed tabs in the file".to_string(),
                    ],
                    Some("Replace all tabs with spaces and ensure consistent indentation depth".to_string()),
                ),
                (
                    "ModuleNotFoundError".to_string(),
                    vec![
                        "Module not installed".to_string(),
                        "Import path incorrect".to_string(),
                    ],
                    vec![
                        "Run pip install <module>".to_string(),
                        "Check the module name spelling".to_string(),
                    ],
                    Some("Install the missing module with: pip install <module_name>".to_string()),
                ),
            ],
        );

        // ── Rust ────────────────────────────────────────────────────────
        patterns.insert(
            "rust".to_string(),
            vec![
                (
                    "borrow of moved value".to_string(),
                    vec!["Ownership violation".to_string(), "Value moved without clone".to_string()],
                    vec![
                        "Use .clone() to duplicate".to_string(),
                        "Pass by reference (&) instead".to_string(),
                    ],
                    Some("Call `.clone()` on the value before moving, or pass a reference `&value`".to_string()),
                ),
                (
                    "cannot borrow".to_string(),
                    vec![
                        "Multiple mutable references".to_string(),
                        "Borrow checker violation".to_string(),
                    ],
                    vec![
                        "Use a single mutable reference or multiple immutable refs".to_string(),
                        "Restructure to avoid simultaneous mutable borrows".to_string(),
                    ],
                    Some(
                        "Restructure code to avoid simultaneous mutable borrows; use `RefCell` or split borrows"
                            .to_string(),
                    ),
                ),
                (
                    "missing lifetime".to_string(),
                    vec![
                        "Lifetime elision ambiguous".to_string(),
                        "Multiple references in signature".to_string(),
                    ],
                    vec![
                        "Add explicit lifetime parameters".to_string(),
                        "Use '_ for elided lifetimes".to_string(),
                    ],
                    Some(
                        "Add explicit lifetime annotations: `fn foo<'a>(x: &'a str, y: &'a str) -> &'a str`"
                            .to_string(),
                    ),
                ),
                (
                    "cannot return value referencing".to_string(),
                    vec![
                        "Returning reference to local variable".to_string(),
                        "Dangling reference".to_string(),
                    ],
                    vec![
                        "Return an owned value instead".to_string(),
                        "Use String instead of &str".to_string(),
                    ],
                    Some("Return an owned type (String, Vec<T>) instead of a reference".to_string()),
                ),
                (
                    "use of moved value".to_string(),
                    vec![
                        "Used after move".to_string(),
                        "Ownership transferred".to_string(),
                    ],
                    vec![
                        "Clone the value before moving".to_string(),
                        "Use references instead of moving".to_string(),
                    ],
                    Some("Clone the value before moving: `value.clone()`".to_string()),
                ),
                (
                    "expected `;`".to_string(),
                    vec!["Missing semicolon".to_string(), "Incomplete statement".to_string()],
                    vec!["Add ; at the end of the statement".to_string()],
                    Some("Add a semicolon `;` at the end of the statement".to_string()),
                ),
                (
                    "mismatched types".to_string(),
                    vec![
                        "Type mismatch".to_string(),
                        "Wrong type used in expression".to_string(),
                    ],
                    vec![
                        "Check the expected vs actual type".to_string(),
                        "Use explicit type conversion".to_string(),
                    ],
                    Some("Check the expected type and convert using `as` or `.into()`".to_string()),
                ),
                (
                    "unresolved import".to_string(),
                    vec![
                        "Module or crate not in dependencies".to_string(),
                        "Wrong import path".to_string(),
                    ],
                    vec![
                        "Check Cargo.toml for the dependency".to_string(),
                        "Verify the import path is correct".to_string(),
                    ],
                    Some("Add the missing dependency to Cargo.toml or fix the import path".to_string()),
                ),
            ],
        );

        // ── TypeScript / JavaScript ──────────────────────────────────────
        patterns.insert(
            "typescript".to_string(),
            vec![
                (
                    "TS2304".to_string(),
                    vec![
                        "Cannot find name".to_string(),
                        "Type or variable not defined".to_string(),
                    ],
                    vec![
                        "Check import statements".to_string(),
                        "Ensure the type is declared or installed".to_string(),
                    ],
                    Some("Import the missing type or add a declaration".to_string()),
                ),
                (
                    "TS2554".to_string(),
                    vec![
                        "Expected X arguments but got Y".to_string(),
                        "Wrong number of function arguments".to_string(),
                    ],
                    vec![
                        "Check the function signature for parameter count".to_string(),
                        "Add or remove arguments to match".to_string(),
                    ],
                    Some(
                        "Adjust the number of arguments to match the function signature"
                            .to_string(),
                    ),
                ),
                (
                    "TS2322".to_string(),
                    vec![
                        "Type not assignable".to_string(),
                        "Type mismatch in assignment".to_string(),
                    ],
                    vec![
                        "Check the expected type".to_string(),
                        "Use type assertion or conversion".to_string(),
                    ],
                    Some("Fix the type to match, or use `as` for type assertion".to_string()),
                ),
                (
                    "TS2339".to_string(),
                    vec![
                        "Property does not exist on type".to_string(),
                        "Wrong property name or missing interface field".to_string(),
                    ],
                    vec![
                        "Check the property name spelling".to_string(),
                        "Add property to interface definition".to_string(),
                    ],
                    Some(
                        "Check for typos or add the missing property to the type definition"
                            .to_string(),
                    ),
                ),
                (
                    "TS18048".to_string(),
                    vec![
                        "Object is possibly 'undefined'".to_string(),
                        "Missing null check".to_string(),
                    ],
                    vec![
                        "Use optional chaining (?.)".to_string(),
                        "Add null check before access".to_string(),
                    ],
                    Some("Use optional chaining `?.` or add a null check".to_string()),
                ),
                (
                    "Cannot read properties of undefined".to_string(),
                    vec![
                        "Accessing property on undefined value".to_string(),
                        "Missing null/undefined check".to_string(),
                    ],
                    vec![
                        "Use optional chaining (?.)".to_string(),
                        "Check if the object exists before accessing".to_string(),
                    ],
                    Some(
                        "Use optional chaining `?.` or add a guard: `if (obj) { ... }`".to_string(),
                    ),
                ),
            ],
        );

        // ── Windows / system errors ──────────────────────────────────────
        patterns.insert(
            "windows".to_string(),
            vec![
                (
                    "The system cannot find the file specified".to_string(),
                    vec![
                        "File path is incorrect".to_string(),
                        "File does not exist".to_string(),
                    ],
                    vec![
                        "Check the file path spelling".to_string(),
                        "Use absolute path or verify working directory".to_string(),
                    ],
                    Some(
                        "Verify the file path and ensure the file exists at that location"
                            .to_string(),
                    ),
                ),
                (
                    "Access is denied".to_string(),
                    vec![
                        "Permission denied".to_string(),
                        "File or directory is protected".to_string(),
                    ],
                    vec![
                        "Run as administrator".to_string(),
                        "Check file permissions".to_string(),
                    ],
                    Some(
                        "Run the application with elevated privileges or change file permissions"
                            .to_string(),
                    ),
                ),
                (
                    "The process cannot access the file".to_string(),
                    vec![
                        "File is locked by another process".to_string(),
                        "File handle not closed".to_string(),
                    ],
                    vec![
                        "Close other programs using the file".to_string(),
                        "Ensure file handles are properly closed".to_string(),
                    ],
                    Some(
                        "Close the file in other programs or ensure proper handle cleanup"
                            .to_string(),
                    ),
                ),
                (
                    "No such file or directory".to_string(),
                    vec![
                        "Path does not exist".to_string(),
                        "Wrong path separator".to_string(),
                    ],
                    vec![
                        "Verify the path exists".to_string(),
                        "Use forward slashes or escaped backslashes".to_string(),
                    ],
                    Some("Check the path and ensure the directory structure exists".to_string()),
                ),
            ],
        );

        Self { patterns }
    }

    /// Analyze an error message and return matching error analyses.
    pub fn analyze(&self, error_msg: &str, language: &str) -> Vec<ErrorAnalysis> {
        let mut results = Vec::new();

        if let Some(lang_patterns) = self.patterns.get(language) {
            for (error_type, causes, suggestions, suggested_fix) in lang_patterns {
                if error_msg.contains(error_type) {
                    results.push(ErrorAnalysis {
                        error_type: error_type.clone(),
                        message: error_msg.to_string(),
                        possible_causes: causes.clone(),
                        suggestions: suggestions.clone(),
                        line: self.extract_line(error_msg),
                        column: self.extract_column(error_msg),
                        suggested_fix: suggested_fix.clone(),
                    });
                }
            }
        }

        results
    }

    /// Convenience method: returns the best suggested fix string for an
    /// error message, or `None` if no pattern matches.
    pub fn suggest_fix(&self, error_msg: &str, language: &str) -> Option<String> {
        if let Some(lang_patterns) = self.patterns.get(language) {
            for (error_type, _causes, _suggestions, suggested_fix) in lang_patterns {
                if error_msg.contains(error_type) {
                    return suggested_fix.clone();
                }
            }
        }

        None
    }

    fn extract_line(&self, msg: &str) -> Option<usize> {
        let re = regex::Regex::new(r"line\s+(\d+)")
            .expect("failed to compile line regex pattern 'line\\s+(\\d+)'");
        re.captures(msg)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    fn extract_column(&self, msg: &str) -> Option<usize> {
        let re = regex::Regex::new(r"column\s+(\d+)")
            .expect("failed to compile column regex pattern 'column\\s+(\\d+)'");
        re.captures(msg)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    /// Return all registered languages.
    pub fn supported_languages(&self) -> Vec<String> {
        self.patterns.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Python patterns ─────────────────────────────────────────────────

    #[test]
    fn test_python_syntax_error() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("SyntaxError: invalid syntax at line 10", "python");
        assert!(!results.is_empty());
        let result = &results[0];
        assert_eq!(result.error_type, "SyntaxError");
        assert_eq!(result.line, Some(10));
        assert!(result.suggested_fix.is_some());
    }

    #[test]
    fn test_python_name_error() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("NameError: name 'x' is not defined", "python");
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "NameError");
    }

    #[test]
    fn test_python_module_not_found() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("ModuleNotFoundError: No module named 'foo'", "python");
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "ModuleNotFoundError");
    }

    // ── Rust patterns ───────────────────────────────────────────────────

    #[test]
    fn test_rust_borrow_of_moved_value() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("error[E0382]: borrow of moved value: `x`", "rust");
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "borrow of moved value");
    }

    #[test]
    fn test_rust_missing_semicolon() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("error: expected `;` at line 42", "rust");
        assert!(!results.is_empty());
        assert!(results[0].suggested_fix.is_some());
    }

    #[test]
    fn test_rust_cannot_borrow() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze(
            "error[E0502]: cannot borrow `x` as mutable because it is also borrowed as immutable",
            "rust",
        );
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "cannot borrow");
    }

    #[test]
    fn test_rust_missing_lifetime() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("error[E0106]: missing lifetime specifier", "rust");
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "missing lifetime");
    }

    // ── TypeScript patterns ─────────────────────────────────────────────

    #[test]
    fn test_ts2304() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("TS2304: Cannot find name 'FooBar'", "typescript");
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "TS2304");
    }

    #[test]
    fn test_ts2554() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("TS2554: Expected 2 arguments but got 3", "typescript");
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "TS2554");
    }

    #[test]
    fn test_ts2339() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze(
            "TS2339: Property 'foo' does not exist on type 'Bar'",
            "typescript",
        );
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "TS2339");
    }

    #[test]
    fn test_ts18048() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("TS18048: 'obj' is possibly 'undefined'", "typescript");
        assert!(!results.is_empty());
        assert_eq!(results[0].error_type, "TS18048");
    }

    // ── Windows patterns ────────────────────────────────────────────────

    #[test]
    fn test_windows_file_not_found() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("The system cannot find the file specified", "windows");
        assert!(!results.is_empty());
        assert!(results[0].suggested_fix.is_some());
    }

    #[test]
    fn test_windows_access_denied() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("Access is denied", "windows");
        assert!(!results.is_empty());
    }

    // ── suggest_fix convenience ─────────────────────────────────────────

    #[test]
    fn test_suggest_fix_rust() {
        let analyzer = ErrorAnalyzer::new();
        let fix = analyzer.suggest_fix("error: expected `;` at line 5", "rust");
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("semicolon"));
    }

    #[test]
    fn test_suggest_fix_none() {
        let analyzer = ErrorAnalyzer::new();
        let fix = analyzer.suggest_fix("unknown error message", "rust");
        assert!(fix.is_none());
    }

    #[test]
    fn test_suggest_fix_typescript() {
        let analyzer = ErrorAnalyzer::new();
        let fix = analyzer.suggest_fix("TS2304: Cannot find name 'Foo'", "typescript");
        assert!(fix.is_some());
    }

    // ── Helper parsing ──────────────────────────────────────────────────

    #[test]
    fn test_extract_line() {
        let analyzer = ErrorAnalyzer::new();
        let line = analyzer.extract_line("error at line 42 column 7");
        assert_eq!(line, Some(42));
    }

    #[test]
    fn test_no_match_returns_empty() {
        let analyzer = ErrorAnalyzer::new();
        let results = analyzer.analyze("just some random error", "rust");
        assert!(results.is_empty());
    }
}
