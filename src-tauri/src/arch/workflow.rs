use super::event_bus::EventBusCore;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub action: String,
    pub params: serde_json::Value,
    pub retry_count: u32,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDef {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub edges: Vec<(usize, usize)>, // (from_step, to_step)
}

pub struct WorkflowEngine {
    workflows: RwLock<HashMap<String, WorkflowDef>>,
    running: RwLock<HashMap<String, WorkflowInstance>>,
    bus: Arc<EventBusCore>,
}

#[allow(dead_code)]
struct WorkflowInstance {
    def: WorkflowDef,
    status: String,
    current_step: usize,
    results: HashMap<String, serde_json::Value>,
}

impl WorkflowEngine {
    pub fn new(bus: Arc<EventBusCore>) -> Self {
        Self {
            workflows: RwLock::new(HashMap::new()),
            running: RwLock::new(HashMap::new()),
            bus,
        }
    }

    pub async fn register(&self, workflow: WorkflowDef) {
        self.workflows
            .write()
            .await
            .insert(workflow.id.clone(), workflow);
    }

    pub async fn execute(&self, workflow_id: &str) -> anyhow::Result<String> {
        let workflows = self.workflows.read().await;
        let def = workflows
            .get(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?
            .clone();
        drop(workflows);

        let instance_id = uuid::Uuid::new_v4().to_string();
        let instance = WorkflowInstance {
            def: def.clone(),
            status: "running".to_string(),
            current_step: 0,
            results: HashMap::new(),
        };

        self.running
            .write()
            .await
            .insert(instance_id.clone(), instance);
        tracing::info!("Workflow started: {} (id: {})", def.name, instance_id);

        // Build DAG and execute in order
        let mut graph = DiGraph::<&WorkflowStep, ()>::new();
        let indices: HashMap<&str, NodeIndex> = def
            .steps
            .iter()
            .map(|step| (step.id.as_str(), graph.add_node(step)))
            .collect();

        for (from, to) in &def.edges {
            if let (Some(from_idx), Some(to_idx)) = (
                def.steps
                    .get(*from)
                    .and_then(|s| indices.get(s.id.as_str())),
                def.steps.get(*to).and_then(|s| indices.get(s.id.as_str())),
            ) {
                graph.add_edge(*from_idx, *to_idx, ());
            }
        }

        // Execute steps in topological order via event bus
        let bus = self.bus.clone();
        let instance_id_clone = instance_id.clone();
        let steps = def.steps.clone();
        let edges = def.edges.clone();
        tokio::spawn(async move {
            use petgraph::algo::toposort;
            let mut graph = DiGraph::<WorkflowStep, ()>::new();
            let indices: HashMap<usize, NodeIndex> = steps
                .iter()
                .enumerate()
                .map(|(i, step)| (i, graph.add_node(step.clone())))
                .collect();

            for (from, to) in &edges {
                if let (Some(from_idx), Some(to_idx)) = (indices.get(from), indices.get(to)) {
                    graph.add_edge(*from_idx, *to_idx, ());
                }
            }

            if let Ok(sorted) = toposort(&graph, None) {
                for node_idx in sorted {
                    let step = &graph[node_idx];
                    tracing::info!("Workflow {}: executing step '{}' (action: {})", 
                        instance_id_clone, step.id, step.action);
                    
                    let event = super::event_bus::SystemEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        event_type: format!("workflow.step.{}", step.action),
                        source: format!("workflow:{}", instance_id_clone),
                        payload: serde_json::json!({
                            "step_id": step.id,
                            "step_name": step.name,
                            "action": step.action,
                            "params": step.params,
                        }),
                        timestamp: chrono::Utc::now(),
                    };
                    bus.emit(event).await;
                }
            }
        });

        Ok(instance_id)
    }

    pub async fn list_workflows(&self) -> Vec<serde_json::Value> {
        use serde_json::json;
        let workflows = self.workflows.read().await;
        let mut result = Vec::new();
        for (id, def) in workflows.iter() {
            let status = if self.running.read().await.contains_key(id) {
                "running"
            } else {
                "idle"
            };
            result.push(json!({
                "id": id,
                "name": def.name,
                "description": "",
                "status": status,
                "steps": def.steps.iter().map(|s| json!({
                    "id": s.id,
                    "name": s.name,
                    "status": "pending"
                })).collect::<Vec<_>>(),
                "created_at": 0,
                "progress_pct": 0,
                "dag": []
            }));
        }
        result
    }

    pub async fn start_workflow(&self, id: &str) -> Result<(), String> {
        self.execute(id).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn cancel_workflow(&self, id: &str) -> Result<(), String> {
        let mut running = self.running.write().await;
        running.remove(id);
        Ok(())
    }

    pub async fn pause_workflow(&self, id: &str) -> Result<(), String> {
        let mut running = self.running.write().await;
        if let Some(instance) = running.get_mut(id) {
            instance.status = "paused".to_string();
            Ok(())
        } else {
            Err(format!("Workflow not running: {}", id))
        }
    }

    pub async fn resume_workflow(&self, id: &str) -> Result<(), String> {
        let mut running = self.running.write().await;
        if let Some(instance) = running.get_mut(id) {
            instance.status = "running".to_string();
            Ok(())
        } else {
            Err(format!("Workflow not running: {}", id))
        }
    }

    pub async fn status(&self, id: &str) -> Option<String> {
        self.running.read().await.get(id).map(|i| i.status.clone())
    }
}
