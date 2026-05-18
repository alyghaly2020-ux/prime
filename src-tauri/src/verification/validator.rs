use regex::Regex;

pub struct OutputValidator;

impl Default for OutputValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_syntax(&self, code: &str, language: &str) -> Vec<String> {
        let mut errors = Vec::new();
        if language == "json" && serde_json::from_str::<serde_json::Value>(code).is_err() {
            errors.push("Invalid JSON syntax".to_string());
        }
        errors
    }

    pub fn validate_no_secrets(&self, output: &str) -> Vec<String> {
        let mut warnings = Vec::new();
        let patterns = [
            (
                r#"(?i)(api[_-]?key|secret|password|token|credential)\s*[:=]\s*['"][^'"]+['"]"#,
                "Potential credential leak",
            ),
            (
                r"(?i)(-----BEGIN\s+(RSA|EC|OPENSSH|PGP)\s+PRIVATE\s+KEY-----)",
                "Private key detected in output",
            ),
            (r"(?i)ghp_[a-zA-Z0-9]{36}", "GitHub token detected"),
            (r"(?i)sk-[a-zA-Z0-9]{32,}", "OpenAI API key detected"),
        ];

        for (pattern, message) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(output) {
                    warnings.push(message.to_string());
                }
            }
        }

        warnings
    }

    pub fn validate_size(&self, output: &str, max_size: usize) -> bool {
        output.len() <= max_size
    }
}
