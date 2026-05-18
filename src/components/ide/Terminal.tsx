import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useIdeStore } from "@/stores/useIdeStore";
import { Terminal as TerminalIcon, Trash2 } from "lucide-react";

export function Terminal() {
  const [output, setOutput] = useState<string[]>([
    "Prime Terminal v0.2.0",
    "Type 'exit' to close, Ctrl+C to interrupt",
    "─".repeat(40),
  ]);
  const [input, setInput] = useState("");
  const [history, setHistory] = useState<string[]>([]);
  const [historyIdx, setHistoryIdx] = useState(-1);
  const [cwd, setCwd] = useState("/");
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const workspace = useIdeStore((s) => s.workspace);

  useEffect(() => {
    if (workspace?.path) setCwd(workspace.path);
  }, [workspace]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [output]);

  const executeCommand = async (cmd: string) => {
    const trimmed = cmd.trim();
    if (!trimmed) return;

    setHistory((h) => [...h, trimmed]);
    setHistoryIdx(-1);
    setOutput((o) => [...o, `$ ${trimmed}`]);

    try {
      const result = await invoke("plugin:shell|execute", {
        command: "bash",
        args: ["-c", trimmed],
        cwd,
      });
      const stdout = (result as any)?.stdout || "";
      const stderr = (result as any)?.stderr || "";
      if (stdout) setOutput((o) => [...o, ...stdout.split("\n").filter(Boolean)]);
      if (stderr) setOutput((o) => [...o, ...stderr.split("\n").filter(Boolean)]);
    } catch (e) {
      setOutput((o) => [...o, `Error: ${e}`]);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      executeCommand(input);
      setInput("");
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (history.length > 0) {
        const idx = historyIdx === -1 ? history.length - 1 : Math.max(0, historyIdx - 1);
        setHistoryIdx(idx);
        setInput(history[idx]);
      }
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (historyIdx >= 0) {
        const idx = historyIdx + 1;
        if (idx >= history.length) {
          setHistoryIdx(-1);
          setInput("");
        } else {
          setHistoryIdx(idx);
          setInput(history[idx]);
        }
      }
    }
  };

  return (
    <div
      className="flex flex-col bg-black/90 text-green-400 text-xs font-mono"
      style={{ height: 180 }}
      onClick={() => inputRef.current?.focus()}
    >
      <div className="flex items-center justify-between border-b border-white/10 px-2 py-1">
        <div className="flex items-center gap-1.5">
          <TerminalIcon className="h-3 w-3" />
          <span className="text-[10px] text-white/60">TERMINAL</span>
        </div>
        <button
          onClick={() => useIdeStore.getState().setTerminalOpen(false)}
          className="rounded p-0.5 text-white/40 hover:text-white"
        >
          <Trash2 className="h-3 w-3" />
        </button>
      </div>
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-2 leading-relaxed">
        {output.map((line, i) => (
          <div key={i} className="whitespace-pre-wrap">{line}</div>
        ))}
      </div>
      <div className="flex items-center border-t border-white/10 px-2 py-1">
        <span className="text-blue-300 shrink-0">{cwd.split("/").pop() || "~"} $ </span>
        <input
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 bg-transparent ml-1 outline-none border-none text-green-400"
        />
      </div>
    </div>
  );
}
