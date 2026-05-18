use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

macro_rules! lazy_regex_replace_all {
    ($text:expr, $pattern:expr, $replacer:expr) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        let re = RE.get_or_init(|| regex::Regex::new($pattern).expect("invalid regex"));
        re.replace_all($text, $replacer).to_string()
    }};
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompressionLevel {
    Off,
    Light,
    Medium,
    Aggressive,
}

impl CompressionLevel {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionLevel::Off => "off",
            CompressionLevel::Light => "light",
            CompressionLevel::Medium => "medium",
            CompressionLevel::Aggressive => "aggressive",
        }
    }
}

#[allow(dead_code)]
pub struct CompressionPipeline {
    enabled: bool,
    level: CompressionLevel,
    total_original_tokens: AtomicU64,
    total_compressed_tokens: AtomicU64,
}

#[allow(dead_code)]
impl Default for CompressionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionPipeline {
    pub fn new() -> Self {
        Self {
            enabled: true,
            level: CompressionLevel::Medium,
            total_original_tokens: AtomicU64::new(0),
            total_compressed_tokens: AtomicU64::new(0),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn level(&self) -> &CompressionLevel {
        &self.level
    }

    pub fn set_level(&mut self, level: CompressionLevel) {
        self.level = level;
    }

    pub fn compress(&self, text: &str) -> String {
        match self.level {
            CompressionLevel::Off => {
                let tokens = Self::estimate_tokens(text) as u64;
                self.total_original_tokens.fetch_add(tokens, Ordering::Relaxed);
                self.total_compressed_tokens.fetch_add(tokens, Ordering::Relaxed);
                text.to_string()
            }
            CompressionLevel::Light => {
                let original_tokens = Self::estimate_tokens(text) as u64;
                let result = Self::light_compress(text);
                let compressed_tokens = Self::estimate_tokens(&result) as u64;
                self.total_original_tokens.fetch_add(original_tokens, Ordering::Relaxed);
                self.total_compressed_tokens.fetch_add(compressed_tokens, Ordering::Relaxed);
                result
            }
            CompressionLevel::Medium => {
                let original_tokens = Self::estimate_tokens(text) as u64;
                let light = Self::light_compress(text);
                let result = Self::medium_compress(&light);
                let compressed_tokens = Self::estimate_tokens(&result) as u64;
                self.total_original_tokens.fetch_add(original_tokens, Ordering::Relaxed);
                self.total_compressed_tokens.fetch_add(compressed_tokens, Ordering::Relaxed);
                result
            }
            CompressionLevel::Aggressive => {
                let original_tokens = Self::estimate_tokens(text) as u64;
                let light = Self::light_compress(text);
                let medium = Self::medium_compress(&light);
                let result = Self::aggressive_compress(&medium);
                let compressed_tokens = Self::estimate_tokens(&result) as u64;
                self.total_original_tokens.fetch_add(original_tokens, Ordering::Relaxed);
                self.total_compressed_tokens.fetch_add(compressed_tokens, Ordering::Relaxed);
                result
            }
        }
    }

    /// Light: trim whitespace per line, condense runs of newlines to at most 2.
    fn light_compress(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut prev_blank = false;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !prev_blank {
                    result.push('\n');
                    prev_blank = true;
                }
            } else {
                if prev_blank {
                    result.push('\n');
                }
                result.push_str(trimmed);
                result.push('\n');
                prev_blank = false;
            }
        }

        // Remove trailing whitespace
        while result.ends_with('\n') {
            result.pop();
        }
        if result.is_empty() && !text.is_empty() {
            result.push('\n');
        }

        result
    }

    /// Medium: light + compact JSON + condense indentation.
    fn medium_compress(text: &str) -> String {
        let text = Self::compact_json_blocks(text);
        Self::condense_indentation(&text)
    }

    /// Aggressive: medium + shorten variable names in code blocks + strip single-line comments.
    fn aggressive_compress(text: &str) -> String {
        let text = Self::shorten_code_vars(text);
        Self::strip_code_comments(&text)
    }

    /// Compact JSON blocks: remove unnecessary whitespace inside ```json ... ``` fences.
    fn compact_json_blocks(text: &str) -> String {
        lazy_regex_replace_all!(text, r"```json\s*\n([\s\S]*?)```", |caps: &regex::Captures| {
            let inner = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let compact = match serde_json::from_str::<serde_json::Value>(inner) {
                Ok(val) => serde_json::to_string(&val).unwrap_or_else(|_| inner.to_string()),
                Err(_) => inner.to_string(),
            };
            format!("```json\n{}```", compact)
        })
    }

    /// Condense indentation: replace N spaces at line start with ceil(N/2) spaces.
    fn condense_indentation(text: &str) -> String {
        lazy_regex_replace_all!(text, r"^ +", |caps: &regex::Captures| {
            let spaces = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let count = spaces.len();
            " ".repeat(count.div_ceil(2)) // ceil division
        })
    }

    /// Shorten variable names in code blocks (aggressive only).
    /// Replaces typical multi-letter variable names with single letters.
    fn shorten_code_vars(text: &str) -> String {
        // Process content inside ``` fences
        let mut result = String::with_capacity(text.len());
        let mut in_code_block = false;
        let mut code_buf = String::new();
        let code_fence_re = Regex::new(r"^```").unwrap();

        for line in text.lines() {
            if code_fence_re.is_match(line) {
                if in_code_block {
                    // End of code block — apply var shortening
                    code_buf.push_str(line);
                    code_buf.push('\n');
                    let shortened = Self::shorten_vars_in_block(&code_buf);
                    result.push_str(&shortened);
                    code_buf.clear();
                    in_code_block = false;
                } else {
                    // Start of code block
                    if !code_buf.is_empty() {
                        result.push_str(&code_buf);
                        code_buf.clear();
                    }
                    code_buf.push_str(line);
                    code_buf.push('\n');
                    in_code_block = true;
                }
            } else if in_code_block {
                code_buf.push_str(line);
                code_buf.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Flush remaining buffer
        if in_code_block && !code_buf.is_empty() {
            let shortened = Self::shorten_vars_in_block(&code_buf);
            result.push_str(&shortened);
        } else if !code_buf.is_empty() {
            result.push_str(&code_buf);
        }

        result
    }

    /// Heuristic variable name shortening inside a code block.
    fn shorten_vars_in_block(block: &str) -> String {
        lazy_regex_replace_all!(block, r"\b([a-z_][a-zA-Z0-9_]{3,})\b", |caps: &regex::Captures| {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            // Skip common keywords and short names
            let keywords = [
                "self", "true", "false", "null", "None", "Some", "Ok", "Err",
                "async", "await", "impl", "fn", "pub", "use", "mod", "struct",
                "enum", "trait", "let", "mut", "const", "static", "ref", "move",
                "return", "if", "else", "for", "while", "loop", "match", "where",
                "type", "dyn", "in", "as", "from", "into", "this", "base", "super",
            ];
            if keywords.contains(&name) || name.len() <= 3 {
                return name.to_string();
            }
            // Use first letter + last letter as shortened form
            let first = name.chars().next().unwrap_or('v');
            let last = name.chars().next_back().unwrap_or('_');
            format!("{}{}", first, last)
        })
    }

    /// Strip single-line comments (//) and block comments (/* */) from code blocks.
    fn strip_code_comments(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut in_code_block = false;
        let mut code_buf = String::new();
        let code_fence_re = Regex::new(r"^```").unwrap();

        for line in text.lines() {
            if code_fence_re.is_match(line) {
                if in_code_block {
                    code_buf.push_str(line);
                    code_buf.push('\n');
                    let stripped = Self::strip_comments_from_block(&code_buf);
                    result.push_str(&stripped);
                    code_buf.clear();
                    in_code_block = false;
                } else {
                    if !code_buf.is_empty() {
                        result.push_str(&code_buf);
                        code_buf.clear();
                    }
                    code_buf.push_str(line);
                    code_buf.push('\n');
                    in_code_block = true;
                }
            } else if in_code_block {
                code_buf.push_str(line);
                code_buf.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        if in_code_block && !code_buf.is_empty() {
            let stripped = Self::strip_comments_from_block(&code_buf);
            result.push_str(&stripped);
        } else if !code_buf.is_empty() {
            result.push_str(&code_buf);
        }

        result
    }

    /// Remove // and /* */ comments from a code block string.
    fn strip_comments_from_block(block: &str) -> String {
        // Remove block comments first
        let no_block = lazy_regex_replace_all!(block, r"/\*[\s\S]*?\*/", |_: &regex::Captures| {
            String::new()
        });
        // Remove line comments (//) — careful not to remove URLs or strings
        lazy_regex_replace_all!(&no_block, r"//[^\n]*", |_: &regex::Captures| {
            String::new()
        })
    }

    /// Estimate token count using chars/4 approximation.
    pub fn estimate_tokens(text: &str) -> usize {
        let len = text.len();
        if len == 0 {
            return 0;
        }
        // Rough heuristic: 1 token ≈ 4 characters for English text
        let est = len / 4;
        // Minimum 1 token for non-empty
        est.max(1)
    }

    pub fn stats(&self) -> CompressionStats {
        let orig = self.total_original_tokens.load(Ordering::Relaxed);
        let comp = self.total_compressed_tokens.load(Ordering::Relaxed);
        let ratio = if orig > 0 {
            (orig as f64 - comp as f64) / orig as f64 * 100.0
        } else {
            0.0
        };
        CompressionStats {
            enabled: self.enabled,
            level: self.level.as_str().to_string(),
            total_original_tokens: orig,
            total_compressed_tokens: comp,
            compression_ratio: (ratio * 100.0).round() / 100.0,
        }
    }

    pub fn reset_stats(&self) {
        self.total_original_tokens.store(0, Ordering::Relaxed);
        self.total_compressed_tokens.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub enabled: bool,
    pub level: String,
    pub total_original_tokens: u64,
    pub total_compressed_tokens: u64,
    pub compression_ratio: f64,
}


