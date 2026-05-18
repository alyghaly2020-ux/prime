use super::event_bus::{EventBusCore, SystemEvent};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait Actor: Send + Sync {
    fn id(&self) -> &str;
    async fn handle(&self, event: SystemEvent);
}

pub struct ActorSystem {
    actors: RwLock<HashMap<String, Arc<dyn Actor>>>,
    bus: Arc<EventBusCore>,
}

impl ActorSystem {
    pub fn new(bus: Arc<EventBusCore>) -> Self {
        Self {
            actors: RwLock::new(HashMap::new()),
            bus,
        }
    }

    pub async fn register(&self, actor: Arc<dyn Actor>) {
        let id = actor.id().to_string();
        let mut rx = self.bus.subscribe(&id).await;
        let actor_clone = actor.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                actor_clone.handle(event).await;
            }
        });

        self.actors.write().await.insert(id, actor);
    }

    pub async fn send(&self, _actor_id: &str, event: SystemEvent) {
        self.bus.emit(event).await;
    }
}
