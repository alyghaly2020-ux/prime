#[derive(Debug)]
pub struct CodePatches;

impl Default for CodePatches {
    fn default() -> Self {
        Self::new()
    }
}

impl CodePatches {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_patch(&self, original: &str, modified: &str) -> String {
        let diff = similar::TextDiff::from_lines(original, modified);
        let mut patch = String::new();

        for change in diff.iter_all_changes() {
            let prefix = match change.tag() {
                similar::ChangeTag::Equal => " ",
                similar::ChangeTag::Insert => "+",
                similar::ChangeTag::Delete => "-",
            };
            patch.push_str(&format!("{}{}", prefix, change.value()));
        }

        patch
    }

    pub async fn apply_patch(&self, original: &str, patch: &str) -> anyhow::Result<String> {
        let diff = similar::TextDiff::from_lines(original, patch);
        let mut result = String::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Equal | similar::ChangeTag::Insert => {
                    result.push_str(change.value());
                }
                similar::ChangeTag::Delete => {}
            }
        }

        Ok(result)
    }
}
