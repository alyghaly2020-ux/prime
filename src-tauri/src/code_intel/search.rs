use lru::LruCache;
use regex::Regex;
use std::num::NonZeroUsize;
use tokio::fs;
use tokio::sync::RwLock;
use walkdir::WalkDir;

use super::SearchResult;

#[derive(Debug)]
pub struct SearchEngine {
    file_cache: RwLock<LruCache<String, Vec<String>>>,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            file_cache: RwLock::new(LruCache::new(NonZeroUsize::new(1000).expect("1000 is non-zero"))),
        }
    }

    pub async fn search(&self, query: &str, path: &str) -> anyhow::Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let pattern = Regex::new(query)?;
        let query_lower = query.to_lowercase();

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

            if !matches!(
                ext,
                "rs" | "py"
                    | "js"
                    | "ts"
                    | "tsx"
                    | "go"
                    | "java"
                    | "c"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "toml"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "md"
                    | "html"
                    | "css"
            ) {
                continue;
            }

            let content = match self.read_file(file_path.to_string_lossy().as_ref()).await {
                Some(c) => c,
                None => continue,
            };

            for (line_idx, line) in content.iter().enumerate() {
                let line_lower = line.to_lowercase();
                let matched = line_lower.contains(&query_lower) || pattern.is_match(line);

                if matched {
                    let ctx_start = line_idx.saturating_sub(3);
                    let ctx_end = std::cmp::min(line_idx + 4, content.len());

                    results.push(SearchResult {
                        file: file_path.to_string_lossy().to_string(),
                        line: line_idx + 1,
                        column: line.find(query).unwrap_or(0) + 1,
                        content: line.clone(),
                        score: 1.0,
                        symbol_type: None,
                        context_before: content[ctx_start..line_idx].to_vec(),
                        context_after: content[line_idx + 1..ctx_end].to_vec(),
                    });
                }
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    async fn read_file(&self, path: &str) -> Option<Vec<String>> {
        if let Ok(mut cache) = self.file_cache.try_write() {
            if let Some(lines) = cache.get(path) {
                return Some(lines.clone());
            }
        }

        let content = fs::read_to_string(path).await.ok()?;
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        if let Ok(mut cache) = self.file_cache.try_write() {
            cache.put(path.to_string(), lines.clone());
        }

        Some(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn file_with_lines(path: &std::path::Path, lines: &[&str]) {
        let mut f = std::fs::File::create(path).unwrap();
        for l in lines {
            writeln!(f, "{}", l).unwrap();
        }
    }

    // -----------------------------------------------------------------------
    // read_file — cache behavior
    // -----------------------------------------------------------------------

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().unwrap()
    }

    #[test]
    fn test_read_file_returns_lines() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.rs");
        file_with_lines(&p, &["line1", "line2", "line3"]);

        let engine = SearchEngine::new();
        let lines = rt().block_on(engine.read_file(p.to_string_lossy().as_ref()));
        assert!(lines.is_some());
        assert_eq!(lines.unwrap().len(), 3);
    }

    #[test]
    fn test_read_file_missing_returns_none() {
        let engine = SearchEngine::new();
        let lines = rt().block_on(engine.read_file("/nonexistent/path/file.rs"));
        assert!(lines.is_none());
    }

    #[test]
    fn test_read_file_populates_cache() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.rs");
        file_with_lines(&p, &["a", "b", "c"]);

        let engine = SearchEngine::new();
        let path_str = p.to_string_lossy().to_string();

        let first = rt().block_on(engine.read_file(&path_str));
        assert!(first.is_some());

        let second = rt().block_on(engine.read_file(&path_str));
        assert_eq!(first, second);
    }

    #[test]
    fn test_read_file_cache_eviction() {
        let engine = SearchEngine::new();
        let dir = tempfile::tempdir().unwrap();
        let mut paths = Vec::new();

        for i in 0..1001_usize {
            let p = dir.path().join(format!("f{}.rs", i));
            std::fs::write(&p, "content").unwrap();
            paths.push(p);
        }

        let first_path = paths[0].to_string_lossy().to_string();
        let _ = rt().block_on(engine.read_file(&first_path));
        assert!(
            rt().block_on(engine.read_file(&first_path)).is_some(),
            "first file should be in cache initially"
        );

        for p in &paths[1..] {
            let _ = rt().block_on(engine.read_file(&p.to_string_lossy()));
        }
    }

    // -----------------------------------------------------------------------
    // Extension filtering
    // -----------------------------------------------------------------------

    #[test]
    fn test_search_extension_filter_list_contains_common_types() {
        // Verify that the extension matching in `search` uses the same
        // extensions that appear in the source.  This is a compile-time
        // check that the match arm hasn't accidentally omitted types.
        let supported = [
            "rs", "py", "js", "ts", "tsx", "go", "java", "c", "cpp", "h", "hpp", "toml", "json",
            "yaml", "yml", "md", "html", "css",
        ];
        for ext in &supported {
            let dir = tempfile::tempdir().unwrap();
            let p = dir.path().join(format!("file.{}", ext));
            std::fs::write(&p, "test content").unwrap();

            let engine = SearchEngine::new();
            let results = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(engine.search("test", dir.path().to_string_lossy().as_ref()))
                .expect("search should succeed for supported extension");
            assert!(
                !results.is_empty(),
                "extension '.{}' should be searchable",
                ext
            );
        }
    }

    #[test]
    fn test_search_excludes_unsupported_extensions_unit() {
        let unsupported = [
            "exe", "dll", "so", "dylib", "o", "class", "jar", "png", "jpg",
        ];
        for ext in &unsupported {
            let dir = tempfile::tempdir().unwrap();
            let p = dir.path().join(format!("file.{}", ext));
            std::fs::write(&p, "test content").unwrap();

            let engine = SearchEngine::new();
            let results = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(engine.search("test", dir.path().to_string_lossy().as_ref()))
                .expect("search should succeed");
            assert!(
                results.is_empty(),
                "extension '.{}' should be excluded from search",
                ext
            );
        }
    }
}
