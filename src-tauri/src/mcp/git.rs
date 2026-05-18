use super::McpServer;
use async_trait::async_trait;
use git2::Repository;

#[derive(Debug)]
pub struct GitMcp {
    id: String,
}

impl Default for GitMcp {
    fn default() -> Self {
        Self::new()
    }
}

impl GitMcp {
    pub fn new() -> Self {
        Self {
            id: "git".to_string(),
        }
    }
}

#[async_trait]
impl McpServer for GitMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Git MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Git MCP server ready");
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn is_running(&self) -> bool {
        true
    }

    async fn handle_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let repo_path = params["repo_path"].as_str().unwrap_or(".");
        let repo = Repository::open(repo_path)?;

        match method {
            "status" => {
                let statuses = repo.statuses(None)?;
                let files: Vec<String> = statuses
                    .iter()
                    .filter_map(|s| s.path().map(|p| p.to_string()))
                    .collect();
                Ok(serde_json::json!({ "files": files }))
            }
            "log" => {
                let mut revwalk = repo.revwalk()?;
                revwalk.push_head()?;
                let commits: Vec<serde_json::Value> = revwalk
                    .take(10)
                    .filter_map(|id| {
                        let id = id.ok()?;
                        let commit = repo.find_commit(id).ok()?;
                        Some(serde_json::json!({
                            "id": commit.id().to_string(),
                            "message": commit.message().unwrap_or(""),
                            "author": commit.author().name().unwrap_or(""),
                            "time": commit.time().seconds(),
                        }))
                    })
                    .collect();
                Ok(serde_json::json!({ "commits": commits }))
            }
            "diff" => {
                let tree = repo.head()?.peel_to_tree()?;
                let diff = repo.diff_tree_to_workdir(Some(&tree), None)?;
                let mut diff_text = String::new();
                diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
                    let prefix = match line.origin() {
                        '+' => "+",
                        '-' => "-",
                        _ => " ",
                    };
                    if let Ok(content) = std::str::from_utf8(line.content()) {
                        diff_text.push_str(&format!("{}{}", prefix, content));
                    }
                    true
                })?;
                Ok(serde_json::json!({ "diff": diff_text }))
            }
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
