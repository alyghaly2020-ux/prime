import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { AgentInfo } from "@/types";
import {
  Brain,
  Code2,
  Terminal,
  Cpu,
  Search,
  Shield,
  Monitor,
  Puzzle,
  Package,
  Palette,
  BookOpen,
  Beaker,
  Layers,
  Users,
  Loader2,
  AlertCircle,
  RefreshCw,
  Play,
  ChevronRight,
} from "lucide-react";
import { useState } from "react";

type AgentCategory = {
  id: string;
  label: string;
  icon: typeof Brain;
  agents: string[];
};

export function AgentMonitor() {
  const { t } = useTranslation();

  const AGENT_CATEGORIES: AgentCategory[] = [
    { id: "core", label: t("agents.category.core"), icon: Brain, agents: ["execution-supervisor","task-planner","multi-agent-coordinator","memory-compression"] },
    { id: "development", label: t("agents.category.development"), icon: Code2, agents: ["coding-agent","architecture-agent","code-review-agent","debugging-agent","refactoring-agent","performance-optimization-agent"] },
    { id: "rust", label: t("agents.category.rust"), icon: Terminal, agents: ["rust-expert-agent","tauri-expert-agent","tokio-expert-agent","wasm-engineer-agent"] },
    { id: "ml", label: t("agents.category.ml"), icon: Cpu, agents: ["llm-routing-agent","prompt-engineering-agent","context-engineering-agent","rag-engineer-agent","local-llm-agent","ollama-expert-agent","llamacpp-expert-agent","model-fine-tuning-agent","inference-optimization-agent"] },
    { id: "search", label: t("agents.category.search"), icon: Search, agents: ["repo-intelligence-agent","search-engine-agent","tantivy-expert-agent","treesitter-expert-agent","semantic-search-agent","vector-search-agent","knowledge-graph-agent"] },
    { id: "security", label: t("agents.category.security"), icon: Shield, agents: ["security-engineer-agent","sandbox-engineer-agent","capability-permission-agent","privacy-engineer-agent","encryption-agent","enterprise-security-agent","audit-logging-agent"] },
    { id: "platform", label: t("agents.category.platform"), icon: Monitor, agents: ["windows-native-agent","linux-native-agent","macos-native-agent","cross-platform-agent"] },
    { id: "automation", label: t("agents.category.automation"), icon: Terminal, agents: ["playwright-automation-agent","terminal-automation-agent","git-operations-agent","cicd-agent","testing-agent","qa-agent"] },
    { id: "observability", label: t("agents.category.observability"), icon: Beaker, agents: ["observability-agent","tracing-engineer-agent","crash-recovery-agent","self-healing-agent"] },
    { id: "orchestration", label: t("agents.category.orchestration"), icon: Layers, agents: ["workflow-engine-agent","agent-orchestration-agent","event-bus-engineer-agent","actor-systems-engineer-agent"] },
    { id: "plugin", label: t("agents.category.plugin"), icon: Puzzle, agents: ["plugin-system-agent","plugin-sdk-agent","marketplace-system-agent","mcp-engineer-agent","mcp-server-builder-agent","skill-builder-agent","workflow-builder-agent"] },
    { id: "packaging", label: t("agents.category.packaging"), icon: Package, agents: ["packaging-agent","installer-builder-agent","auto-update-agent","desktop-runtime-agent"] },
    { id: "design", label: t("agents.category.design"), icon: Palette, agents: ["documentation-writer-agent","api-designer-agent","product-designer-agent","ux-wizard-flow-agent","ui-animation-agent","ai-operating-system-agent"] },
    { id: "research", label: t("agents.category.research"), icon: BookOpen, agents: ["research-agent","benchmarking-agent","ai-evaluation-agent"] },
    { id: "resource", label: t("agents.category.resource"), icon: Users, agents: ["resource-manager-agent","os-integration-agent","tool-calling-agent","verification-agent"] },
    { id: "memory", label: t("agents.category.memory"), icon: Brain, agents: ["memory-system-agent","memory-compression-agent"] },
  ];

  const WORKFLOW_TEMPLATES = [
    { id: "code-review", name: t("agents.workflow.code_review"), description: t("agents.workflow.code_review_desc"), agents: ["code-review-agent","security-engineer-agent","performance-optimization-agent"] },
    { id: "deep-research", name: t("agents.workflow.deep_research"), description: t("agents.workflow.deep_research_desc"), agents: ["research-agent","search-engine-agent","knowledge-graph-agent"] },
    { id: "ai-chat", name: t("agents.workflow.ai_chat"), description: t("agents.workflow.ai_chat_desc"), agents: ["llm-routing-agent","prompt-engineering-agent","context-engineering-agent"] },
    { id: "agent-orch", name: t("agents.workflow.agent_orch"), description: t("agents.workflow.agent_orch_desc"), agents: ["multi-agent-coordinator","agent-orchestration-agent","workflow-engine-agent"] },
    { id: "fullstack", name: t("agents.workflow.fullstack"), description: t("agents.workflow.fullstack_desc"), agents: ["architecture-agent","coding-agent","code-review-agent","testing-agent"] },
  ];

  function getAgentCategoryId(agentId: string): string | null {
    for (const cat of AGENT_CATEGORIES) {
      if (cat.agents.includes(agentId)) return cat.id;
    }
    return null;
  }

  function AgentCard({ agent }: { agent: AgentInfo }) {
    const catId = getAgentCategoryId(agent.id);
    const cat = AGENT_CATEGORIES.find((c) => c.id === catId);
    const Icon = cat?.icon ?? Brain;

    return (
      <div className="flex items-start gap-3 rounded-lg border border-border bg-card p-3 transition-colors hover:bg-accent/50">
        <div className="rounded-md bg-primary/10 p-1.5">
          <Icon className="h-3.5 w-3.5 text-primary" />
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium text-card-foreground">{agent.name}</p>
          <p className="mt-0.5 truncate text-xs text-muted-foreground">{agent.role}</p>
          <div className="mt-1 flex items-center gap-2">
            <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{agent.model}</span>
            {cat && <span className="text-[10px] text-muted-foreground">{cat.label}</span>}
          </div>
        </div>
      </div>
    );
  }

  function WorkflowCard({ template }: { template: (typeof WORKFLOW_TEMPLATES)[0] }) {
    return (
      <div className="rounded-lg border border-border bg-card p-4 transition-colors hover:bg-accent/50">
        <div className="flex items-start justify-between">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-card-foreground">{template.name}</p>
            <p className="mt-1 text-xs text-muted-foreground">{template.description}</p>
            <div className="mt-2 flex flex-wrap gap-1">
              {template.agents.map((a) => (
                <span key={a} className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{a}</span>
              ))}
            </div>
          </div>
          <button className="ml-3 rounded-md bg-primary p-2 text-primary-foreground hover:bg-primary/90">
            <Play className="h-4 w-4" />
          </button>
        </div>
      </div>
    );
  }

  const [expandedCat, setExpandedCat] = useState<string | null>("core");
  const [filter, setFilter] = useState<string>("all");

  const { data: agents, isLoading, error, refetch } = useQuery({
    queryKey: ["agents"],
    queryFn: () => invoke<string>("list_agents").then((r) => JSON.parse(r) as AgentInfo[]),
    refetchInterval: 30000,
  });

  const filteredAgents = agents?.filter((a) => {
    if (filter === "all") return true;
    return getAgentCategoryId(a.id) === filter;
  }) ?? [];

  const agentsByCategory = new Map<string, AgentInfo[]>();
  for (const agent of filteredAgents) {
    const catId = getAgentCategoryId(agent.id) ?? "other";
    if (!agentsByCategory.has(catId)) agentsByCategory.set(catId, []);
    agentsByCategory.get(catId)!.push(agent);
  }

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-bold text-foreground">{t("agents.title")}</h1>
          <p className="text-sm text-muted-foreground">
            {agents ? t("agents.count", { count: agents.length, categories: AGENT_CATEGORIES.length }) : t("agents.loading")}
          </p>
        </div>
        <button onClick={() => refetch()} className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent">
          <RefreshCw className="h-4 w-4" />
          {t("agents.refresh")}
        </button>
      </div>

      {error && (
        <div role="alert" className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>{t("agents.failed", { error })}</p>
        </div>
      )}

      {isLoading && (
        <div className="flex items-center justify-center py-24">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {agents && agents.length === 0 && (
        <div className="flex flex-col items-center justify-center py-24 text-muted-foreground">
          <Users className="mb-3 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm font-medium">{t("agents.empty")}</p>
          <p className="text-xs">{t("agents.empty_hint")}</p>
        </div>
      )}

      {agents && agents.length > 0 && (
        <>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={() => setFilter("all")}
              className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${filter === "all" ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:bg-accent"}`}
            >
              {t("agents.all", { count: agents.length })}
            </button>
            {AGENT_CATEGORIES.map((cat) => {
              const count = agents.filter((a) => getAgentCategoryId(a.id) === cat.id).length;
              if (count === 0) return null;
              return (
                <button
                  key={cat.id}
                  onClick={() => setFilter(cat.id)}
                  className={`inline-flex items-center gap-1 rounded-full px-3 py-1 text-xs font-medium transition-colors ${filter === cat.id ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:bg-accent"}`}
                >
                  <cat.icon className="h-3 w-3" />
                  {cat.label} ({count})
                </button>
              );
            })}
          </div>

          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {Array.from(agentsByCategory.entries()).map(([catId, catAgents]) => {
              const cat = AGENT_CATEGORIES.find((c) => c.id === catId);
              const Icon = cat?.icon ?? Brain;

              return (
                <div key={catId} className="rounded-xl border border-border bg-card">
                  <button
                    onClick={() => setExpandedCat(expandedCat === catId ? null : catId)}
                    className="flex w-full items-center gap-2 px-4 py-3 text-left"
                  >
                    <ChevronRight className={`h-4 w-4 text-muted-foreground transition-transform ${expandedCat === catId ? "rotate-90" : ""}`} />
                    <Icon className="h-4 w-4 text-primary" />
                    <span className="text-sm font-medium text-card-foreground">{cat?.label ?? catId}</span>
                    <span className="ml-auto text-xs text-muted-foreground">{catAgents.length}</span>
                  </button>
                  {expandedCat === catId && (
                    <div className="border-t border-border px-4 py-3 space-y-2">
                      {catAgents.map((agent) => (
                        <AgentCard key={agent.id} agent={agent} />
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          {/* Workflow Templates */}
          <div>
            <h2 className="mb-3 text-sm font-semibold text-foreground uppercase tracking-wider">{t("agents.workflows")}</h2>
            <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
              {WORKFLOW_TEMPLATES.map((tmpl) => (
                <WorkflowCard key={tmpl.id} template={tmpl} />
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
