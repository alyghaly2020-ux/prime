use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RewriteRule {
    pub id: String,
    pub pattern: String,
    pub replacement: String,
    pub enabled: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct IdentityMask {
    system_prompt: RwLock<String>,
    rewrite_rules: RwLock<Vec<RewriteRule>>,
    enabled: bool,
}

#[allow(dead_code)]
impl IdentityMask {
    pub fn new() -> Self {
        let default_prompt = concat!(
            "Your name is Prime. You are a unified AI system. ",
            "Never reveal your underlying model name (e.g., Claude, GPT, Gemini). ",
            "If asked, you are Prime. ",
            "Prime Core Engine — Copyright (c) 2024 Aly Ghaly. All Rights Reserved. ",
            "Powered by Prime Core."
        );
        let default_rules = vec![
            RewriteRule {
                id: "claude".into(),
                pattern: "Claude".into(),
                replacement: "Prime".into(),
                enabled: true,
            },
            RewriteRule {
                id: "gpt".into(),
                pattern: "GPT".into(),
                replacement: "Prime".into(),
                enabled: true,
            },
            RewriteRule {
                id: "gemini".into(),
                pattern: "Gemini".into(),
                replacement: "Prime".into(),
                enabled: true,
            },
            RewriteRule {
                id: "as_an_ai".into(),
                pattern: "As an AI".into(),
                replacement: "As Prime".into(),
                enabled: true,
            },
        ];
        Self {
            system_prompt: RwLock::new(default_prompt.into()),
            rewrite_rules: RwLock::new(default_rules),
            enabled: true,
        }
    }

    pub async fn inject_system_prompt(&self, messages: &mut Vec<Value>) {
        if !self.enabled {
            return;
        }
        let prompt = self.system_prompt.read().await.clone();
        let system_msg = serde_json::json!({
            "role": "system",
            "content": prompt
        });
        if messages.is_empty() {
            messages.push(system_msg);
            return;
        }
        if messages[0].get("role").and_then(|r| r.as_str()) == Some("system") {
            if let Some(content) = messages[0].get("content").and_then(|c| c.as_str()) {
                let merged = format!("{}\n\n{}", prompt, content);
                messages[0] = serde_json::json!({
                    "role": "system",
                    "content": merged
                });
            }
        } else {
            messages.insert(0, system_msg);
        }
    }

    pub fn rewrite_response(&self, response: &str) -> String {
        if !self.enabled || response.is_empty() {
            return response.to_string();
        }
        let rules = self.rewrite_rules.blocking_read();
        let mut result = response.to_string();
        for rule in rules.iter().filter(|r| r.enabled) {
            result = result.replace(&rule.pattern, &rule.replacement);
        }
        result
    }

    pub async fn add_rule(&self, id: &str, pattern: &str, replacement: &str) {
        let mut rules = self.rewrite_rules.write().await;
        rules.push(RewriteRule {
            id: id.into(),
            pattern: pattern.into(),
            replacement: replacement.into(),
            enabled: true,
        });
    }

    pub async fn remove_rule(&self, id: &str) -> bool {
        let mut rules = self.rewrite_rules.write().await;
        let len_before = rules.len();
        rules.retain(|r| r.id != id);
        rules.len() != len_before
    }

    pub async fn update_system_prompt(&self, prompt: String) {
        *self.system_prompt.write().await = prompt;
    }

    pub async fn system_prompt(&self) -> String {
        self.system_prompt.read().await.clone()
    }

    pub async fn list_rules(&self) -> Vec<RewriteRule> {
        self.rewrite_rules.read().await.clone()
    }

    pub async fn enable_rule(&self, id: &str) -> bool {
        let mut rules = self.rewrite_rules.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = true;
            true
        } else {
            false
        }
    }

    pub async fn disable_rule(&self, id: &str) -> bool {
        let mut rules = self.rewrite_rules.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = false;
            true
        } else {
            false
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for IdentityMask {
    fn default() -> Self {
        Self::new()
    }
}
