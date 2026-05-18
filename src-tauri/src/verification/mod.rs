//! Code verification pipeline. Multi-language linting, automated code review (diff analysis), error message analysis with root-cause suggestions, self-healing auto-fixes, and test output parsing.

pub mod error_analyzer;
pub mod linter;
pub mod reviewer;
pub mod self_heal;
pub mod test_runner;
pub mod validator;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub score: f64,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: IssueSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub code: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

pub struct System {
    pub linter: Arc<linter::LintEngine>,
    pub test_runner: Arc<test_runner::TestRunner>,
    pub self_heal: Arc<self_heal::SelfHealingLoop>,
    pub validator: Arc<validator::OutputValidator>,
    pub reviewer: Arc<reviewer::CodeReviewer>,
    pub error_analyzer: Arc<error_analyzer::ErrorAnalyzer>,
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    pub fn new() -> Self {
        Self {
            linter: Arc::new(linter::LintEngine::new()),
            test_runner: Arc::new(test_runner::TestRunner::new()),
            self_heal: Arc::new(self_heal::SelfHealingLoop::new()),
            validator: Arc::new(validator::OutputValidator::new()),
            reviewer: Arc::new(reviewer::CodeReviewer::new()),
            error_analyzer: Arc::new(error_analyzer::ErrorAnalyzer::new()),
        }
    }

    pub async fn verify(&self, code: &str, language: &str) -> VerificationResult {
        let lint_result = self.linter.lint(code, language);

        let mut errors = lint_result.errors;
        let warnings = lint_result.warnings;
        let suggestions = lint_result.suggestions;

        let test_result = self.test_runner.run_tests(code, language).await;
        if let Some(issues) = test_result.failures {
            for issue in issues {
                errors.push(Issue {
                    severity: IssueSeverity::Error,
                    message: issue,
                    file: None,
                    line: None,
                    column: None,
                    code: None,
                    suggestion: None,
                });
            }
        }

        let passed = errors.is_empty();

        VerificationResult {
            passed,
            score: if passed { 1.0 } else { 0.0 },
            errors,
            warnings,
            suggestions,
        }
    }
}
