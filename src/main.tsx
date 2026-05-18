import { Component, ErrorInfo, ReactNode } from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import "./index.css";
import "./locales/i18n";

window.onerror = (message, source, lineno, colno, error) => {
  console.error("[Global Error]", { message, source, lineno, colno, error });
};

window.onunhandledrejection = (event) => {
  console.error("[Unhandled Promise Rejection]", event.reason);
};

import { AlertTriangle, RefreshCw, Trash2, Terminal as TerminalIcon } from "lucide-react";

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  showDetails: boolean;
}

class ErrorBoundary extends Component<{ children: ReactNode }, ErrorBoundaryState> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null, showDetails: false };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error, showDetails: false };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("[ErrorBoundary]", error, errorInfo);
  }

  handleHardReset = () => {
    try {
      localStorage.clear();
      sessionStorage.clear();
      window.location.reload();
    } catch (e) {
      console.error("Failed to clear storage:", e);
    }
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen w-screen flex-col items-center justify-center bg-slate-950 p-6 text-slate-100 selection:bg-indigo-500/30">
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(99,102,241,0.05),transparent_60%)] pointer-events-none" />
          
          <div className="relative z-10 w-full max-w-xl overflow-hidden rounded-2xl border border-slate-800 bg-slate-900/60 backdrop-blur-xl p-8 shadow-[0_0_50px_-12px_rgba(99,102,241,0.15)] transition-all">
            
            <div className="flex items-center justify-center mx-auto h-16 w-16 rounded-2xl bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 mb-6">
              <AlertTriangle className="h-8 w-8 animate-pulse" />
            </div>

            <div className="text-center mb-6">
              <h1 className="text-xl font-bold text-slate-50 tracking-tight">حدث خطأ غير متوقع في النظام</h1>
              <p className="text-xs text-indigo-400 font-semibold tracking-wider uppercase mt-1">An unexpected crash has occurred</p>
              <p className="text-sm text-slate-400 mt-3 leading-relaxed">
                لقد واجه Prime مشكلة غير متوقعة وتوقف عن العمل. يمكنك محاولة إعادة تشغيل التطبيق أو تفريغ ذاكرة التخزين المؤقت إذا استمرت المشكلة.
              </p>
            </div>

            <div className="flex flex-col sm:flex-row gap-3 justify-center mb-6">
              <button
                onClick={() => window.location.reload()}
                className="flex items-center justify-center gap-2 rounded-xl bg-indigo-600 px-5 py-2.5 text-xs font-semibold text-white shadow-md hover:bg-indigo-500 active:scale-95 transition-all"
              >
                <RefreshCw className="h-3.5 w-3.5" />
                إعادة تشغيل التطبيق (Reload App)
              </button>
              
              <button
                onClick={this.handleHardReset}
                className="flex items-center justify-center gap-2 rounded-xl bg-slate-800 px-5 py-2.5 text-xs font-semibold text-slate-300 border border-slate-700/50 hover:bg-slate-700 hover:text-white hover:border-slate-600 active:scale-95 transition-all"
                title="تصفير الجلسات وحذف الملفات المؤقتة المخزنة في المتصفح بالكامل"
              >
                <Trash2 className="h-3.5 w-3.5 text-red-400" />
                تصفير بيانات التطبيق (Hard Reset)
              </button>
            </div>

            <div className="border-t border-slate-800/80 pt-4">
              <button
                onClick={() => this.setState({ showDetails: !this.state.showDetails })}
                className="flex items-center gap-2 text-[10px] font-bold uppercase tracking-widest text-slate-500 hover:text-slate-300 transition-colors mx-auto"
              >
                <TerminalIcon className="h-3.5 w-3.5 text-indigo-500/70" />
                {this.state.showDetails ? "إخفاء التفاصيل الفنية (Hide Details)" : "إظهار التفاصيل الفنية (Show Details)"}
              </button>

              {this.state.showDetails && (
                <div className="mt-3 overflow-hidden rounded-xl border border-slate-800/80 bg-slate-950 p-4">
                  <p className="text-[10px] font-semibold text-slate-500 mb-1.5 uppercase font-mono">Error Log Trace:</p>
                  <pre className="max-h-40 overflow-y-auto text-[11px] font-mono leading-relaxed text-red-400/90 whitespace-pre-wrap select-text pr-2 scrollbar-thin scrollbar-thumb-slate-800">
                    {this.state.error?.stack || this.state.error?.message || "No stack trace available"}
                  </pre>
                </div>
              )}
            </div>
            
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60,
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <ErrorBoundary>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </ErrorBoundary>,
);
