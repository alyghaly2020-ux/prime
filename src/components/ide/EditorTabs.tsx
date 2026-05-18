import { useIdeStore } from "@/stores/useIdeStore";
import { X, FileCode, Circle } from "lucide-react";

export function EditorTabs() {
  const openFiles = useIdeStore((s) => s.openFiles);
  const activeFile = useIdeStore((s) => s.activeFile);
  const setActiveFile = useIdeStore((s) => s.setActiveFile);
  const closeFile = useIdeStore((s) => s.closeFile);

  return (
    <div className="flex items-center overflow-x-auto border-b border-border bg-card/30">
      {openFiles.map((file) => (
        <div
          key={file.path}
          onClick={() => setActiveFile(file.path)}
          className={`group flex items-center gap-1.5 border-r border-border px-3 py-1.5 text-xs cursor-pointer transition-colors whitespace-nowrap ${
            activeFile === file.path
              ? "bg-background/80 text-foreground"
              : "text-muted-foreground hover:text-foreground hover:bg-accent/30"
          }`}
        >
          {file.dirty && <Circle className="h-2 w-2 fill-muted-foreground text-muted-foreground shrink-0" />}
          <FileCode className="h-3.5 w-3.5 shrink-0 text-primary" />
          <span>{file.name}</span>
          <button
            onClick={(e) => {
              e.stopPropagation();
              closeFile(file.path);
            }}
            className="ml-1 rounded p-0.5 opacity-0 group-hover:opacity-100 hover:bg-accent"
          >
            <X className="h-3 w-3" />
          </button>
        </div>
      ))}
    </div>
  );
}
