import { useRef, useCallback, useEffect } from "react";
import Editor, { type OnMount, type OnChange } from "@monaco-editor/react";

interface MonacoEditorProps {
  value?: string;
  language?: string;
  path?: string;
  onChange?: (value: string | undefined) => void;
}

export function MonacoEditor({ value, language, path, onChange }: MonacoEditorProps) {
  const editorRef = useRef<Parameters<OnMount>[0] | null>(null);
  const monacoRef = useRef<any>(null);

  const handleEditorMount: OnMount = useCallback((editorInstance, monaco) => {
    editorRef.current = editorInstance;
    monacoRef.current = monaco;
    editorInstance.focus();

    monaco.editor.defineTheme("prime-dark", {
      base: "vs-dark",
      inherit: true,
      rules: [
        { token: "comment", foreground: "6A9955" },
        { token: "keyword", foreground: "569CD6" },
        { token: "string", foreground: "CE9178" },
        { token: "number", foreground: "B5CEA8" },
        { token: "type", foreground: "4EC9B0" },
        { token: "function", foreground: "DCDCAA" },
        { token: "variable", foreground: "9CDCFE" },
      ],
      colors: {
        "editor.background": "#0D1117",
        "editor.foreground": "#C9D1D9",
        "editor.lineHighlightBackground": "#161B22",
        "editor.selectionBackground": "#264F78",
        "editorCursor.foreground": "#528BFF",
        "editorLineNumber.foreground": "#484F58",
        "editorLineNumber.activeForeground": "#6E7681",
        "editorGutter.background": "#0D1117",
        "editorWidget.background": "#161B22",
        "editorWidget.border": "#30363D",
        "editorBracketMatch.background": "#2EA043",
        "editorBracketMatch.border": "#2EA043",
      },
    });
    monaco.editor.setTheme("prime-dark");

    editorInstance.onDidChangeCursorPosition((e) => {
      import("@/stores/useIdeStore").then((module) => {
        module.useIdeStore.getState().setCursorPosition({
          lineNumber: e.position.lineNumber,
          column: e.position.column,
        });
      });
    });
  }, []);

  const handleChange: OnChange = useCallback(
    (val) => {
      if (onChange) onChange(val);
    },
    [onChange]
  );

  const handleSave = useCallback(() => {
    const editor = editorRef.current;
    if (!editor) return;
    const model = editor.getModel();
    if (model) {
      // Trigger save via keyboard shortcut - parent listens
      const ev = new KeyboardEvent("keydown", {
        key: "s",
        code: "KeyS",
        ctrlKey: true,
        metaKey: true,
        bubbles: true,
      });
      document.dispatchEvent(ev);
    }
  }, []);

  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) return;
    const disposable = editor.addAction({
      id: "save",
      label: "Save",
      keybindings: [monacoRef.current?.KeyMod.CtrlCmd | monacoRef.current?.KeyCode.KeyS],
      run: () => handleSave(),
    });
    return () => disposable?.dispose();
  }, [handleSave]);

  return (
    <div className="h-full w-full">
      <Editor
        height="100%"
        language={language || "typescript"}
        value={value}
        path={path}
        onChange={handleChange}
        theme="prime-dark"
        onMount={handleEditorMount}
        options={{
          fontSize: 14,
          fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
          lineNumbers: "on",
          minimap: { enabled: true, scale: 1 },
          scrollBeyondLastLine: false,
          renderWhitespace: "selection",
          tabSize: 2,
          cursorBlinking: "smooth",
          cursorSmoothCaretAnimation: "on",
          smoothScrolling: true,
          padding: { top: 12 },
          bracketPairColorization: { enabled: true },
          autoIndent: "full",
          formatOnPaste: true,
          suggest: { showKeywords: true, showSnippets: true },
          wordWrap: "on",
          automaticLayout: true,
          fixedOverflowWidgets: true,
          renderLineHighlight: "all",
          occurrencesHighlight: "singleFile",
          selectionHighlight: true,
          folding: true,
          foldingHighlight: true,
          foldingStrategy: "indentation",
        }}
      />
    </div>
  );
}
