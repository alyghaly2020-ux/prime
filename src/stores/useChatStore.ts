import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage } from "@/types";

export interface ChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
  model: string;
  provider: string;
}

const tauriStorage = {
  getItem: () => null,
  setItem: () => {},
  removeItem: () => {},
};

let saveTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleSave(get: () => ChatStore) {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    const { sessions, activeSessionId } = get();
    const data = JSON.stringify({ sessions, activeSessionId });
    invoke("save_chat_data", { data }).catch(() => {});
  }, 1000);
}

interface ChatStore {
  sessions: ChatSession[];
  activeSessionId: string | null;
  loading: boolean;

  createSession: (title?: string, model?: string, provider?: string, customId?: string) => string;
  deleteSession: (id: string) => void;
  setActiveSession: (id: string) => void;
  renameSession: (id: string, title: string) => void;

  addMessage: (sessionId: string, message: ChatMessage) => void;
  setMessages: (sessionId: string, messages: ChatMessage[]) => void;
  clearMessages: (sessionId: string) => void;

  activeSession: () => ChatSession | null;
  activeMessages: () => ChatMessage[];

  loadSessions: () => Promise<void>;
  saveSessions: () => Promise<void>;
}

function generateId(): string {
  return `session_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

function generateTitle(messages: ChatMessage[]): string {
  const firstUser = messages.find((m) => m.role === "user");
  if (!firstUser) return "New Chat";
  const content = firstUser.content.trim();
  return content.length > 50 ? content.slice(0, 50) + "..." : content;
}

export const useChatStore = create<ChatStore>()(
  persist(
    (set, get) => ({
      sessions: [],
      activeSessionId: null,
      loading: true,

      createSession: (title, model = "auto", provider = "auto", customId) => {
        const id = customId || generateId();
        const now = Date.now();
        const session: ChatSession = {
          id,
          title: title || "New Chat",
          messages: [],
          createdAt: now,
          updatedAt: now,
          model,
          provider,
        };
        set((s) => ({
          sessions: [session, ...s.sessions],
          activeSessionId: id,
        }));
        scheduleSave(get);
        return id;
      },

      deleteSession: (id) => {
        set((s) => {
          const sessions = s.sessions.filter((sess) => sess.id !== id);
          const activeSessionId =
            s.activeSessionId === id
              ? sessions.length > 0
                ? sessions[0].id
                : null
              : s.activeSessionId;
          return { sessions, activeSessionId };
        });
        scheduleSave(get);
      },

      setActiveSession: (id) => {
        set({ activeSessionId: id });
      },

      renameSession: (id, title) => {
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === id ? { ...sess, title } : sess
          ),
        }));
        scheduleSave(get);
      },

      addMessage: (sessionId, message) => {
        set((s) => ({
          sessions: s.sessions.map((sess) => {
            if (sess.id !== sessionId) return sess;
            const updatedMessages = [...sess.messages, message];
            return {
              ...sess,
              messages: updatedMessages,
              updatedAt: Date.now(),
              title:
                sess.title === "New Chat"
                  ? generateTitle(updatedMessages)
                  : sess.title,
            };
          }),
        }));
        scheduleSave(get);
      },

      setMessages: (sessionId, messages) => {
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === sessionId
              ? { ...sess, messages, updatedAt: Date.now() }
              : sess
          ),
        }));
        scheduleSave(get);
      },

      clearMessages: (sessionId) => {
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === sessionId
              ? { ...sess, messages: [], updatedAt: Date.now() }
              : sess
          ),
        }));
        scheduleSave(get);
      },

      activeSession: () => {
        const { sessions, activeSessionId } = get();
        return sessions.find((s) => s.id === activeSessionId) || null;
      },

      activeMessages: () => {
        const { sessions, activeSessionId } = get();
        const session = sessions.find((s) => s.id === activeSessionId);
        return session?.messages || [];
      },

      loadSessions: async () => {
        set({ loading: true });
        try {
          const data = await invoke<string | null>("load_chat_data");
          if (data) {
            const parsed = JSON.parse(data);
            set({
              sessions: parsed.sessions || [],
              activeSessionId: parsed.activeSessionId || null,
              loading: false,
            });
          } else {
            set({ loading: false });
          }
        } catch (e) {
          console.error("Failed to load chat sessions", e);
          set({ loading: false });
        }
      },

      saveSessions: async () => {
        try {
          const { sessions, activeSessionId } = get();
          const data = JSON.stringify({ sessions, activeSessionId });
          await invoke("save_chat_data", { data });
        } catch (e) {
          console.error("Failed to save chat sessions", e);
        }
      },
    }),
    {
      name: "prime-chat-store",
      storage: createJSONStorage(() => tauriStorage),
      partialize: (state) => ({
        sessions: state.sessions,
        activeSessionId: state.activeSessionId,
      }),
      skipHydration: true,
    }
  )
);
