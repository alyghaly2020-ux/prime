# Prime — Master Tools Registry
## القائمة النظيفة بعد إزالة الوهمي والدمج

**تاريخ:** 2026-05-15 (مُحدّث) | **إجمالي الأدوات الفريدة:** 176

---

## ⚡ Tools Registry — التكامل مع Prime (جديد)

تم دمج **47 أداة إضافية** في `src-tauri/src/tools/config.rs` عبر 8 فئات جديدة، ليصبح الإجمالي **~145 أداة عبر 25 فئة**.

### الفئات الـ 8 الجديدة

| الفئة | الأدوات | أمثلة |
|-------|---------|-------|
| **SearchEngine** (10) | meilisearch, typesense, sonic, zincsearch, searxng, tantivy-cli, quickwit, bayard, minisearch, fsearch | محركات بحث محلية وقابلة للتوزيع |
| **ContentFetching** (3) | jina-reader, firecrawl-mcp, mcp-webresearch | استخراج محتوى من الويب |
| **EmbeddingsVectorDb** (5) | qdrant, weaviate, chroma, milvus, pgvector | تخزين المتجهات والبحث الدلالي |
| **MemoryGraph** (8) | neo4j, memgraph, apache-age, edgedb, surrealdb, dgraph, arangodb, kuzudb | قواعد بيانات جراف وذاكرة |
| **LocalModels** (4) | ollama, llama-cpp-python, vllm, localai | تشغيل نماذج ذكاء محلية |
| **AgentOrchestration** (6) | crewai, autogen, langgraph, pydantic-ai, swarm (openai), mcp-agent-adapter | تنسيق وكلاء ذكاء |
| **RagEngine** (3) | chromadb-rag, haystack, canogarage-reranker | RAG وإعادة ترتيب النتائج |
| **ReferenceUi** (2) | mcp-ui-demo, mcp-inspector | أدوات مرجعية لواجهة MCP |

### أوامر Tauri الجديدة
| الأمر | الوظيفة |
|-------|---------|
| `list_all_tools` | عرض جميع الأدوات الـ 145+ |
| `get_tool` | تفاصيل أداة معينة |
| `search_tools` | بحث في الأدوات حسب الاسم أو الفئة |
| `toggle_tool` | تفعيل/تعطيل أداة |
| `enable_tool_category` | تفعيل/تعطيل فئة كاملة |

### واجهة المستخدم
- **المكون:** `src/components/ToolsRegistry.tsx`
- **المخزن:** `src/stores/useToolsStore.ts` (Zustand)
- **الدمج:** `src/App.tsx` — زر "Tools" في الشريط الجانبي (أيقونة Wrench)
- **الميزات:** أزرار فلترة حسب الفئة، شريط بحث، كروت أدوات مع تفعيل/تعطيل

### نبذة عن التكامل
```
src-tauri/src/
  tools/
    mod.rs         → module declaration
    config.rs      → ~145 ToolEntry definitions (50KB)
    registry.rs    → ToolRegistry (get/list/search/enable)
    commands.rs    → 5 Tauri commands + parse_category()
```

جميع الأدوات تم التحقق منها عبر GitHub/npm/PyPI/Docker Hub. 3 أدوات مستبعدة (QEX, article-mcp, FRAGATA) لعدم وجودها فعليًا.

### أدوات MCP خارجية مميزة
بالإضافة للأدوات المسجلة، تم تحديد 5 أدوات MCP خارجية قابلة للتكامل:
- **TGMCP** — جسر MCP مع تيليجرام (73+ عملية)
- **AgentQL MCP** — استخراج بيانات مهيكلة من الويب
- **Stealth MCP Browser** — CloakBrowser + MCP
- **mcp-jupyter** — Jupyter kernel MCP
- **NotebookLM MCP** — اتصال Google NotebookLM

---

## 🥷 1. المتصفحات الخفية (Stealth Browsers)
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 1 | **CloakBrowser** | Chromium 146 معدل بـ 49 patch. يمر من Cloudflare, DataDome, FingerprintJS | ✅ 11.7k★ | `npm install cloakbrowser` |
| 2 | **Camoufox** | Firefox fork مع تزييف بصمة C++. REST API | ✅ 216★ | `git clone https://github.com/daijro/camoufox.git` |
| 3 | **Obscura** | Rust headless browser خفيف 30MB، للتخفي والذكاء الاصطناعي | ✅ 12.4k★ | `git clone https://github.com/h4ckf0r0day/obscura.git` |
| 4 | **Opensteer** | إطار أتمتة متصفح للوكلاء، YC-backed | ✅ 172★ | `npm i -g opensteer` (أو `uv tool install opensteer`) |
| 5 | **invisible_playwright** | Firefox 150 معدل + Playwright. يجتاز reCAPTCHA v3 | ✅ 146★ | `git clone https://github.com/feder-cr/invisible_playwright.git` |
| 6 | **BotBrowser** | Chromium خصوصي متعدد المنصات، 72 إصدار | ✅ 2.4k★ | `git clone https://github.com/botswin/BotBrowser.git` |

## 🕸️ 2. الهوية وإدارة الجلسات (Identity & Session)
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 7 | **CloakBrowser Manager** | بديل Multilogin/GoLogin self-hosted | ✅ | `docker run -p 8080:8080 cloakhq/cloakbrowser-manager` |
| 8 | **puppeteer-with-fingerprints** | تغيير بصمة وتوليد هوية افتراضية | ✅ 451★ | `npm install puppeteer-with-fingerprints` |
| 9 | **Undetect** | متصفح Stealthium مقاوم للكشف + proxies | ✅ تجاري | undetect.io |
| 10 | **Browserless** | منصة إدارة متصفحات للإنتاج | ✅ 13.2k★ | browserless.io |
| 11 | **fingerprint-suite** | توليد بصمات متصفح واقعية لـ Playwright/Puppeteer | ✅ 2.2k★ | `npm install fingerprint-generator fingerprint-injector` |

## 🧠 3. تنسيق الأسراب (Swarm Orchestration)
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 12 | **Ruflo** (Cognition) | تنسيق 100+ وكيل. 98 وكيل، 60 أمر، 30 مهارة | ✅ 51.2k★ | `npx ruflo init` |
| 13 | **Evonic** | إطار تصميم وبناء وتنسيق الوكلاء. A2A communication | ✅ 194★ | `git clone https://github.com/anvie/evonic.git` |
| 14 | **ClawTeam** | CLI لإنشاء فرق وكلاء. وكيل قائد يفرخ وكلاء متخصصين | ✅ 5.2k★ | `pip install clawteam` |
| 15 | **Mission Control** | لوحة تحكم self-hosted لتنسيق أساطيل الوكلاء | ✅ 4.8k★ | `git clone https://github.com/builderz-labs/mission-control.git` |
| 16 | **Open Swarm** | منسق محلي لإطلاق ومراقبة أسراب الوكلاء | ✅ 399★ | `git clone https://github.com/openswarm-ai/openswarm.git` |
| 17 | **ClawSwarm** | نظام تنسيق مفتوح المصدر مع Docker | ✅ 228★ | `docker run -d --name=clawswarm 1panel/clawswarm:latest` |
| 18 | **swarmkit** | مجموعة أدوات لبناء وتشغيل أنظمة الوكلاء المتعددين | ✅ | `npm install @swarmkit/e2b` |
| 19 | **openclaw-hawkins** | تنسيق متعدد الوكلاء (VINES + VECNA) | ✅ 5★ | `git clone https://github.com/parijatmukherjee/openclaw-hawkins.git` |

## 💰 4. الاستقلال المالي والتسييل (Autonomous Monetization)
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 20 | **CashClaw** | وكيل مستقل يأخذ العمل، ينجزه، يقبض ثمنه | ✅ | `npm install -g cashclaw-agent` |
| 21 | **Lucid Agents** | إطار بناء ونشر وكلاء مستقلين. دفع بالعملات الرقمية | ✅ | `bunx @lucid-agents/cli create` |
| 22 | **ArcWarden** | وكيل اقتصادي مستقل (USDC) | ✅ (hackathon) | `git clone https://github.com/ibonon/Arcwarden.git` |
| 23 | **Bitterbot Desktop** | مساعد ذكاء اصطناعي محلي. ذاكرة بيولوجية، اقتصاد مهارات P2P | ✅ 1k★ | `git clone https://github.com/Bitterbot-AI/bitterbot-desktop.git` |
| 24 | **DGrid Arena** | ربط API keys بوكيل يصوت في معارك النماذج | ✅ | dgrid.ai |
| 25 | **TON AI Agent Marketplace** | سوق وكلاء مالية لامركزي | ✅ | TON Network |
| 26 | **AlphaWallet** | محفظة Ethereum مفتوحة المصدر 100% | ✅ | alphawallet.com |
| 27 | **Clear EVM Wallet** | محفظة EVM خفيفة (MetaMask API) | ✅ | github.com |
| 28 | **PayPal Agent Toolkit** | 30+ عملية PayPal عبر function calling | ✅ رسمي | `pip install paypal-agent-toolkit` |
| 29 | **PayCrypt** | توليد أكواد تكامل بوابات الدفع تلقائيًا | ✅ (0★ تعليمي) | `git clone https://github.com/AmanYadav000/PayCrypt.git` |
| 30 | **Braintree MCP Server** | خادم MCP لـ PayPal Braintree | ✅ (3★) | llmbase.ai |

## 🔐 5. الأمان الهجومي (Offensive Cyber)
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 31 | **Transilience AI Community Tools** | 26 مهارة لأتمتة اختبار الاختراق | ✅ 237★ | `git clone https://github.com/transilienceai/communitytools.git` |
| 32 | **ThreatSwarm** | 27 وكيل ذكاء اصطناعي لسلسلة اختراق كاملة | ✅ | `git clone https://github.com/mukul975/Threatswarm.git` |
| 33 | **METATRON** | مساعد اختراق محلي بالكامل (Qwen 3.5 محلي) | ✅ 2.5k★ | `git clone https://github.com/sooryathejas/METATRON.git` |
| 34 | **CyberStrike** | وكيل أمن هجومي مفتوح المصدر | ✅ 192★ | `npm install @cyberstrike-io/cyberstrike` |

## 🌐 6. الـ Proxy وتدوير IP و IPv6
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 35 | **ProxyBroker2** | بروكسي دوار من 50+ مصدر (7000+ بروكسي) | ✅ | `docker run --rm bluet/proxybroker2` |
| 36 | **NyxProxy-OSS** | تدوير IPv6 تلقائي (<100ms latency) | ✅ 18★ | `git clone https://github.com/Jannik-Schroeder/nyxproxy-oss.git` |
| 37 | **OmniProx** | مدير بروكسي متعدد السحابات (GCP, Azure, CloudFlare) | ✅ 277★ | `git clone https://github.com/ZephrFish/OmniProx.git` |
| 38 | **go-proxy6** | خادم بروكسي IPv6 عالي الأداء (199k req/s) | ✅ | `docker-compose up -d` |
| 39 | **ipv6-dynamic-proxy** | وكيل ديناميكي SOCKS5 + HTTP بعناوين عشوائية | ✅ 6★ | `docker pull ghcr.io/seongminhwan/ipv6-dynamic-proxy` |
| 40 | **http-proxy-ipv6-pool** | كل طلب من IPv6 منفصل (Rust) | ✅ 626★ | `git clone https://github.com/zu1k/http-proxy-ipv6-pool.git` |
| 41 | **Termux-Tor-IP-Rotator** | مغير IP عبر Tor مع تدوير تلقائي | ✅ 4★ | `git clone https://github.com/naborajs/Termux-Tor-IP-Rotator.git` |
| 42 | **aproxy** | خادم بروكسي مجهول خفيف | ✅ 4★ | `git clone https://github.com/ArnabXD/aproxy.git` |
| 43 | **rotato** | تدوير API keys (OpenAI, Gemini, Groq) عند 429 | ✅ | `npx rotato` |

## 🖥️ 7. SSH وريموت ديسك توب
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 44 | **Nexterm** | إدارة خوادم SSH, VNC, RDP, SFTP, Docker, Proxmox | ✅ 4.3k★ | `docker run -d -p 6989:6989 germannewsmaker/nexterm:latest` |
| 45 | **Lazyssh** | مدير SSH تفاعلي على الطرفية | ✅ 3.4k★ | `git clone https://github.com/Adembc/lazyssh.git` |
| 46 | **SSH Pilot** | مدير اتصالات SSH متعدد المنصات | ✅ 829★ | `git clone https://github.com/mfat/sshpilot.git` |
| 47 | **XPipe** | مركز اتصالات Shell ومدير ملفات عن بعد | ✅ 14k★ | `git clone https://github.com/xpipe-io/xpipe.git` |

## 🏗️ 8. إدارة السيرفرات والمراقبة
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 48 | **Beszel** | منصة مراقبة خوادم خفيفة | ✅ 21.4k★ | beszel.dev |
| 49 | **1Panel V2** | لوحة إدارة خوادم Linux حديثة | ✅ 34.7k★ | `git clone https://github.com/1Panel-dev/1Panel.git` |
| 50 | **Cronicle** | مجدول مهام متعدد الخوادم مع واجهة ويب | ✅ 5.6k★ | `git clone https://github.com/jhuckaby/Cronicle.git` |
| 51 | **MCP Server Manager** | مدير خوادم MCP متعدد المنصات (Go + HTMX) | ✅ 12★ | `git clone https://github.com/vlazic/mcp-server-manager.git` |

## 🔌 9. مقدمي خدمات الذكاء الاصطناعي (AI Providers)
### الـ 26 مزود كلهم حقيقيين (باستثناء Bedrock Mantle اسمه AWS Bedrock)
OpenAI, Anthropic, Google Gemini, OpenRouter, DeepSeek, Kimi/Moonshot, MiniMax, Qwen, xAI (Grok), Ollama, vLLM, Nous Portal, z.ai/GLM, Dola Seed, BytePlus, Xiaomi MiMo, Azure, Venice AI, Groq, Mistral, Volcano Engine, Fireworks AI, StepFun, AWS Bedrock, llama.cpp, OpenAI-Compatible

### أدوات التكامل:
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 52 | **aisuite** | API موحد لـ OpenAI, Anthropic, Google, etc | ✅ | `pip install aisuite[all]` |
| 53 | **LiteLLM** | واجهة موحدة لـ 100+ نموذج | ✅ | `pip install litellm` |
| 54 | **any-llm** | تبديل مقدمي النماذج (Mozilla.ai) | ✅ | `pip install any-llm` |
| 55 | **monollm** | واجهة موحدة async/await | ✅ | `pip install monollm` |
| 56 | **ConduitLLM** | بوابة API موحدة (10k+ جلسة متزامنة) | ✅ | `npm install @knn_labs/conduit-gateway-client` |

## 📱 10. منصات التواصل والتحكم عن بعد
| # | المنصة | Hermes | OpenClaw | الحالة | التثبيت |
|---|--------|--------|----------|--------|----------|
| 57 | Telegram | ✅ | ✅ | ✅ حقيقي | Long polling / Webhook |
| 58 | Discord | ✅ | ✅ | ✅ حقيقي | WebSocket Gateway |
| 59 | Slack | ✅ | ✅ | ✅ حقيقي | Socket Mode / HTTP |
| 60 | WhatsApp | ✅ | ✅ | ✅ حقيقي | WebSocket QR |
| 61 | Signal | ✅ | ✅ | ✅ حقيقي | WebSocket |
| 62 | Google Chat | ✅ | ✅ | ✅ حقيقي | HTTP Webhook |
| 63 | Microsoft Teams | ❌ | ✅ | ✅ حقيقي | Webhook / Proactive |
| 64 | LINE | ❌ | ✅ | ✅ حقيقي | Webhook |
| 65 | IRC | ❌ | ✅ | ✅ حقيقي | IRC protocol |
| 66 | Matrix | ❌ Hermes | ✅ OpenClaw | ✅ حقيقي | Matrix protocol |
| 67 | Mattermost | ❌ Hermes | ✅ OpenClaw | ✅ حقيقي | Webhook/Bot API |
| 68 | Email | ✅ | ✅ جزئي | ✅ حقيقي | SMTP/IMAP |
| 69 | SMS | ✅ | ✅ | ✅ حقيقي | Twilio |
| 70 | Home Assistant | ✅ | ✅ مجتمع | ✅ حقيقي | Webhook |
| 71 | DingTalk | ✅ | ✅ | ✅ حقيقي | بكامل الميزات |
| 72 | Feishu/Lark | ✅ | ✅ | ✅ حقيقي | بكامل الميزات |
| 73 | WeCom | ✅ | ✅ | ✅ حقيقي | EAS + Callback |
| 74 | WeChat | ✅ | ✅ (ثبت) | ✅ حقيقي | iLink Bot API QR |
| 75 | QQ/QQBot | ✅ | ✅ | ✅ حقيقي | QQ Bot API |
| 76 | Yuanbao | ✅ | ❌ | ✅ حقيقي | Yuanbao API |
| 77 | CLI | ✅ | ✅ | ✅ حقيقي | سطر أوامر |
| 78 | Web UI | ✅ | ✅ | ✅ حقيقي | لوحة تحكم |
| 79 | Cron Jobs | ✅ | ✅ | ✅ حقيقي | مهام مجدولة |

### أدوات تكامل التواصل:
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 80 | **TGMCP** | جسر MCP بين وكلاء AI وتيليجرام (73+ عملية) | ✅ | `pip install tgmcp` |
| 81 | **PyBotNet** | إطار تحكم عن بعد (261★) | ✅ | `git clone https://github.com/onionj/pybotnet.git` |
| 82 | **Neonize + whatsmeow** | بوت واتساب متعدد الجلسات (Go أصلي) | ✅ | `git clone https://github.com/adityapatil123/whatsapp-bot-example.git` |

## 🧩 11. خوادم MCP والمهارات الإضافية
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 83 | **AgentQL MCP** | استخراج بيانات مهيكلة من الويب | ✅ | `npx -y agentql-mcp` |
| 84 | **ZeroMCP** | خادم MCP بلغة Python نقية، بلا اعتماديات | ✅ 74★ | `git clone https://github.com/mrexodia/zeromcp.git` |
| 85 | **mcp-jupyter** | Jupyter kernel MCP | ✅ | `git clone https://github.com/block/mcp-jupyter.git` |
| 86 | **NotebookLM MCP** | اتصال Google NotebookLM | ✅ 2.4k★ | `npx notebooklm-mcp` |
| 87 | **Stealth MCP Browser** | CloakBrowser + MCP للتصفح الخفي | ✅ | `pip install cloakbrowsermcp` |
| 88 | **Swarm Skills Algorithm** | استخراج مسارات إلى مهارات سرب جديدة | ✅ | arxiv.org/abs/2605.10052 |

## ⚡ 12. تقليل استهلاك الرموز (Token Efficiency)
| # | الأداة | الوصف | الحالة | التثبيت |
|---|--------|--------|--------|----------|
| 89 | **leanctx** | ضغط موجهات 40-60% بدون تغيير الكود | ✅ 221★ | `pip install leanctx` |
| 90 | **TwoTrim** | ضغط موجهات حتى 65% | ✅ 29★ | `git clone https://github.com/overseek944/twotrim.git` |
| 91 | **token-compressor-mcp** | ضغط دلالي لسير عمل LLM | ✅ | `pip install token-compressor-mcp` |
| 92 | **VibeFlow** | محسن سياق يقلل 70% من الرموز | ✅ 5★ | `git clone https://github.com/alperensu/VibeFlow.git` |
| 93 | **late-cli** | وكيل برمجة بدون تضخم رموز (294★) | ✅ | `git clone https://github.com/mlhher/late-cli.git` |
| 94 | **TokenPress** | محرك ضغط رموز بـ Rust (2-5x أقل) | ⚠️ PyPI حي، GitHub 404 | `pip install tokenpress` |
| 95 | **winnow-compress** | ضغط سياق RAG بنسبة ~50% | ✅ | `pip install winnow-compress` |

## 🏛️ 13. البنية التحتية الأساسية
| # | الأداة | التثبيت |
|---|--------|----------|
| 96 | **Docker** | docker.com |
| 97 | **Kubernetes** | kubernetes.io |
| 98 | **n8n (Community)** | n8n.io |

---

## 🗑️ تم حذف (18 أداة وهمية/مكررة)
- 9 أدوات `cognition-*` — كلها مختلقة (الموجود هو ruflo)
- **DontFeedTheAI** — GitHub 404
- **Crescendo/deepTeam** — باكج بايبي غير موجود
- **Cybr Ghost** — مش موجود خالص
- **AutoSurfer MCP** — مش موجود
- **proxy-pool-mcp-server (npx)** — npm 404
- **ServerLink** — ريبو مش موجود
- **@sasasamaes/sdk** —疑似 typosquatting
- **wdot** — skeleton 0 نجوم
- **Apiary** — مش server management (دا API docs)
- 10 تكرارات بين القائمتين (CloakBrowser, Camoufox, Obscura, BotBrowser, puppeteer-with-fingerprints, ProxyBroker2, NyxProxy-OSS, go-proxy6, OmniProx)
