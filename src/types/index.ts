// =============================================================================
// Prime — Type Definitions
// =============================================================================

export interface SystemState {
  version: string;
  uptime_secs: number;
  active_skills: number;
  active_connections: number;
  memory_used_mb: number;
  cpu_usage_pct: number;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
}

export interface ModelConfig {
  id: string;
  provider: string;
  model: string;
  max_tokens: number;
  temperature: number;
  streaming: boolean;
}

export interface SkillManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  entry: string;
  permissions: string[];
}

export interface MemoryEntry {
  id: string;
  memory_type: string;
  content: string;
  metadata: Record<string, unknown>;
  created_at: string;
  importance: number;
}

export interface SearchResult {
  file: string;
  line: number;
  content: string;
  score: number;
}

export interface ExecutionResult {
  success: boolean;
  exit_code: number;
  stdout: string;
  stderr: string;
  duration_ms: number;
}

export interface McpServerInfo {
  id: string;
  name: string;
  version: string;
  running: boolean;
}

export interface BrowserSnapshot {
  url: string;
  title: string;
  text: string;
}

// =============================================================================
// Plugin / Skill Types
// =============================================================================

export type PluginStatus = "active" | "inactive" | "error";

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  status: PluginStatus;
  enabled: boolean;
  permissions: string[];
  entry: string;
  type: "skill" | "mcp" | "theme" | "other";
}

// =============================================================================
// Workflow Types
// =============================================================================

export type WorkflowStatus = "idle" | "running" | "paused" | "completed" | "failed" | "cancelled";

export interface WorkflowStep {
  id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed" | "skipped";
  started_at?: number;
  completed_at?: number;
  error?: string;
  duration_ms?: number;
}

export interface Workflow {
  id: string;
  name: string;
  description: string;
  status: WorkflowStatus;
  steps: WorkflowStep[];
  created_at: number;
  started_at?: number;
  completed_at?: number;
  progress_pct: number;
  dag: WorkflowDagNode[];
}

export interface WorkflowDagNode {
  id: string;
  label: string;
  parents: string[];
  children: string[];
  status: WorkflowStep["status"];
}

// =============================================================================
// Memory Types
// =============================================================================

export type MemoryType = "working" | "episodic" | "semantic" | "vector";

export interface MemoryStats {
  working_count: number;
  episodic_count: number;
  semantic_count: number;
  vector_count: number;
  total_size_bytes: number;
  last_pruned: string;
}

// =============================================================================
// Event Types
// =============================================================================

export type EventSeverity = "info" | "warning" | "error" | "debug";

export interface SystemEvent {
  id: string;
  type: string;
  source: string;
  severity: EventSeverity;
  message: string;
  metadata?: Record<string, unknown>;
  timestamp: number;
}

// =============================================================================
// Log Types
// =============================================================================

export type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

export interface LogEntry {
  id: string;
  level: LogLevel;
  message: string;
  target: string;
  file?: string;
  line?: number;
  timestamp: number;
}

// =============================================================================
// AI Model Types
// =============================================================================

export type ModelStatus = "online" | "offline" | "error" | "loading";

export interface ModelInfo {
  id: string;
  provider: string;
  model: string;
  status: ModelStatus;
  max_tokens: number;
  temperature: number;
  streaming: boolean;
  latency_ms?: number;
  last_used?: string;
}

// =============================================================================
// Settings Types
// =============================================================================

export type Theme = "light" | "dark" | "system";

export type TaskStatus = "Running" | "Pending" | "Completed" | "Failed";

export interface TaskInfo {
  id: string;
  metadata: Record<string, string>;
  status: TaskStatus;
  created_at: string;
  duration_ms: number | null;
  error: string | null;
}

export interface TaskSummary {
  running: number;
  pending: number;
  completed: number;
  failed: number;
  total: number;
  avg_duration_ms: number;
}

export interface AgentInfo {
  id: string;
  name: string;
  role: string;
  model: string;
}

export interface AppSettings {
  theme: Theme;
  language: string;
  sidebar_collapsed: boolean;
  auto_save_interval: number;
  telemetry_enabled: boolean;
  storage_paths: {
    data: string;
    config: string;
    cache: string;
    logs: string;
  };
  mcp_config: McpServerConfig[];
  plugin_permissions: Record<string, string[]>;
  security: SecuritySettings;
}

export interface McpServerConfig {
  id: string;
  name: string;
  command?: string;
  args?: string[];
  enabled: boolean;
  auto_start: boolean;
  env?: Record<string, string>;
}

export interface SecuritySettings {
  sandbox_enabled: boolean;
  network_access: boolean;
  filesystem_access: boolean;
  require_permission_prompt: boolean;
  audit_logging: boolean;
}

// =============================================================================
// Tool Registry Types
// =============================================================================

export type ToolSource = "Pip" | "Npm" | "Docker" | "Binary" | "Rust" | "Mcp" | "BuiltIn";

export type ToolCategory =
  | "TokenCompression" | "BrowserStealth" | "ApiGateway"
  | "PromptObfuscation" | "ProxyInfrastructure" | "IdentityMasking"
  | "SwarmOrchestration" | "Monetization" | "OffensiveCyber"
  | "ProxyIp" | "Ipv6Blocks" | "SshRemoteDesktop"
  | "ServerManagement" | "AiProviderIntegration" | "CommunicationPlatform"
  | "McpSkills" | "Infrastructure"
  | "SearchEngine" | "ContentFetching" | "EmbeddingsVectorDb"
  | "MemoryGraph" | "LocalModels" | "AgentOrchestration"
  | "RagEngine" | "ReferenceUi";

export interface ToolInfo {
  id: string;
  name: string;
  category: ToolCategory;
  source: ToolSource;
  install_cmd: string | null;
  run_cmd: string | null;
  health_check_url: string | null;
  port: number | null;
  enabled: boolean;
  installed: boolean;
  description: string;
  version: string;
  homepage: string;
}

export interface ToolsListResult {
  tools: ToolInfo[];
  total: number;
  by_category: { category: string; count: number }[];
}

// =============================================================================
// Onboarding Types
// =============================================================================

export interface OnboardingState {
  completed: boolean;
  current_step: number;
  total_steps: number;
  steps: {
    id: string;
    title: string;
    completed: boolean;
  }[];
}
