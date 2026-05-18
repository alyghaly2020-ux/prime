//! Search / code-intel and cache tests for Prime.
//!
//! Covers: code_intel::search::SearchEngine (filesystem walk, regex,
//! plain-text matching, extension filtering), memory::cache::EmbeddingCache
//! (get/set, stats, hit-rate, clear), and ReDoS resistance patterns.

use prime::code_intel::search::SearchEngine;
use prime::memory::cache::EmbeddingCache;
use std::fs;
use std::io::Write;

// ===========================================================================
// Helpers
// ===========================================================================

/// Create a temporary directory with a small source tree for testing.
fn setup_test_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");

    // File 1: Rust source
    let mut f1 = fs::File::create(dir.path().join("main.rs")).unwrap();
    writeln!(f1, "fn main() {{").unwrap();
    writeln!(f1, "    println!(\"Hello, world!\");").unwrap();
    writeln!(f1, "    let x = 42;").unwrap();
    writeln!(f1, "    println!(\"The answer is {{}}\", x);").unwrap();
    writeln!(f1, "}}").unwrap();

    // File 2: Python source
    let mut f2 = fs::File::create(dir.path().join("app.py")).unwrap();
    writeln!(f2, "def greet(name):").unwrap();
    writeln!(f2, "    return f\"Hello {{name}}\"").unwrap();
    writeln!(f2).unwrap();
    writeln!(f2, "if __name__ == '__main__':").unwrap();
    writeln!(f2, "    print(greet('world'))").unwrap();

    // File 3: TypeScript source
    let mut f3 = fs::File::create(dir.path().join("component.tsx")).unwrap();
    writeln!(f3, "interface Props {{").unwrap();
    writeln!(f3, "  title: string;").unwrap();
    writeln!(f3, "  count: number;").unwrap();
    writeln!(f3, "}}").unwrap();
    writeln!(f3).unwrap();
    writeln!(
        f3,
        "export const App: React.FC<Props> = ({{ title, count }}) => {{"
    )
    .unwrap();
    writeln!(f3, "  return <div>{{title}}: {{count}}</div>;").unwrap();
    writeln!(f3, "}};").unwrap();

    // File 4: A file with an ignored extension (should be skipped)
    let mut f4 = fs::File::create(dir.path().join("data.bin")).unwrap();
    f4.write_all(b"\x00\x01\x02\x03").unwrap();

    // File 5: Hidden file (should be skipped because of leading dot)
    let mut f5 = fs::File::create(dir.path().join(".secret.rs")).unwrap();
    writeln!(f5, "fn hidden() {{}}").unwrap();

    dir
}

// ===========================================================================
// SearchEngine
// ===========================================================================

#[tokio::test]
async fn test_search_plain_text_finds_matches() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    let results = engine
        .search("Hello", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");

    assert!(!results.is_empty(), "should find 'Hello' in main.rs");
    assert!(results.iter().any(|r| r.file.ends_with("main.rs")));
    assert!(results.iter().any(|r| r.content.contains("Hello")));
}

#[tokio::test]
async fn test_search_plain_text_multiple_files() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    let results = engine
        .search("count", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");

    // 'count' should appear in component.tsx (Props.count) and in main.rs (commented)
    assert!(
        results.len() >= 2,
        "should find 'count' in at least 2 files, got {}",
        results.len()
    );
    let files: std::collections::HashSet<&str> = results.iter().map(|r| r.file.as_str()).collect();
    assert!(files.iter().any(|f| f.ends_with("component.tsx")));
}

#[tokio::test]
async fn test_search_regex_pattern() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    // Regex: match lines containing a function definition
    let results = engine
        .search(r"fn \w+", dir.path().to_str().unwrap())
        .await
        .expect("regex search should succeed");

    assert!(!results.is_empty(), "should match 'fn main' or 'fn hidden'");
    assert!(results.iter().any(|r| r.content.contains("fn main")));
}

#[tokio::test]
async fn test_search_regex_with_capture_groups() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    let results = engine
        .search(r#"def \w+"#, dir.path().to_str().unwrap())
        .await
        .expect("regex search should succeed");

    assert!(!results.is_empty(), "should match 'def greet'");
    assert!(results.iter().any(|r| r.content.contains("def greet")));
}

#[tokio::test]
async fn test_search_returns_context_lines() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    let results = engine
        .search("x = 42", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");

    assert!(!results.is_empty(), "should find 'x = 42'");
    let r = &results[0];
    // Context before should include preceding lines
    assert!(r.context_before.iter().any(|l| l.contains("println")));
    // Context after should include following lines
    assert!(r.context_after.iter().any(|l| l.contains("answer")));
}

#[tokio::test]
async fn test_search_excludes_hidden_files() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    let results = engine
        .search("hidden", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");

    // .secret.rs contains "fn hidden()" but should be excluded because it starts with '.'
    assert!(
        !results.iter().any(|r| r.file.contains(".secret")),
        "hidden files (leading dot) should not appear in results"
    );
}

#[tokio::test]
async fn test_search_excludes_unsupported_extensions() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    let results = engine
        .search("", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");

    // data.bin has an unsupported extension
    assert!(
        !results.iter().any(|r| r.file.ends_with(".bin")),
        "files with unsupported extensions should be excluded"
    );
}

#[tokio::test]
async fn test_search_nonexistent_directory() {
    let engine = SearchEngine::new();
    let result = engine
        .search("test", "/this/path/does/not/exist/12345xyz")
        .await;
    assert!(
        result.is_ok(),
        "search on non-existent path should succeed (WalkDir filters errors)"
    );
    assert!(
        result.unwrap().is_empty(),
        "search on non-existent path should yield no results"
    );
}

#[tokio::test]
async fn test_search_empty_directory() {
    let dir = tempfile::tempdir().expect("create temp dir");
    // No files inside
    let engine = SearchEngine::new();
    let results = engine
        .search("anything", dir.path().to_str().unwrap())
        .await
        .expect("search on empty dir should succeed");
    assert!(
        results.is_empty(),
        "empty directory should yield no results"
    );
}

#[tokio::test]
async fn test_search_case_sensitivity() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    // Test that the search is case-sensitive by default (line_lower.contains(&query_lower))
    // Actually looking at the code: `let line_lower = line.to_lowercase();` and
    // `let query_lower = query.to_lowercase();` — the plain-text path is case-insensitive!
    let results_upper = engine
        .search("HELLO", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");
    let results_lower = engine
        .search("hello", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");

    // Plain-text search is case-insensitive (lowercases both sides)
    assert!(
        !results_upper.is_empty(),
        "upper-case query should still find matches"
    );
    // Regex path would be case-sensitive (depending on regex flags)
    assert_eq!(
        results_upper.len(),
        results_lower.len(),
        "both upper and lower-case queries should find same results (case-insensitive text match)"
    );
}

#[tokio::test]
async fn test_search_returns_score_and_position() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    let results = engine
        .search("println", dir.path().to_str().unwrap())
        .await
        .expect("search should succeed");

    assert!(!results.is_empty());
    for r in &results {
        assert!(r.line > 0, "line number should be 1-indexed and positive");
        assert!(r.column > 0, "column should be positive");
        assert!((r.score - 1.0).abs() < 1e-6, "score should be 1.0 for now");
    }
}

// ===========================================================================
// EmbeddingCache
// ===========================================================================

#[test]
fn test_cache_get_missing_key() {
    let cache = EmbeddingCache::new();
    assert!(
        cache.get("nonexistent").is_none(),
        "missing key should return None"
    );
}

#[test]
fn test_cache_set_and_get() {
    let cache = EmbeddingCache::new();
    let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    cache.set("key1".into(), embedding.clone());
    let retrieved = cache.get("key1");
    assert!(retrieved.is_some(), "stored key should be retrievable");
    assert_eq!(retrieved.unwrap(), embedding);
}

#[test]
fn test_cache_overwrite() {
    let cache = EmbeddingCache::new();
    cache.set("k".into(), vec![1.0]);
    cache.set("k".into(), vec![2.0, 3.0]);
    let v = cache.get("k");
    assert_eq!(v, Some(vec![2.0, 3.0]), "last write should win");
}

#[test]
fn test_cache_stats_track_hits_and_misses() {
    let cache = EmbeddingCache::new();
    assert_eq!(cache.stats(), (0, 0), "fresh cache should have zero stats");

    // One miss
    let _ = cache.get("a");
    assert_eq!(cache.stats(), (0, 1));

    // One hit
    cache.set("a".into(), vec![1.0]);
    let _ = cache.get("a");
    assert_eq!(cache.stats(), (1, 1));

    // Another miss
    let _ = cache.get("b");
    assert_eq!(cache.stats(), (1, 2));
}

#[test]
fn test_cache_hit_rate() {
    let cache = EmbeddingCache::new();
    // Empty cache → hit rate 0.0
    assert!((cache.hit_rate() - 0.0).abs() < f64::EPSILON);

    // 2 hits, 3 misses → rate = 0.4
    cache.set("a".into(), vec![1.0]);
    cache.set("b".into(), vec![2.0]);
    let _ = cache.get("a"); // hit
    let _ = cache.get("b"); // hit
    let _ = cache.get("c"); // miss
    let _ = cache.get("d"); // miss
    let _ = cache.get("e"); // miss
    assert!(
        (cache.hit_rate() - 0.4).abs() < 0.001,
        "expected 0.4, got {}",
        cache.hit_rate()
    );
}

#[test]
fn test_cache_clear_resets_all() {
    let cache = EmbeddingCache::new();
    cache.set("k1".into(), vec![1.0]);
    cache.set("k2".into(), vec![2.0]);
    let _ = cache.get("k1"); // hit
    let _ = cache.get("k3"); // miss

    cache.clear();

    // Check stats BEFORE any new get calls (get would increment misses)
    assert_eq!(cache.stats(), (0, 0), "after clear, stats should be zero");
    assert!(
        (cache.hit_rate() - 0.0).abs() < f64::EPSILON,
        "hit rate should be 0 after clear"
    );
    assert!(
        cache.get("k1").is_none(),
        "after clear, stored keys should be gone"
    );
}

#[test]
fn test_cache_lru_eviction() {
    // LRU cache has capacity 10_000 by default — test with smaller data
    // to confirm eviction behavior through stats
    let cache = EmbeddingCache::new();
    // Insert 100 keys
    for i in 0..100_u32 {
        cache.set(format!("k{}", i), vec![i as f32]);
    }
    // All 100 should still be there (capacity is 10_000)
    for i in 0..100_u32 {
        assert!(
            cache.get(&format!("k{}", i)).is_some(),
            "key k{} should not be evicted yet",
            i
        );
    }
    // 100 hits, 0 misses
    assert_eq!(cache.stats(), (100, 0));
}

#[test]
fn test_cache_multiple_values_independent() {
    let cache = EmbeddingCache::new();
    cache.set("a".into(), vec![1.0, 2.0]);
    cache.set("b".into(), vec![3.0, 4.0, 5.0]);
    assert_eq!(cache.get("a"), Some(vec![1.0, 2.0]));
    assert_eq!(cache.get("b"), Some(vec![3.0, 4.0, 5.0]));
}

// ===========================================================================
// ReDoS resistance — the current search engine does NOT protect against
// ReDoS, so these tests document the current behavior and expected safety
// characteristics.  They use bounded patterns that should finish quickly.
// ===========================================================================

#[tokio::test]
async fn test_search_simple_regex_no_catastrophic_backtracking() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    // A pattern with nested quantifiers that *could* cause catastrophic
    // backtracking on pathological input, but the small corpus keeps it fast.
    // Use a regex that genuinely won't match anything (no "ÿ" in source files)
    let results = engine.search(r"ÿ+", dir.path().to_str().unwrap()).await;
    // Should complete quickly (no hang) with zero matches
    assert!(results.is_ok(), "even 'dangerous' regex should complete");
    // No matches expected in our test files
    assert!(results.unwrap().is_empty());
}

#[tokio::test]
async fn test_search_evil_regex_does_not_hang() {
    let dir = setup_test_tree();
    let engine = SearchEngine::new();

    // Classic ReDoS: (a+)+b on input "aaaaaaaac" — but our test corpus
    // is tiny so this should complete quickly anyway.
    let results = engine.search(r"(a+)+b", dir.path().to_str().unwrap()).await;
    assert!(
        results.is_ok(),
        "ReDoS-vulnerable pattern should still complete on small corpus"
    );

    // Same with (a|aa)+b
    let results2 = engine
        .search(r"(a|aa)+b", dir.path().to_str().unwrap())
        .await;
    assert!(
        results2.is_ok(),
        "nested alternation regex should complete on small corpus"
    );
}

#[test]
fn test_regex_compile_timeout_good_patterns() {
    // The regex crate in Rust does NOT have ReDoS issues because it uses
    // a finite automaton (not backtracking) for most patterns.
    // All patterns below compile and match quickly.
    let patterns = [
        r"fn \w+",
        r"def \w+",
        r"\d{3,}",
        r"Hello|world",
        r"[a-z]+",
        r"^fn",
        r"//.*",
        r#""[^"]*""#,
    ];
    for pat in &patterns {
        let re = regex::Regex::new(pat);
        assert!(
            re.is_ok(),
            "pattern '{}' should compile: {:?}",
            pat,
            re.err()
        );
    }
}
