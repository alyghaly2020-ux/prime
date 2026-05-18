use super::loader::SkillLoader;
use notify::{Event, EventKind, RecommendedWatcher, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HotReload {
    loader: Arc<SkillLoader>,
    watched: RwLock<HashMap<String, RecommendedWatcher>>,
}

impl HotReload {
    pub fn new(loader: Arc<SkillLoader>) -> Self {
        Self {
            loader,
            watched: RwLock::new(HashMap::new()),
        }
    }

    pub async fn watch_skill(&self, path: &str) -> anyhow::Result<()> {
        let path = path.to_string();
        let watch_path = path.clone();
        let loader = self.loader.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    let loader = loader.clone();
                    let path = watch_path.clone();
                    tokio::spawn(async move {
                        if let Ok(manifest) = loader.load_manifest(&path).await {
                            tracing::info!("Hot-reloaded skill: {}", manifest.name);
                        }
                    });
                }
            }
        })?;

        watcher.watch(Path::new(&path), notify::RecursiveMode::NonRecursive)?;
        self.watched.write().await.insert(path, watcher);
        Ok(())
    }

    pub async fn unwatch(&self, path: &str) {
        self.watched.write().await.remove(path);
    }
}
