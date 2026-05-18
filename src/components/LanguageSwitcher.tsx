import { useState, useRef, useEffect } from "react";
import { LANGUAGES, setLanguage } from "@/locales/i18n";
import { useTranslation } from "react-i18next";
import { Globe } from "lucide-react";

export function LanguageSwitcher() {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const { i18n } = useTranslation();
  const current = LANGUAGES.find((l) => l.code === i18n.language) ?? LANGUAGES[0];

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    if (open) document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        className="inline-flex items-center gap-1 rounded-lg px-2 py-1.5 text-xs text-muted-foreground hover:text-foreground hover:bg-accent transition-all"
        title={current.native}
      >
        <Globe className="h-3.5 w-3.5" />
        <span className="hidden sm:inline text-[10px] uppercase tracking-wider">{current.code}</span>
      </button>
      {open && (
        <div className="absolute right-0 top-full z-20 mt-1 w-36 rounded-lg border border-border bg-card py-1 shadow-lg">
          {LANGUAGES.map((lang) => (
            <button
              key={lang.code}
              onClick={() => { setLanguage(lang.code); setOpen(false); }}
              className={`flex w-full items-center gap-2 px-3 py-1.5 text-xs transition-colors hover:bg-accent ${
                lang.code === i18n.language ? "text-primary font-medium" : "text-card-foreground"
              }`}
            >
              <span className="w-5 text-center text-[10px] text-muted-foreground">{lang.code.toUpperCase()}</span>
              <span>{lang.native}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
