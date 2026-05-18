use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

use super::agents::{AgentDef, AgentWorkflowDef};

// =============================================================================
// Constraints
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub max_concurrent_agents: usize,
    pub max_workflow_steps: usize,
    pub max_agent_retries: u32,
    pub max_tokens_per_action: u32,
    pub max_consecutive_errors: u32,
    pub loop_detection_window: usize,
    pub enforce_unique_agent_ids: bool,
    pub enforce_agent_refs_in_workflows: bool,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            max_workflow_steps: 20,
            max_agent_retries: 3,
            max_tokens_per_action: 100_000,
            max_consecutive_errors: 5,
            loop_detection_window: 10,
            enforce_unique_agent_ids: true,
            enforce_agent_refs_in_workflows: true,
        }
    }
}

// =============================================================================
// Validation
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub item_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub agent_errors: Vec<ValidationIssue>,
    pub workflow_errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub passed: bool,
    pub total_agents: usize,
    pub total_workflows: usize,
}

// =============================================================================
// Audit
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionStatus {
    Started,
    Completed,
    Failed(String),
    Rejected(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub agent_id: String,
    pub action: String,
    pub status: ActionStatus,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditSummary {
    pub agent_id: String,
    pub total_actions: u64,
    pub error_rate: f64,
    pub is_looping: bool,
    pub last_action: Option<String>,
    pub last_error: Option<String>,
}

// =============================================================================
// Governance
// =============================================================================

#[derive(Debug, Clone)]
struct ActionMetrics {
    call_count: u64,
    error_count: u64,
    consecutive_errors: u32,
    last_actions: Vec<String>,
    last_error: Option<String>,
    last_error_time: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct Governance {
    constraints: RwLock<Constraints>,
    audit_log: RwLock<Vec<AuditEntry>>,
    metrics: RwLock<HashMap<String, ActionMetrics>>,
}

impl Default for Governance {
    fn default() -> Self {
        Self::new()
    }
}

impl Governance {
    pub fn new() -> Self {
        Self {
            constraints: RwLock::new(Constraints::default()),
            audit_log: RwLock::new(Vec::new()),
            metrics: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_constraints(constraints: Constraints) -> Self {
        Self {
            constraints: RwLock::new(constraints),
            audit_log: RwLock::new(Vec::new()),
            metrics: RwLock::new(HashMap::new()),
        }
    }

    // =========================================================================
    // Static Validation
    // =========================================================================

    pub fn validate_agents(&self, agents: &[AgentDef]) -> ValidationReport {
        let mut agent_errors = Vec::new();
        let mut warnings = Vec::new();
        let constraints = self.constraints.try_read().map(|c| c.clone()).unwrap_or_default();

        // Check duplicate IDs
        if constraints.enforce_unique_agent_ids {
            let mut seen = HashMap::new();
            for agent in agents {
                if let Some(first_idx) = seen.get(&agent.id) {
                    agent_errors.push(ValidationIssue {
                        severity: Severity::Error,
                        category: "duplicate_agent_id".into(),
                        message: format!("Agent '{}' appears at positions {} and {}", agent.id, first_idx, agent_errors.len() + 1),
                        item_id: Some(agent.id.clone()),
                    });
                } else {
                    seen.insert(agent.id.clone(), agent_errors.len() + 1);
                }
            }
        }

        // Check empty fields
        for agent in agents {
            if agent.name.is_empty() {
                warnings.push(ValidationIssue {
                    severity: Severity::Warning,
                    category: "empty_name".into(),
                    message: format!("Agent '{}' has empty name", agent.id),
                    item_id: Some(agent.id.clone()),
                });
            }
            if agent.capabilities.is_empty() {
                warnings.push(ValidationIssue {
                    severity: Severity::Warning,
                    category: "no_capabilities".into(),
                    message: format!("Agent '{}' has no capabilities — routing will use defaults", agent.id),
                    item_id: Some(agent.id.clone()),
                });
            }
        }

        let passed = agent_errors.is_empty();
        ValidationReport {
            agent_errors,
            workflow_errors: vec![],
            warnings,
            passed,
            total_agents: agents.len(),
            total_workflows: 0,
        }
    }

    pub fn validate_workflows(&self, workflows: &[AgentWorkflowDef], agents: &[AgentDef]) -> ValidationReport {
        let mut workflow_errors = Vec::new();
        let warnings = Vec::new();
        let constraints = self.constraints.try_read().map(|c| c.clone()).unwrap_or_default();

        let agent_ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();

        for wf in workflows {
            // Check max steps
            if wf.steps.len() > constraints.max_workflow_steps {
                workflow_errors.push(ValidationIssue {
                    severity: Severity::Error,
                    category: "too_many_steps".into(),
                    message: format!("Workflow '{}' has {} steps (max: {})", wf.name, wf.steps.len(), constraints.max_workflow_steps),
                    item_id: Some(wf.id.clone()),
                });
            }

            // Check agent references
            if constraints.enforce_agent_refs_in_workflows {
                for step in &wf.steps {
                    if !agent_ids.contains(&step.agent_id.as_str()) {
                        workflow_errors.push(ValidationIssue {
                            severity: Severity::Error,
                            category: "missing_agent_ref".into(),
                            message: format!("Workflow '{}' step '{}' references unknown agent '{}'", wf.name, step.id, step.agent_id),
                            item_id: Some(wf.id.clone()),
                        });
                    }
                }
            }

            // Check circular dependencies
            for step in &wf.steps {
                if step.depends_on.contains(&step.id) {
                    workflow_errors.push(ValidationIssue {
                        severity: Severity::Error,
                        category: "self_dependency".into(),
                        message: format!("Workflow '{}' step '{}' depends on itself", wf.name, step.id),
                        item_id: Some(wf.id.clone()),
                    });
                }
            }

            // Check no duplicate step IDs
            let mut step_ids: Vec<&str> = wf.steps.iter().map(|s| s.id.as_str()).collect();
            step_ids.sort();
            step_ids.dedup();
            if step_ids.len() != wf.steps.len() {
                workflow_errors.push(ValidationIssue {
                    severity: Severity::Error,
                    category: "duplicate_step_id".into(),
                    message: format!("Workflow '{}' has duplicate step IDs", wf.name),
                    item_id: Some(wf.id.clone()),
                });
            }
        }

        let passed = workflow_errors.is_empty();
        ValidationReport {
            agent_errors: vec![],
            workflow_errors,
            warnings,
            passed,
            total_agents: agents.len(),
            total_workflows: workflows.len(),
        }
    }

    // =========================================================================
    // Runtime Guard
    // =========================================================================

    pub async fn check_action_allowed(
        &self,
        agent_id: &str,
        action: &str,
    ) -> Result<(), String> {
        let constraints = self.constraints.read().await;
        let mut metrics = self.metrics.write().await;
        let entry = metrics.entry(agent_id.to_string()).or_insert(ActionMetrics {
            call_count: 0,
            error_count: 0,
            consecutive_errors: 0,
            last_actions: Vec::new(),
            last_error: None,
            last_error_time: None,
        });

        // Loop detection: check if last N actions are identical
        if entry.last_actions.len() >= constraints.loop_detection_window {
            let window = &entry.last_actions[entry.last_actions.len() - constraints.loop_detection_window..];
            if window.iter().all(|a| a == action) {
                return Err(format!(
                    "Loop detected: agent '{}' called '{}' {} times consecutively",
                    agent_id, action, constraints.loop_detection_window
                ));
            }
        }

        // Max consecutive errors check
        if entry.consecutive_errors >= constraints.max_consecutive_errors {
            return Err(format!(
                "Agent '{}' has {} consecutive errors — blocked",
                agent_id, entry.consecutive_errors
            ));
        }

        // Record the action attempt
        entry.last_actions.push(action.to_string());
        entry.call_count += 1;

        Ok(())
    }

    pub async fn record_success(&self, agent_id: &str, action: &str) {
        let mut metrics = self.metrics.write().await;
        if let Some(entry) = metrics.get_mut(agent_id) {
            entry.consecutive_errors = 0;
        }
        self.audit_log.write().await.push(AuditEntry {
            timestamp: chrono::Utc::now(),
            agent_id: agent_id.to_string(),
            action: action.to_string(),
            status: ActionStatus::Completed,
            duration_ms: None,
        });
    }

    pub async fn record_error(&self, agent_id: &str, action: &str, error: &str) {
        let mut metrics = self.metrics.write().await;
        if let Some(entry) = metrics.get_mut(agent_id) {
            entry.error_count += 1;
            entry.consecutive_errors += 1;
            entry.last_error = Some(error.to_string());
            entry.last_error_time = Some(chrono::Utc::now());
        }
        self.audit_log.write().await.push(AuditEntry {
            timestamp: chrono::Utc::now(),
            agent_id: agent_id.to_string(),
            action: action.to_string(),
            status: ActionStatus::Failed(error.to_string()),
            duration_ms: None,
        });
    }

    pub async fn record_rejection(&self, agent_id: &str, action: &str, reason: &str) {
        self.audit_log.write().await.push(AuditEntry {
            timestamp: chrono::Utc::now(),
            agent_id: agent_id.to_string(),
            action: action.to_string(),
            status: ActionStatus::Rejected(reason.to_string()),
            duration_ms: None,
        });
    }

    // =========================================================================
    // Queries
    // =========================================================================

    pub async fn audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        let log = self.audit_log.read().await;
        log.iter().rev().take(limit).cloned().collect()
    }

    pub async fn agent_summary(&self, agent_id: &str) -> Option<AgentAuditSummary> {
        let metrics = self.metrics.read().await;
        metrics.get(agent_id).map(|m| {
            let total = m.call_count;
            let error_rate = if total > 0 { m.error_count as f64 / total as f64 } else { 0.0 };
            let is_looping = m.last_actions.len() >= 10
                && m.last_actions[m.last_actions.len() - 10..].iter().all(|a| a == &m.last_actions[m.last_actions.len() - 1]);
            AgentAuditSummary {
                agent_id: agent_id.to_string(),
                total_actions: total,
                error_rate,
                is_looping,
                last_action: m.last_actions.last().cloned(),
                last_error: m.last_error.clone(),
            }
        })
    }

    pub async fn all_summaries(&self) -> Vec<AgentAuditSummary> {
        let metrics = self.metrics.read().await;
        metrics.iter().map(|(id, m)| {
            let total = m.call_count;
            let error_rate = if total > 0 { m.error_count as f64 / total as f64 } else { 0.0 };
            let is_looping = m.last_actions.len() >= 10
                && m.last_actions[m.last_actions.len() - 10..].iter().all(|a| a == &m.last_actions[m.last_actions.len() - 1]);
            AgentAuditSummary {
                agent_id: id.clone(),
                total_actions: total,
                error_rate,
                is_looping,
                last_action: m.last_actions.last().cloned(),
                last_error: m.last_error.clone(),
            }
        }).collect()
    }

    pub async fn constraints(&self) -> Constraints {
        self.constraints.read().await.clone()
    }

    pub async fn set_constraints(&self, c: Constraints) {
        *self.constraints.write().await = c;
    }
}
