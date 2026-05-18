use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub unchanged: Vec<String>,
    pub change_count: usize,
    pub is_empty: bool,
}

pub struct DiffEngine;

impl Default for DiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn compute(&self, old: &str, new_: &str) -> DiffResult {
        let diff = TextDiff::from_lines(old, new_);
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut unchanged = Vec::new();
        let mut change_count = 0;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => unchanged.push(change.value().to_string()),
                ChangeTag::Insert => {
                    added.push(change.value().to_string());
                    change_count += 1;
                }
                ChangeTag::Delete => {
                    removed.push(change.value().to_string());
                    change_count += 1;
                }
            }
        }

        DiffResult {
            added,
            removed,
            unchanged,
            change_count,
            is_empty: change_count == 0,
        }
    }

    pub async fn compute_stats(&self, old: &str, new_: &str) -> DiffStats {
        let result = self.compute(old, new_).await;
        DiffStats {
            additions: result.added.len(),
            deletions: result.removed.len(),
            changes: result.change_count,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub changes: usize,
}
