use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct EventBusCore {
    subscribers: RwLock<HashMap<String, Vec<mpsc::Sender<SystemEvent>>>>,
    history: RwLock<VecDeque<SystemEvent>>,
}

impl Default for EventBusCore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBusCore {
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            history: RwLock::new(VecDeque::new()),
        }
    }

    pub async fn emit(&self, event: SystemEvent) {
        // Store in history (keep last 1000 events)
        {
            let mut history = self.history.write().await;
            history.push_back(event.clone());
            if history.len() > 1000 {
                history.pop_front();
            }
        }

        let subscribers = self.subscribers.read().await;
        let event_type = &event.event_type;

        // Deliver to exact match subscribers
        if let Some(senders) = subscribers.get(event_type) {
            for sender in senders {
                let _ = sender.send(event.clone()).await;
            }
        }

        // Deliver to wildcard subscribers
        if let Some(senders) = subscribers.get("*") {
            for sender in senders {
                let _ = sender.send(event.clone()).await;
            }
        }
    }

    pub async fn subscribe(&self, pattern: &str) -> mpsc::Receiver<SystemEvent> {
        let (tx, rx) = mpsc::channel(256);
        self.subscribers
            .write()
            .await
            .entry(pattern.to_string())
            .or_default()
            .push(tx);
        rx
    }

    pub async fn unsubscribe(&self, pattern: &str, index: usize) {
        if let Some(senders) = self.subscribers.write().await.get_mut(pattern) {
            if index < senders.len() {
                senders.remove(index);
            }
        }
    }

    pub async fn recent_events(&self, limit: usize) -> Vec<SystemEvent> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).rev().cloned().collect()
    }
}
