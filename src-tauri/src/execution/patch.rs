use similar::{ChangeTag, TextDiff};

pub struct PatchEngine;

impl Default for PatchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn apply(&self, original: &str, patch_str: &str) -> anyhow::Result<String> {
        let diff = TextDiff::from_lines(original, patch_str);
        let mut result = String::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => result.push_str(change.value()),
                ChangeTag::Insert => result.push_str(change.value()),
                ChangeTag::Delete => { /* skip deletions */ }
            }
        }

        Ok(result)
    }

    pub async fn generate_patch(&self, original: &str, modified: &str) -> String {
        let diff = TextDiff::from_lines(original, modified);
        let mut patch = String::new();

        for change in diff.iter_all_changes() {
            let prefix = match change.tag() {
                ChangeTag::Equal => " ",
                ChangeTag::Insert => "+",
                ChangeTag::Delete => "-",
            };
            patch.push_str(&format!("{}{}", prefix, change.value()));
        }

        patch
    }

    pub async fn validate_patch(&self, original: &str, patch_str: &str) -> bool {
        self.apply(original, patch_str).await.is_ok()
    }
}
