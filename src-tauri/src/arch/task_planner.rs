use uuid::Uuid;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskPlan {
    pub id: String,
    pub goal: String,
    pub sub_tasks: Vec<SubTask>,
    pub dependencies: Vec<(String, String)>,
    pub estimated_cost: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub agent_type: String,
    pub status: TaskStatus,
    pub estimated_duration_secs: u64,
    pub cost: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

// ---------------------------------------------------------------------------
// Decomposition rule
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DecompositionRule {
    /// Keywords (lower-cased) that trigger this rule.
    keywords: &'static [&'static str],
    /// Sub-task descriptions to generate.
    sub_task_descriptions: &'static [&'static str],
    /// Agent types for each sub-task.
    agent_types: &'static [&'static str],
}

static RULES: &[DecompositionRule] = &[
    DecompositionRule {
        keywords: &["search", "find", "lookup", "retrieve", "query"],
        sub_task_descriptions: &[
            "Parse and validate search query",
            "Execute search across indexed sources",
            "Rank and filter results",
            "Format and return response",
        ],
        agent_types: &["search", "search", "search", "search"],
    },
    DecompositionRule {
        keywords: &["analyze", "analyse", "inspect", "examine", "review"],
        sub_task_descriptions: &[
            "Collect input data and parameters",
            "Run analysis heuristics",
            "Generate summary report",
        ],
        agent_types: &["analyst", "analyst", "analyst"],
    },
    DecompositionRule {
        keywords: &["code", "implement", "write", "develop", "create"],
        sub_task_descriptions: &[
            "Analyse requirements and existing codebase",
            "Design implementation approach",
            "Write implementation code",
            "Run tests and fix issues",
            "Final review and cleanup",
        ],
        agent_types: &["coding", "architect", "coding", "testing", "reviewer"],
    },
    DecompositionRule {
        keywords: &["test", "verify", "validate", "check"],
        sub_task_descriptions: &[
            "Identify test scope and requirements",
            "Write unit tests",
            "Write integration tests",
            "Execute test suite and report",
        ],
        agent_types: &["testing", "testing", "testing", "testing"],
    },
    DecompositionRule {
        keywords: &["deploy", "release", "publish", "ship"],
        sub_task_descriptions: &[
            "Run pre-deployment checks",
            "Build release artifacts",
            "Deploy to target environment",
            "Verify deployment health",
        ],
        agent_types: &["devops", "devops", "devops", "observability"],
    },
    DecompositionRule {
        keywords: &["refactor", "clean", "optimize", "optimise", "improve"],
        sub_task_descriptions: &[
            "Analyse current code for improvement areas",
            "Apply targeted refactoring",
            "Run regression tests",
            "Benchmark performance improvements",
        ],
        agent_types: &["coding", "refactoring", "testing", "benchmarking"],
    },
    DecompositionRule {
        keywords: &["document", "documentation", "readme", "docs"],
        sub_task_descriptions: &[
            "Analyse codebase to identify documentation needs",
            "Write API reference documentation",
            "Write usage guide and examples",
            "Review and finalise documentation",
        ],
        agent_types: &[
            "documentation",
            "documentation",
            "documentation",
            "reviewer",
        ],
    },
];

/// Fallback generic rule when no specific keywords match.
static FALLBACK_DESCRIPTIONS: &[&str] = &[
    "Analyse and understand the goal",
    "Break goal into actionable steps",
    "Execute each step sequentially",
    "Verify and report results",
];
static FALLBACK_AGENTS: &[&str] = &["general", "general", "general", "general"];

// ---------------------------------------------------------------------------
// TaskPlannerEngine
// ---------------------------------------------------------------------------

pub struct TaskPlannerEngine {
    /// Custom rules can be added at runtime.
    custom_rules: Vec<DecompositionRule>,
}

impl TaskPlannerEngine {
    pub fn new() -> Self {
        Self {
            custom_rules: Vec::new(),
        }
    }

    /// Register an additional decomposition rule.
    /// The strings are leaked to `'static` for simplicity.
    pub fn add_rule(
        &mut self,
        keywords: Vec<String>,
        descriptions: Vec<String>,
        agent_types: Vec<String>,
    ) {
        let leak_slice = |v: Vec<String>| -> &'static [&'static str] {
            Box::leak(
                v.into_iter()
                    .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };

        self.custom_rules.push(DecompositionRule {
            keywords: leak_slice(keywords),
            sub_task_descriptions: leak_slice(descriptions),
            agent_types: leak_slice(agent_types),
        });
    }
}

impl Default for TaskPlannerEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TaskPlanner (non-configurable, uses built-in rules)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct TaskPlanner {
    engine: TaskPlannerEngine,
}

impl TaskPlanner {
    pub fn new() -> Self {
        Self {
            engine: TaskPlannerEngine::new(),
        }
    }

    /// Create a full plan from a goal string using keyword-based rule matching.
    pub fn plan(&self, goal: &str) -> TaskPlan {
        let plan_id = Uuid::new_v4().to_string();
        let goal_lower = goal.to_lowercase();

        // Find the best-matching rule
        let (descriptions, agent_types) = RULES
            .iter()
            .find(|rule| rule.keywords.iter().any(|kw| goal_lower.contains(kw)))
            .map(|rule| (rule.sub_task_descriptions, rule.agent_types))
            .unwrap_or((FALLBACK_DESCRIPTIONS, FALLBACK_AGENTS));

        let mut sub_tasks: Vec<SubTask> = descriptions
            .iter()
            .zip(agent_types.iter())
            .enumerate()
            .map(|(i, (desc, agent))| SubTask {
                id: format!("{}-{:02}", &plan_id[..8], i + 1),
                description: desc.to_string(),
                agent_type: agent.to_string(),
                status: TaskStatus::Pending,
                estimated_duration_secs: 60 * (i as u64 + 1),
                cost: 1,
            })
            .collect();

        // Fill in goal-specific details in first sub-task description
        if let Some(first) = sub_tasks.first_mut() {
            first.description = format!("{}: {}", first.description.trim_end_matches('.'), goal);
        }

        // Build linear dependency chain: 0→1, 1→2, …
        let dependencies: Vec<(String, String)> = (0..sub_tasks.len().saturating_sub(1))
            .map(|i| (sub_tasks[i].id.clone(), sub_tasks[i + 1].id.clone()))
            .collect();

        let estimated_cost = self.estimate_cost(&sub_tasks);

        TaskPlan {
            id: plan_id,
            goal: goal.to_string(),
            sub_tasks,
            dependencies,
            estimated_cost,
        }
    }

    /// Recursively decompose complex sub-tasks into finer-grained steps.
    pub fn decompose(&self, plan: &mut TaskPlan, max_depth: usize) {
        if max_depth == 0 {
            return;
        }

        let mut new_sub_tasks: Vec<SubTask> = Vec::new();
        let mut new_deps: Vec<(String, String)> = Vec::new();
        let mut prev_id: Option<String> = None;

        for task in &plan.sub_tasks {
            let prev = prev_id.take();

            // Determine if this task warrants further decomposition
            // (e.g. estimated cost > 3 or description mentions multiple concerns)
            let should_split = task.cost > 3
                || task.description.contains(" and ")
                || task.description.contains(", ");

            if should_split {
                let split_descriptions = split_description(&task.description, 2);
                let count = split_descriptions.len();

                for (j, desc) in split_descriptions.iter().enumerate() {
                    let sub_id = format!("{}-{}", task.id, j + 1);
                    new_sub_tasks.push(SubTask {
                        id: sub_id.clone(),
                        description: desc.clone(),
                        agent_type: task.agent_type.clone(),
                        status: TaskStatus::Pending,
                        estimated_duration_secs: task.estimated_duration_secs / count as u64,
                        cost: 1,
                    });

                    // Wire dependency from previous to this, or from parent's predecessor
                    if let Some(ref p) = prev {
                        new_deps.push((p.clone(), sub_id.clone()));
                    }
                    // Wire linear chain within split
                    if j > 0 {
                        let prev_sub_id = format!("{}-{}", task.id, j);
                        new_deps.push((prev_sub_id, sub_id.clone()));
                    }
                }

                // The last sub-task in the split becomes the predecessor for the next task
                let last = format!("{}-{}", task.id, count);
                prev_id = Some(last);
            } else {
                new_sub_tasks.push(task.clone());
                if let Some(ref p) = prev {
                    new_deps.push((p.clone(), task.id.clone()));
                }
                prev_id = Some(task.id.clone());
            }
        }

        plan.sub_tasks = new_sub_tasks;
        plan.dependencies = new_deps;
        plan.estimated_cost = self.estimate_cost(&plan.sub_tasks);

        // Recursively decompose at lower depth
        if max_depth > 1 {
            self.decompose(plan, max_depth - 1);
        }
    }

    /// Estimate total cost based on number and type of sub-tasks.
    pub fn estimate_cost(&self, sub_tasks: &[SubTask]) -> u32 {
        sub_tasks
            .iter()
            .map(|t| self.cost_for_agent(&t.agent_type))
            .sum()
    }

    fn cost_for_agent(&self, agent_type: &str) -> u32 {
        match agent_type {
            "search" | "testing" => 1,
            "general" | "coding" | "refactoring" => 3,
            "analyst" | "documentation" => 2,
            "architect" | "devops" => 4,
            "benchmarking" | "reviewer" => 1,
            _ => 2,
        }
    }
}

impl Default for TaskPlanner {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Split a description into `n` roughly equal parts on natural boundaries.
fn split_description(desc: &str, n: usize) -> Vec<String> {
    if n <= 1 {
        return vec![desc.to_string()];
    }

    // Try splitting on " and " or ", " first
    let separators = [" and ", ", ", "; "];
    for sep in &separators {
        let parts: Vec<&str> = desc.split(sep).collect();
        if parts.len() >= n {
            return parts.iter().take(n).map(|s| s.trim().to_string()).collect();
        }
    }

    // Fallback: split by words
    let words: Vec<&str> = desc.split_whitespace().collect();
    if words.is_empty() {
        return vec![desc.to_string()];
    }

    let chunk_size = words.len().div_ceil(n);
    words
        .chunks(chunk_size)
        .map(|chunk| chunk.join(" "))
        .take(n)
        .collect()
}
