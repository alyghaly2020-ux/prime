import { useEffect, useCallback } from "react";
import { useMemoryStore } from "@/stores/useMemoryStore";
import type { MemoryType } from "@/types";
import { useTranslation } from "react-i18next";
import {
  RefreshCw,
  Loader2,
  AlertCircle,
  Brain,
  Search,
  Trash2,
  Archive,
} from "lucide-react";

const MEMORY_TABS: { key: MemoryType; icon: string }[] = [
  { key: "working", icon: "W" },
  { key: "episodic", icon: "E" },
  { key: "semantic", icon: "S" },
  { key: "vector", icon: "V" },
];

export function MemoryViewer() {
  const { t } = useTranslation();
  const {
    entries,
    stats,
    selectedType,
    searchQuery,
    loading,
    error,
    setSelectedType,
    setSearchQuery,
    fetchEntries,
    fetchStats,
    deleteEntry,
    clearMemory,
  } = useMemoryStore();

  useEffect(() => {
    fetchEntries();
    fetchStats();
  }, [fetchEntries, fetchStats]);

  useEffect(() => {
    fetchEntries();
  }, [selectedType, fetchEntries]);

  const handleSearch = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      fetchEntries();
    },
    [fetchEntries],
  );

  const typeCount = (type: MemoryType): number => {
    if (!stats) return 0;
    switch (type) {
      case "working":
        return stats.working_count;
      case "episodic":
        return stats.episodic_count;
      case "semantic":
        return stats.semantic_count;
      case "vector":
        return stats.vector_count;
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("memory.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("memory.description")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => {
              fetchEntries();
              fetchStats();
            }}
            disabled={loading}
            className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            {t("memory.refresh")}
          </button>
          <button
            onClick={() => clearMemory(selectedType)}
            className="inline-flex items-center gap-1 rounded-md border border-destructive/30 bg-background px-3 py-1.5 text-sm text-destructive hover:bg-destructive/10"
          >
            <Archive className="h-4 w-4" />
            {t("memory.clear")}
          </button>
        </div>
      </div>

      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive"
        >
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>{error}</p>
        </div>
      )}

      {/* Memory type tabs */}
      <div className="flex gap-1 rounded-lg bg-muted p-1">
        {MEMORY_TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setSelectedType(tab.key)}
            className={`flex flex-1 items-center justify-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors ${
              selectedType === tab.key
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <span className="flex h-5 w-5 items-center justify-center rounded bg-muted-foreground/10 text-xs font-bold">
              {tab.icon}
            </span>
            <span>{t("memory." + tab.key)}</span>
            {stats && (
              <span className="rounded-full bg-muted-foreground/10 px-1.5 text-xs">
                {typeCount(tab.key)}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Search */}
      <form onSubmit={handleSearch} className="flex gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t("memory.search")}
            className="w-full rounded-md border border-input bg-background py-2 pl-9 pr-3 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
        </div>
        <button
          type="submit"
          className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
        >
          {t("memory.search_btn")}
        </button>
      </form>

      {/* Stats row */}
      {stats && (
        <div className="grid grid-cols-4 gap-3">
          {MEMORY_TABS.map((tab) => (
            <div
              key={tab.key}
              className="rounded-lg border border-border bg-card p-3 text-center"
            >
              <p className="text-2xl font-bold text-card-foreground">
                {typeCount(tab.key)}
              </p>
              <p className="text-xs text-muted-foreground">{t("memory." + tab.key)}</p>
            </div>
          ))}
        </div>
      )}

      {/* Entries */}
      {loading && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      )}

      {!loading && entries.length === 0 && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Brain className="mb-2 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm text-muted-foreground">{t("memory.empty")}</p>
        </div>
      )}

      <div className="space-y-2">
        {entries.map((entry) => (
          <div
            key={entry.id}
            className="group rounded-lg border border-border bg-card p-3 transition-colors hover:bg-accent/50"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-medium text-muted-foreground">
                    {entry.memory_type}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {new Date(entry.created_at).toLocaleString()}
                  </span>
                  {entry.importance > 0 && (
                    <span className="rounded bg-yellow-500/10 px-1.5 py-0.5 text-xs text-yellow-600 dark:text-yellow-400">
                      {entry.importance.toFixed(1)}
                    </span>
                  )}
                </div>
                <p className="mt-1 text-sm text-card-foreground line-clamp-3">
                  {entry.content}
                </p>
              </div>
              <button
                onClick={() => deleteEntry(entry.id)}
                className="ml-2 rounded p-1 text-muted-foreground opacity-0 hover:text-destructive group-hover:opacity-100"
                title={t("memory.delete")}
              >
                <Trash2 className="h-4 w-4" />
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
