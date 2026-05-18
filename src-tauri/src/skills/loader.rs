use super::SkillManifest;
use std::path::Path;

pub struct SkillLoader;

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillLoader {
    pub fn new() -> Self {
        Self
    }

    pub async fn load_manifest(&self, path: &str) -> anyhow::Result<SkillManifest> {
        let manifest_path = Path::new(path).join("manifest.toml");
        let content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: SkillManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    pub async fn find_skills(&self, dir: &str) -> anyhow::Result<Vec<SkillManifest>> {
        let mut skills = Vec::new();
        let mut read_dir = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let manifest_path = entry.path().join("manifest.toml");
                if manifest_path.exists() {
                    if let Ok(manifest) = self
                        .load_manifest(entry.path().to_string_lossy().as_ref())
                        .await
                    {
                        skills.push(manifest);
                    }
                }
            }
        }

        Ok(skills)
    }
}
