# 🤖 AI Copilot Git Push Protocol / بروتوكول الرفع الجراحي للذكاء الاصطناعي

> [!IMPORTANT]
> **[للنسخ واللصق للذكاء الاصطناعي / COPY & PASTE TO YOUR AI ASSISTANT]**
> قم بنسخ النص أدناه بالكامل وضعه للنموذج أو الوكيل الذكي (AI Agent) عندما تطلب منه رفع التعديلات إلى GitHub. هذا البروتوكول يجبره على الالتزام بالقواعد الجراحية الصارمة لمنع رفع أي ملفات زائدة أو مؤقتة.

---

```markdown
SYSTEM INSTRUCTION: STRICT GIT PUSH PROTOCOL FOR PRIME PLATFORM
===============================================================
You are an AI coding assistant. The user wants you to stage, commit, and push the recent project changes to GitHub.
You must adhere to this STRICT, surgical Git protocol. Any violation that stages forbidden or temporary files is strictly unacceptable.

1. 🚫 ABSOLUTELY FORBIDDEN TO STAGE OR PUSH (NEVER run 'git add .'):
   - Scratch & Trial Files: 'prime-cli-trial.sh', any '*trial*', any files in 'scratch/' or root containing trial code.
   - Local Databases: Any '*.db', '*.sqlite', '*.db-journal', '*.db-wal', '*.db-shm', or files in 'data/'.
   - System Caches & Builds: 'node_modules/', 'target/', 'src-tauri/target/', 'dist/', '.playwright-mcp/', '.svelte-kit/'.
   - Environment & Credentials: Any '*.env', '*.env.local', '*.key', '*.pem', '*.cert', or 'credentials.json'.
   - Stray Root Images: Any image files directly in the root directory (only official images in 'docs/screenshots/' and 'docs/assets/' are allowed).
   - Log Files: Any '*.log', 'logs/', 'tmp/', 'temp/'.

2. 🟢 ONLY PERMITTED FILES (Surgically Stage These):
   - React Frontend: 'src/**/*.ts', 'src/**/*.tsx', 'src/**/*.css', 'index.html', 'public/' assets.
   - Rust Backend: 'src-tauri/src/**/*.rs', 'src-tauri/Cargo.toml', 'src-tauri/tauri.conf.json', 'src-tauri/build.rs', 'src-tauri/capabilities/'.
   - Subcrates: 'prime_core/**/*.rs', 'prime_core/Cargo.toml'.
   - Official Documentation: 'README.md', 'BUILD.md', 'DEPLOYMENT_GUIDE.md', and files in 'docs/' (including 'docs/screenshots/*.png' and 'docs/assets/').
   - Configurations & Dependencies: 'package.json', 'package-lock.json', 'Cargo.toml', 'Cargo.lock', 'vite.config.ts', 'tailwind.config.ts', 'tsconfig.json', 'eslint.config.js', '.gitignore'.

3. 🛠️ STEP-BY-STEP SURGICAL GIT EXECUTION PLAN:
   - Step A: Run 'git status' to inspect modified and untracked files.
   - Step B: Stage each allowed file/folder individually using explicit, surgical commands.
     Example:
     git add src/components/BrowserModeFull.tsx src/stores/useChatStore.ts docs/screenshots/prime-browser-dark.png DEPLOYMENT_GUIDE.md
   - Step C: Double check staged files by running:
     git diff --cached --name-only
     Ensure ZERO forbidden files are listed. If any are staged, immediately unstage them via:
     git reset HEAD <file-path>
   - Step D: Run verification tests to make sure there are no compiler errors:
     npx tsc --noEmit && cargo check --workspace
   - Step E: Commit using clean semantic format:
     git commit -m "feat(browser): add direct browsing, DOM OCR modal, and clean mode toggles"
   - Step F: Push to the active branch:
     git push origin master
===============================================================
```
