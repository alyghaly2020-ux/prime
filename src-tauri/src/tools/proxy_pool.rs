use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::proxy::ProxyRotator;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum ExternalProxyType {
    ProxyBroker2,
    ElfProxy,
    NyxProxy,
    GoProxy6,
    OmniProx,
    Rotato,
    AiEdge,
    WebSearchMcpCloak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ExternalProxyProvider {
    pub id: String,
    pub name: String,
    pub provider_type: ExternalProxyType,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub enabled: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ProxyPoolStats {
    pub total_proxies_available: AtomicU64,
    pub successful_requests: AtomicU64,
    pub failed_requests: AtomicU64,
    pub rotations_performed: AtomicU64,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ProxyPool {
    rotator: Arc<RwLock<ProxyRotator>>,
    external_providers: RwLock<Vec<ExternalProxyProvider>>,
    stats: ProxyPoolStats,
}

#[allow(dead_code)]
impl ProxyPool {
    pub fn new(rotator: Arc<RwLock<ProxyRotator>>) -> Self {
        Self {
            rotator,
            external_providers: RwLock::new(Vec::new()),
            stats: ProxyPoolStats {
                total_proxies_available: AtomicU64::new(0),
                successful_requests: AtomicU64::new(0),
                failed_requests: AtomicU64::new(0),
                rotations_performed: AtomicU64::new(0),
            },
        }
    }

    pub async fn add_provider(&self, provider: ExternalProxyProvider) {
        let mut providers = self.external_providers.write().await;
        providers.push(provider);
    }

    pub async fn remove_provider(&self, id: &str) -> bool {
        let mut providers = self.external_providers.write().await;
        let len_before = providers.len();
        providers.retain(|p| p.id != id);
        providers.len() != len_before
    }

    pub async fn enable_provider(&self, id: &str) -> bool {
        let mut providers = self.external_providers.write().await;
        if let Some(provider) = providers.iter_mut().find(|p| p.id == id) {
            provider.enabled = true;
            true
        } else {
            false
        }
    }

    pub async fn disable_provider(&self, id: &str) -> bool {
        let mut providers = self.external_providers.write().await;
        if let Some(provider) = providers.iter_mut().find(|p| p.id == id) {
            provider.enabled = false;
            true
        } else {
            false
        }
    }

    pub async fn list_providers(&self) -> Vec<ExternalProxyProvider> {
        self.external_providers.read().await.clone()
    }

    pub async fn get_provider(&self, id: &str) -> Option<ExternalProxyProvider> {
        let providers = self.external_providers.read().await;
        providers.iter().find(|p| p.id == id).cloned()
    }

    pub async fn health_check(&self, id: &str) -> Result<bool, String> {
        let provider = {
            let providers = self.external_providers.read().await;
            providers.iter().find(|p| p.id == id).cloned()
        };
        let provider = provider.ok_or_else(|| format!("provider '{}' not found", id))?;
        if !provider.enabled {
            return Err("provider is disabled".to_string());
        }
        match reqwest::get(&provider.endpoint).await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => Err(format!("health check failed: {}", e)),
        }
    }

    pub async fn rotate(&self) -> Option<String> {
        let rotator = self.rotator.read().await;
        let url = rotator.next();
        if url.is_some() {
            self.stats.rotations_performed.fetch_add(1, Ordering::SeqCst);
        }
        url
    }

    pub async fn mark_success(&self, url: &str) {
        let rotator = self.rotator.read().await;
        rotator.mark_success(url);
        self.stats.successful_requests.fetch_add(1, Ordering::SeqCst);
    }

    pub async fn mark_failed(&self, url: &str) {
        let rotator = self.rotator.read().await;
        rotator.mark_failed(url);
        let _fails = self.stats.failed_requests.fetch_add(1, Ordering::SeqCst);
    }

    pub async fn active_proxy_count(&self) -> usize {
        let rotator = self.rotator.read().await;
        rotator.active_count()
    }

    pub fn stats(&self) -> &ProxyPoolStats {
        &self.stats
    }

    pub async fn sync_providers_to_rotator(&self) {
        let providers = self.external_providers.read().await;
        let rotator = self.rotator.read().await;
        for provider in providers.iter().filter(|p| p.enabled) {
            rotator.add_proxy(provider.endpoint.clone(), None, None, Some(provider.id.clone()));
        }
        self.stats
            .total_proxies_available
            .store(rotator.list_proxies().len() as u64, Ordering::SeqCst);
    }
}
