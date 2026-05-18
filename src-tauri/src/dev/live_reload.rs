use notify::{Event, EventKind, Watcher};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct LiveReload {
    watchers: RwLock<HashMap<String, notify::RecommendedWatcher>>,
}

impl Default for LiveReload {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveReload {
    pub fn new() -> Self {
        Self {
            watchers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn watch_directory(&self, path: &str) -> anyhow::Result<()> {
        let path = path.to_string();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    tracing::info!("File changed: {:?}", event.paths);
                }
            }
        })?;

        watcher.watch(
            std::path::Path::new(&path),
            notify::RecursiveMode::Recursive,
        )?;
        self.watchers.write().await.insert(path, watcher);
        Ok(())
    }

    pub async fn unwatch(&self, path: &str) {
        self.watchers.write().await.remove(path);
    }
}
