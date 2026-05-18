//! # Supervisor Heartbeat System (Internal Observer)
//!
//! Monitors AI agent execution in real time to detect and correct:
//! - **Loop detection**: repeated text/actions ≥3 times
//! - **Stall detection**: same state >30 seconds
//! - **Hallucination detection**: output drifting from stated objective
//! - **Token explosion**: output growing exponentially
//!
//! Architecture:
//! ```text
//! Agent ──(mpsc Sender)──▶ Supervisor ──(watchdog timer)──▶ Intervention
//!   ▲                                                        │
//!   └──────────────────── (AbortHandle) ─────────────────────┘
//! ```
//!
//! Prime Core Engine — Copyright (c) 2024 Aly Ghaly. All Rights Reserved.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

// ============================================================================
// Heartbeat — the tick signal from agent → supervisor
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AgentState {
    Starting,
    Thinking,
    ExecutingTool,
    GeneratingOutput,
    WaitingInput,
    Completed,
    Error(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Heartbeat {
    pub agent_id: String,
    pub state: AgentState,
    pub thought: Option<String>,
    pub output: Option<String>,
    pub step: u64,
    pub elapsed_ms: u64,
    pub tokens_generated: u64,
    pub model: Option<String>,
    pub objective: Option<String>,
}

impl Heartbeat {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            state: AgentState::Starting,
            thought: None,
            output: None,
            step: 0,
            elapsed_ms: 0,
            tokens_generated: 0,
            model: None,
            objective: None,
        }
    }

    pub fn with_state(mut self, state: AgentState) -> Self {
        self.state = state;
        self
    }

    pub fn with_thought(mut self, thought: impl Into<String>) -> Self {
        self.thought = Some(thought.into());
        self
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }

    pub fn with_step(mut self, step: u64) -> Self {
        self.step = step;
        self
    }

    pub fn with_elapsed(mut self, elapsed_ms: u64) -> Self {
        self.elapsed_ms = elapsed_ms;
        self
    }

    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens_generated = tokens;
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_objective(mut self, objective: impl Into<String>) -> Self {
        self.objective = Some(objective.into());
        self
    }
}

// ============================================================================
// Detected Issue Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueType {
    /// Agent repeating same thought/output ≥ threshold times
    Loop,
    /// Agent stuck at same step for > stall_timeout
    Stall,
    /// Output diverging from stated objective (heuristic)
    Hallucination,
    /// Token count growing exponentially (unlimited budget)
    TokenExplosion,
    /// Time exceeded max allowed
    Timeout,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Issue {
    pub issue_type: IssueType,
    pub agent_id: String,
    pub detail: String,
    pub occurred_at: String,
    pub severity: u8, // 1-10
}

// ============================================================================
// Intervention Actions
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Intervention {
    /// Clear last N tokens of context, inject correction prompt
    ResetContext { keep_turns: u64, correction: String },
    /// Instruct agent to re-focus on original objective
    CorrectionPrompt(String),
    /// Switch to a different model to break the loop
    SwitchModel(String),
    /// Force-kill the agent
    Kill(String),
    /// Log warning but take no action
    WarnOnly(String),
}

// ============================================================================
// Supervisor Configuration
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupervisorConfig {
    /// Heartbeat interval in milliseconds (how often agent should send tick)
    pub heartbeat_interval_ms: u64,
    /// Max time without heartbeat before watchdog triggers
    pub watchdog_timeout_ms: u64,
    /// How many repeated identical thoughts/outputs before loop detection
    pub loop_threshold: usize,
    /// Max time in same step before stall detection
    pub stall_timeout_ms: u64,
    /// Max tokens allowed before token explosion detection
    pub max_tokens: u64,
    /// Max total agent run time
    pub max_run_time_ms: u64,
    /// How many recent heartbeats to keep in ring buffer
    pub history_size: usize,
    /// Whether to auto-intervene (vs just log)
    pub auto_intervene: bool,
    /// Fallback model to switch to on persistent issues
    pub fallback_model: Option<String>,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 2000,
            watchdog_timeout_ms: 15_000,
            loop_threshold: 3,
            stall_timeout_ms: 30_000,
            max_tokens: 32_000,
            max_run_time_ms: 300_000, // 5 minutes
            history_size: 50,
            auto_intervene: true,
            fallback_model: Some("gpt-4".into()),
        }
    }
}

// ============================================================================
// Heartbeat History (Ring Buffer per Agent)
// ============================================================================

#[allow(dead_code)]
#[derive(Debug)]
struct AgentHistory {
    agent_id: String,
    heartbeats: VecDeque<Heartbeat>,
    max_len: usize,
    stall_start: Option<Instant>,
    last_step: u64,
    intervention_count: u64,
}

impl AgentHistory {
    fn new(agent_id: String, max_len: usize) -> Self {
        Self {
            agent_id,
            heartbeats: VecDeque::with_capacity(max_len + 1),
            max_len,
            stall_start: None,
            last_step: 0,
            intervention_count: 0,
        }
    }

    fn push(&mut self, hb: Heartbeat) {
        if self.heartbeats.len() >= self.max_len {
            self.heartbeats.pop_front();
        }
        // Check if step changed — reset stall timer
        if hb.step != self.last_step {
            self.stall_start = Some(Instant::now());
            self.last_step = hb.step;
        }
        self.heartbeats.push_back(hb);
    }

    /// Detect loops by checking last N outputs for repetition
    fn detect_loop(&self, threshold: usize) -> Option<String> {
        let count = self.heartbeats.len();
        if count < threshold {
            return None;
        }

        // Check last threshold outputs for repetition
        let recent: Vec<_> = self
            .heartbeats
            .iter()
            .rev()
            .take(threshold)
            .collect();

        if recent.len() < threshold {
            return None;
        }

        // Check if all recent outputs are identical (or nearly identical)
        let outputs: Vec<Option<&str>> = recent.iter().map(|h| h.output.as_deref()).collect();
        let thoughts: Vec<Option<&str>> = recent.iter().map(|h| h.thought.as_deref()).collect();

        // Output loop
        if outputs.iter().all(|o| o.is_some() && *o == outputs[0]) {
            if let Some(text) = outputs[0] {
                if !text.is_empty() {
                    return Some(format!("Output loop detected: last {} outputs are identical", threshold));
                }
            }
        }

        // Thought loop
        if thoughts.iter().all(|t| t.is_some() && *t == thoughts[0]) {
            if let Some(text) = thoughts[0] {
                if !text.is_empty() {
                    return Some(format!("Thought loop detected: last {} thoughts are identical", threshold));
                }
            }
        }

        None
    }

    /// Detect stall (same step for too long)
    fn detect_stall(&self, timeout: Duration) -> Option<String> {
        if let Some(start) = self.stall_start {
            if start.elapsed() >= timeout {
                return Some(format!(
                    "Stall detected: step {} unchanged for {:?}",
                    self.last_step,
                    start.elapsed()
                ));
            }
        }
        None
    }

    /// Detect hallucination (output drifts from objective)
    fn detect_hallucination(&self) -> Option<String> {
        let hbs: Vec<_> = self.heartbeats.iter().collect();
        if hbs.len() < 2 {
            return None;
        }

        // Get the objective from the first heartbeat
        let objective = hbs.first().and_then(|h| h.objective.as_deref())?;
        let latest_output = hbs.last().and_then(|h| h.output.as_deref())?;

        // Simple heuristic: check for output that contradicts or ignores objective
        // (In production, use cosine similarity or LLM-as-judge)
        let objective_lower = objective.to_lowercase();
        let output_lower = latest_output.to_lowercase();

        // If output doesn't contain any key words from objective, flag it
        let key_words: Vec<&str> = objective_lower
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .collect();

        let matches = key_words
            .iter()
            .filter(|w| output_lower.contains(**w))
            .count();

        if key_words.len() >= 3 && matches == 0 {
            return Some(format!(
                "Hallucination suspect: output doesn't reference objective key terms (0/{} matches)",
                key_words.len()
            ));
        }

        None
    }

    /// Detect token explosion
    fn detect_token_explosion(&self, max_tokens: u64) -> Option<String> {
        let hbs: Vec<_> = self.heartbeats.iter().collect();
        if hbs.len() < 3 {
            return None;
        }

        let recent: Vec<_> = hbs.iter().rev().take(3).collect();
        let tokens: Vec<u64> = recent.iter().map(|h| h.tokens_generated).collect();

        // Check if token count is growing exponentially (doubling each step)
        // tokens[0] = most recent, tokens[1] = middle, tokens[2] = oldest
        if tokens.len() == 3 {
            let oldest = tokens[2];
            let middle = tokens[1];
            let newest = tokens[0];
            if oldest > 0 && middle >= oldest * 2 && newest >= middle * 2 {
                return Some(format!(
                    "Token explosion: {} → {} → {} (doubling each step)",
                    newest, middle, oldest
                ));
            }
        }

        // Check hard limit
        if let Some(latest) = tokens.last() {
            if *latest > max_tokens {
                return Some(format!("Token limit exceeded: {} > {}", latest, max_tokens));
            }
        }

        None
    }
}

// ============================================================================
// Supervisor — the core observer
// ============================================================================

#[derive(Debug)]
struct SupervisorInner {
    /// Per-agent heartbeat history
    histories: HashMap<String, AgentHistory>,
    /// Detected issues (not yet resolved)
    active_issues: Vec<Issue>,
    /// Resolved issues
    resolved_issues: Vec<Issue>,
    /// Issue count per agent
    issue_counts: HashMap<String, u64>,
    /// Total interventions performed
    total_interventions: u64,
    /// Current watch loop status
    is_running: bool,
}

/// The Supervisor Heartbeat Monitor.
///
/// # Usage
/// ```ignore
/// let supervisor = Arc::new(Supervisor::new(SupervisorConfig::default()));
/// let (tx, mut rx) = supervisor.channel();
///
/// // Agent sends heartbeats
/// tx.send(Heartbeat::new("agent-1").with_state(AgentState::Thinking)).await;
///
/// // Supervisor processes in background
/// tokio::spawn(supervisor.run(rx));
/// ```
pub struct Supervisor {
    inner: RwLock<SupervisorInner>,
    config: SupervisorConfig,
    /// Signal for watchdog to stop
    shutdown: AtomicBool,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new(SupervisorConfig::default())
    }
}

impl Supervisor {
    pub fn new(config: SupervisorConfig) -> Self {
        Self {
            inner: RwLock::new(SupervisorInner {
                histories: HashMap::new(),
                active_issues: Vec::new(),
                resolved_issues: Vec::new(),
                issue_counts: HashMap::new(),
                total_interventions: 0,
                is_running: false,
            }),
            config,
            shutdown: AtomicBool::new(false),
        }
    }

    /// Create a sender-receiver channel for heartbeat communication.
    /// The agent holds the Sender, the supervisor holds the Receiver.
    pub fn channel() -> (mpsc::Sender<Heartbeat>, mpsc::Receiver<Heartbeat>) {
        mpsc::channel(1024)
    }

    /// Run the supervisor loop.
    /// Processes heartbeats AND runs watchdog timers.
    pub async fn run(
        self: Arc<Self>,
        mut rx: mpsc::Receiver<Heartbeat>,
    ) -> Vec<Issue> {
        {
            let mut inner = self.inner.write().await;
            inner.is_running = true;
        }

        tracing::info!("Supervisor started with config: {:?}", self.config);

        let mut issues_detected = Vec::new();
        let watchdog_interval = Duration::from_millis(self.config.watchdog_timeout_ms / 2);

        loop {
            tokio::select! {
                // Process incoming heartbeats
                Some(hb) = rx.recv() => {
                    if let Some(issue) = self.process_heartbeat(hb).await {
                        tracing::warn!("Supervisor detected issue: {:?}", issue);
                        issues_detected.push(issue.clone());
                    }
                }
                // Watchdog timer
                _ = tokio::time::sleep(watchdog_interval) => {
                    if let Some(issue) = self.run_watchdog().await {
                        tracing::warn!("Supervisor watchdog triggered: {:?}", issue);
                        issues_detected.push(issue.clone());
                    }
                }
            }

            // Check shutdown signal
            if self.shutdown.load(Ordering::SeqCst) {
                let mut inner = self.inner.write().await;
                inner.is_running = false;
                break;
            }
        }

        issues_detected
    }

    /// Process a single heartbeat — detect issues, return any found.
    async fn process_heartbeat(&self, hb: Heartbeat) -> Option<Issue> {
        let agent_id = hb.agent_id.clone();

        // Ensure history exists
        {
            let mut inner = self.inner.write().await;
            inner
                .histories
                .entry(agent_id.clone())
                .or_insert_with(|| AgentHistory::new(agent_id.clone(), self.config.history_size));
        }

        // Push to history
        {
            let mut inner = self.inner.write().await;
            if let Some(history) = inner.histories.get_mut(&agent_id) {
                history.push(hb);
            }
        }

        // Run all detectors
        let (loop_issue, stall_issue, hall_issue, tok_issue, _time_issue) = {
            let inner = self.inner.read().await;
            let history = inner.histories.get(&agent_id)?;

            let loop_issue = history.detect_loop(self.config.loop_threshold);
            let stall_issue =
                history.detect_stall(Duration::from_millis(self.config.stall_timeout_ms));
            let hall_issue = history.detect_hallucination();
            let tok_issue = history.detect_token_explosion(self.config.max_tokens);
            let time_issue: Option<String> = None;

            (loop_issue, stall_issue, hall_issue, tok_issue, time_issue)
        };

        // Convert to Issue + Intervention
        if let Some(detail) = loop_issue {
            return Some(self.register_issue(
                &agent_id,
                IssueType::Loop,
                detail,
                7,
            ).await);
        }

        if let Some(detail) = stall_issue {
            return Some(self.register_issue(
                &agent_id,
                IssueType::Stall,
                detail,
                6,
            ).await);
        }

        if let Some(detail) = hall_issue {
            return Some(self.register_issue(
                &agent_id,
                IssueType::Hallucination,
                detail,
                8,
            ).await);
        }

        if let Some(detail) = tok_issue {
            return Some(self.register_issue(
                &agent_id,
                IssueType::TokenExplosion,
                detail,
                9,
            ).await);
        }

        None
    }

    /// Run watchdog — check for missing heartbeats and time limits.
    async fn run_watchdog(&self) -> Option<Issue> {
        let agents_to_check: Vec<String> = {
            self.inner
                .read()
                .await
                .histories
                .keys()
                .cloned()
                .collect()
        };

        let now = Instant::now();

        for agent_id in agents_to_check {
            let should_timeout = {
                let inner = self.inner.read().await;
                let history = inner.histories.get(&agent_id)?;
                let latest = history.heartbeats.back()?;

                // Check time since last heartbeat
                let elapsed_since_hb = Duration::from_millis(latest.elapsed_ms);
                if now.elapsed().checked_sub(elapsed_since_hb).unwrap_or(Duration::ZERO)
                    > Duration::from_millis(self.config.watchdog_timeout_ms)
                {
                    Some((5, format!(
                        "No heartbeat received within watchdog timeout ({}ms)",
                        self.config.watchdog_timeout_ms
                    )))
                } else if latest.elapsed_ms > self.config.max_run_time_ms {
                    Some((10, format!(
                        "Agent exceeded max run time: {}ms > {}ms",
                        latest.elapsed_ms, self.config.max_run_time_ms
                    )))
                } else {
                    None
                }
            }; // `inner`, `history`, `latest` all dropped here

            if let Some((severity, detail)) = should_timeout {
                return Some(
                    self.register_issue(&agent_id, IssueType::Timeout, detail, severity).await,
                );
            }
        }

        None
    }

    /// Register an issue and potentially intervene.
    async fn register_issue(
        &self,
        agent_id: &str,
        issue_type: IssueType,
        detail: String,
        severity: u8,
    ) -> Issue {
        let issue = Issue {
            issue_type,
            agent_id: agent_id.to_string(),
            detail,
            occurred_at: chrono::Utc::now().to_rfc3339(),
            severity,
        };

        let mut inner = self.inner.write().await;
        inner.active_issues.push(issue.clone());
        *inner.issue_counts.entry(agent_id.to_string()).or_insert(0) += 1;
        inner.total_interventions += 1;

        tracing::warn!(
            "[Supervisor] Issue #{} | agent={} | type={:?} | severity={} | detail={}",
            inner.total_interventions,
            agent_id,
            issue.issue_type,
            severity,
            issue.detail
        );

        issue
    }

    /// Generate correction prompt for an issue.
    pub fn build_correction_prompt(issue: &Issue) -> String {
        match issue.issue_type {
            IssueType::Loop => format!(
                "[SYSTEM CORRECTION] Detected loop: you are repeating the same output.\n\
                 Issue: {}\n\
                 Action: Reset context, return to original objective, and try a different approach.\n\
                 Do NOT repeat previous steps.",
                issue.detail
            ),
            IssueType::Stall => format!(
                "[SYSTEM CORRECTION] Detected stall: no progress on current step.\n\
                 Issue: {}\n\
                 Action: Skip the current approach and try an alternative method.\n\
                 Do NOT remain stuck.",
                issue.detail
            ),
            IssueType::Hallucination => format!(
                "[SYSTEM CORRECTION] Detected hallucination: output straying from objective.\n\
                 Issue: {}\n\
                 Action: Re-read the original objective and refocus. Only produce output that \
                 directly addresses the objective.",
                issue.detail
            ),
            IssueType::TokenExplosion => format!(
                "[SYSTEM CORRECTION] Token explosion detected: output growing exponentially.\n\
                 Issue: {}\n\
                 Action: Be concise. Limit output to essential information only.",
                issue.detail
            ),
            IssueType::Timeout => format!(
                "[SYSTEM CORRECTION] Time limit approaching or exceeded.\n\
                 Issue: {}\n\
                 Action: Wrap up current task immediately. Provide a summary of what was done \
                 and what remains.",
                issue.detail
            ),
        }
    }

    /// Resolve an issue manually.
    pub async fn resolve_issue(&self, issue: &Issue) {
        let mut inner = self.inner.write().await;
        inner.active_issues.retain(|i| {
            i.occurred_at != issue.occurred_at || i.agent_id != issue.agent_id
        });
        inner.resolved_issues.push(issue.clone());
    }

    /// Get all active issues.
    pub async fn active_issues(&self) -> Vec<Issue> {
        self.inner.read().await.active_issues.clone()
    }

    /// Get resolved issues.
    pub async fn resolved_issues(&self) -> Vec<Issue> {
        self.inner.read().await.resolved_issues.clone()
    }

    /// Get issue count for an agent.
    pub async fn issue_count(&self, agent_id: &str) -> u64 {
        self.inner
            .read()
            .await
            .issue_counts
            .get(agent_id)
            .copied()
            .unwrap_or(0)
    }

    /// Get total intervention count.
    pub async fn total_interventions(&self) -> u64 {
        self.inner.read().await.total_interventions
    }

    /// Get total agents tracked.
    pub async fn tracked_agents(&self) -> usize {
        self.inner.read().await.histories.len()
    }

    /// Get supervisor stats.
    pub async fn stats(&self) -> SupervisorStats {
        let inner = self.inner.read().await;
        SupervisorStats {
            tracked_agents: inner.histories.len() as u64,
            active_issues: inner.active_issues.len() as u64,
            resolved_issues: inner.resolved_issues.len() as u64,
            total_interventions: inner.total_interventions,
            is_running: inner.is_running,
        }
    }

    /// Stop the supervisor.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Get config.
    pub fn config(&self) -> SupervisorConfig {
        self.config.clone()
    }
}

// ============================================================================
// Supervisor Stats
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupervisorStats {
    pub tracked_agents: u64,
    pub active_issues: u64,
    pub resolved_issues: u64,
    pub total_interventions: u64,
    pub is_running: bool,
}

// ============================================================================
// SupervisorBuilder — fluent construction
// ============================================================================

pub struct SupervisorBuilder {
    config: SupervisorConfig,
}

impl Default for SupervisorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SupervisorBuilder {
    pub fn new() -> Self {
        Self {
            config: SupervisorConfig::default(),
        }
    }

    pub fn heartbeat_interval(mut self, ms: u64) -> Self {
        self.config.heartbeat_interval_ms = ms;
        self
    }

    pub fn watchdog_timeout(mut self, ms: u64) -> Self {
        self.config.watchdog_timeout_ms = ms;
        self
    }

    pub fn loop_threshold(mut self, n: usize) -> Self {
        self.config.loop_threshold = n;
        self
    }

    pub fn stall_timeout(mut self, ms: u64) -> Self {
        self.config.stall_timeout_ms = ms;
        self
    }

    pub fn max_tokens(mut self, n: u64) -> Self {
        self.config.max_tokens = n;
        self
    }

    pub fn max_run_time(mut self, ms: u64) -> Self {
        self.config.max_run_time_ms = ms;
        self
    }

    pub fn auto_intervene(mut self, enabled: bool) -> Self {
        self.config.auto_intervene = enabled;
        self
    }

    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.config.fallback_model = Some(model.into());
        self
    }

    pub fn build(self) -> Supervisor {
        Supervisor::new(self.config)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_heartbeat_create() {
        let hb = Heartbeat::new("agent-1")
            .with_state(AgentState::Thinking)
            .with_thought("I need to solve this problem")
            .with_output("Let me think about it")
            .with_step(1)
            .with_elapsed(100)
            .with_tokens(50)
            .with_model("gpt-4");

        assert_eq!(hb.agent_id, "agent-1");
        assert_eq!(hb.state, AgentState::Thinking);
        assert_eq!(hb.thought.unwrap(), "I need to solve this problem");
        assert_eq!(hb.step, 1);
        assert_eq!(hb.tokens_generated, 50);
    }

    #[tokio::test]
    async fn test_loop_detection() {
        let mut history = AgentHistory::new("agent-1".into(), 10);
        let output_text = "Hello world";

        // Push 5 identical outputs
        for i in 0..5 {
            history.push(
                Heartbeat::new("agent-1")
                    .with_state(AgentState::GeneratingOutput)
                    .with_output(output_text)
                    .with_step(i),
            );
        }

        let loop_result = history.detect_loop(3);
        assert!(loop_result.is_some(), "Should detect loop");
        let msg = loop_result.unwrap();
        assert!(
            msg.contains("loop"),
            "Loop message should contain 'loop', got: {}",
            msg
        );
    }

    #[tokio::test]
    async fn test_no_loop_with_different_outputs() {
        let mut history = AgentHistory::new("agent-1".into(), 10);

        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_output("First output")
                .with_step(1),
        );
        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_output("Second output")
                .with_step(2),
        );
        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_output("Third output")
                .with_step(3),
        );

        assert!(history.detect_loop(3).is_none());
    }

    #[tokio::test]
    async fn test_hallucination_detection() {
        let mut history = AgentHistory::new("agent-1".into(), 10);
        let objective = "Calculate the fibonacci sequence up to 10 terms";

        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::Starting)
                .with_objective(objective)
                .with_step(0),
        );
        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_output("The weather today is sunny with a chance of rain")
                .with_step(1),
        );

        let hall_result = history.detect_hallucination();
        assert!(hall_result.is_some(), "Should detect hallucination");
    }

    #[tokio::test]
    async fn test_no_hallucination_when_on_track() {
        let mut history = AgentHistory::new("agent-1".into(), 10);
        let objective = "Calculate the fibonacci sequence up to 10 terms";

        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::Starting)
                .with_objective(objective)
                .with_step(0),
        );
        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_output("The fibonacci sequence starts with 0, 1, 1, 2, 3, 5, 8...")
                .with_step(1),
        );

        assert!(history.detect_hallucination().is_none());
    }

    #[tokio::test]
    async fn test_token_explosion_detection() {
        let mut history = AgentHistory::new("agent-1".into(), 10);

        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_tokens(100)
                .with_step(1),
        );
        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_tokens(250)
                .with_step(2),
        );
        history.push(
            Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_tokens(600)
                .with_step(3),
        );

        let tok_result = history.detect_token_explosion(1000);
        assert!(tok_result.is_some(), "Should detect token explosion");
    }

    #[tokio::test]
    async fn test_supervisor_process_heartbeat() {
        let supervisor = Arc::new(SupervisorBuilder::new()
            .loop_threshold(3)
            .build());

        // Send normal heartbeats — should not trigger issues
        for i in 0..3 {
            let hb = Heartbeat::new("agent-1")
                .with_state(AgentState::Thinking)
                .with_output(format!("Thinking step {}", i))
                .with_step(i)
                .with_elapsed(i * 1000);
            let issue = supervisor.process_heartbeat(hb).await;
            assert!(issue.is_none(), "Normal heartbeats should not trigger issues");
        }

        let stats = supervisor.stats().await;
        assert_eq!(stats.tracked_agents, 1);
        assert_eq!(stats.active_issues, 0);
    }

    #[tokio::test]
    async fn test_supervisor_detects_loop() {
        let supervisor = Arc::new(SupervisorBuilder::new()
            .loop_threshold(3)
            .build());

        // Send identical outputs to trigger loop detection
        for i in 0..4 {
            let hb = Heartbeat::new("agent-1")
                .with_state(AgentState::GeneratingOutput)
                .with_output("REPEATED OUTPUT")
                .with_step(i)
                .with_elapsed(i * 1000);
            supervisor.process_heartbeat(hb).await;
        }

        let issues = supervisor.active_issues().await;
        assert!(!issues.is_empty(), "Should have detected loop");
        assert_eq!(issues[0].issue_type, IssueType::Loop);
    }

    #[tokio::test]
    async fn test_correction_prompt_generation() {
        let issue = Issue {
            issue_type: IssueType::Loop,
            agent_id: "agent-1".into(),
            detail: "Output loop: last 3 outputs are identical".into(),
            occurred_at: "now".into(),
            severity: 7,
        };

        let prompt = Supervisor::build_correction_prompt(&issue);
        assert!(prompt.contains("[SYSTEM CORRECTION]"));
        assert!(prompt.contains("loop"), "Prompt should contain 'loop', got: {}", prompt);
    }
}
