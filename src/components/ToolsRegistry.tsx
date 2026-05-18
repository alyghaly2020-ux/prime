import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useToolsStore } from "@/stores/useToolsStore";
import type { ToolInfo } from "@/types";
import {
  RefreshCw,
  Loader2,
  AlertCircle,
  Wrench,
  ToggleLeft,
  ToggleRight,
  ExternalLink,
  Search,
  X,
  Filter,
  Package,
  Globe,
  Terminal,
  Box,
  Cpu,
  Database,
  Layers,
  Braces,
  Sparkles,
  Workflow,
  BookOpen,
  Monitor,
} from "lucide-react";

const CATEGORY_ICONS: Record<string, typeof Wrench> = {
  TokenCompression: Braces,
  BrowserStealth: Globe,
  ApiGateway: Workflow,
  PromptObfuscation: Sparkles,
  ProxyInfrastructure: Layers,
  IdentityMasking: Cpu,
  SwarmOrchestration: Workflow,
  Monetization: Package,
  OffensiveCyber: Terminal,
  ProxyIp: Globe,
  Ipv6Blocks: Globe,
  SshRemoteDesktop: Terminal,
  ServerManagement: Terminal,
  AiProviderIntegration: Cpu,
  CommunicationPlatform: Globe,
  McpSkills: Braces,
  Infrastructure: Box,
  SearchEngine: Search,
  ContentFetching: Globe,
  EmbeddingsVectorDb: Database,
  MemoryGraph: Database,
  LocalModels: Cpu,
  AgentOrchestration: Workflow,
  RagEngine: BookOpen,
  ReferenceUi: Monitor,
};

const SOURCE_COLORS: Record<string, string> = {
  Pip: "bg-blue-500/10 text-blue-600",
  Npm: "bg-red-500/10 text-red-600",
  Docker: "bg-sky-500/10 text-sky-600",
  Binary: "bg-purple-500/10 text-purple-600",
  Rust: "bg-orange-500/10 text-orange-600",
  Mcp: "bg-green-500/10 text-green-600",
  BuiltIn: "bg-gray-500/10 text-gray-600",
};

function ToolCard({ tool }: { tool: ToolInfo }) {
  const { t } = useTranslation();
  const { toggleTool } = useToolsStore();
  const CatIcon = CATEGORY_ICONS[tool.category] || Wrench;
  const sourceColor = SOURCE_COLORS[tool.source] || "bg-muted text-muted-foreground";

  return (
    <div className="rounded-lg border border-border bg-card p-4 transition-colors hover:bg-accent/50">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3">
          <div className={`mt-1 flex h-8 w-8 items-center justify-center rounded-md bg-muted ${tool.enabled ? "text-primary" : "text-muted-foreground"}`}>
            <CatIcon className="h-4 w-4" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <h3 className="truncate font-medium text-card-foreground">{tool.name}</h3>
              <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                {t("tools.version", { version: tool.version })}
              </span>
            </div>
            <p className="mt-0.5 line-clamp-2 text-sm text-muted-foreground">
              {tool.description}
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <span className={`rounded px-1.5 py-0.5 text-xs font-medium ${sourceColor}`}>
                {t(`tools.source.${tool.source.toLowerCase()}`)}
              </span>
              {tool.install_cmd && (
                <span className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                  <Package className="h-3 w-3" />
                  {tool.install_cmd.length > 35
                    ? tool.install_cmd.slice(0, 35) + "..."
                    : tool.install_cmd}
                </span>
              )}
              {tool.port && (
                <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                  :{tool.port}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="ml-3 flex shrink-0 items-center gap-1">
          <button
            onClick={() => toggleTool(tool.id, !tool.enabled)}
            className="rounded p-1.5 text-muted-foreground hover:bg-accent"
            title={tool.enabled ? t("tools.disable") : t("tools.enable")}
          >
            {tool.enabled ? (
              <ToggleRight className="h-5 w-5 text-green-500" />
            ) : (
              <ToggleLeft className="h-5 w-5" />
            )}
          </button>
          <a
            href={tool.homepage}
            target="_blank"
            rel="noopener noreferrer"
            className="rounded p-1.5 text-muted-foreground hover:bg-accent"
            title={t("tools.homepage")}
          >
            <ExternalLink className="h-4 w-4" />
          </a>
        </div>
      </div>
    </div>
  );
}

export function ToolsRegistry() {
  const {
    tools, byCategory, total, loading, error, fetchTools,
    searchTools, searchQuery, filterCategory, setFilterCategory,
  } = useToolsStore();

  useEffect(() => {
    fetchTools();
  }, [fetchTools]);

  const { t } = useTranslation();
  const enabledCount = tools.filter((t) => t.enabled).length;

  const filteredTools = useMemo(() => {
    let result = tools;
    if (filterCategory) {
      result = result.filter((t) => t.category === filterCategory);
    }
    return result;
  }, [tools, filterCategory]);

  return (
    <div className="flex h-full flex-col gap-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("tools.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("tools.count", { count: total, enabled: enabledCount })}
          </p>
        </div>
        <button
          onClick={() => fetchTools()}
          disabled={loading}
          className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          {t("tools.refresh")}
        </button>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <input
          type="text"
          placeholder={t("tools.search")}
          value={searchQuery}
          onChange={(e) => {
            const q = e.target.value;
            setFilterCategory(null);
            searchTools(q);
          }}
          className="w-full rounded-md border border-input bg-background py-2 pl-10 pr-8 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        {searchQuery && (
          <button
            onClick={() => {
              setFilterCategory(null);
              searchTools("");
            }}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {/* Categories */}
      <div className="flex flex-wrap gap-2">
        <button
          onClick={() => setFilterCategory(null)}
          className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
            !filterCategory
              ? "bg-primary text-primary-foreground"
              : "bg-muted text-muted-foreground hover:bg-accent"
          }`}
        >
          <Filter className="h-3 w-3" />
          {t("tools.filter_all")}
        </button>
        {byCategory.map((cat) => {
          const CatIcon = CATEGORY_ICONS[cat.category] || Wrench;
          const isActive = filterCategory === cat.category;
          return (
            <button
              key={cat.category}
              onClick={() => {
                setFilterCategory(isActive ? null : cat.category);
                if (searchQuery) searchTools("");
              }}
              className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground hover:bg-accent"
              }`}
            >
              <CatIcon className="h-3 w-3" />
              {cat.category.replace(/([A-Z])/g, " $1").trim()} ({cat.count})
            </button>
          );
        })}
      </div>

      {/* Error */}
      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive"
        >
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>{error}</p>
        </div>
      )}

      {/* Loading */}
      {loading && tools.length === 0 && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      )}

      {/* Empty */}
      {!loading && filteredTools.length === 0 && !error && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Wrench className="mb-2 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm text-muted-foreground">
            {searchQuery
              ? t("tools.no_match")
              : filterCategory
                ? t("tools.no_category")
                : t("tools.empty")}
          </p>
        </div>
      )}

      {/* Tool list */}
      <div className="grid gap-3">
        {filteredTools.map((t) => (
          <ToolCard key={t.id} tool={t} />
        ))}
      </div>
    </div>
  );
}
