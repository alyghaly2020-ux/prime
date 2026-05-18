use super::a11y::AccessibilityTree;
use super::{BrowserAction, PageSnapshot};
use chromiumoxide::{
    cdp::browser_protocol::page::CaptureScreenshotFormat,
    Browser, BrowserConfig, Page,
};
use futures::StreamExt;
use scraper::Html;
use tokio::sync::RwLock;

const CDP_PORT: u16 = 9222;

fn find_chrome() -> Option<std::path::PathBuf> {
    let mut candidates = vec![
        // Linux paths
        "/usr/bin/google-chrome".into(),
        "/usr/bin/google-chrome-stable".into(),
        "/usr/bin/chromium".into(),
        "/usr/bin/chromium-browser".into(),
        "/snap/bin/chromium".into(),
        "/snap/bin/google-chrome".into(),
        
        // macOS paths
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
    ];

    // Windows paths
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    candidates.push("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe".into());
    candidates.push("C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe".into());
    candidates.push("C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe".into());
    candidates.push("C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe".into());
    if !local.is_empty() {
        candidates.push(std::path::Path::new(&local).join("Google\\Chrome\\Application\\chrome.exe"));
        candidates.push(std::path::Path::new(&local).join("Microsoft\\Edge\\Application\\msedge.exe"));
    }

    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

#[derive(Debug)]
pub struct PlaywrightEngine {
    connected: RwLock<bool>,
    headless: RwLock<bool>,
    visible: RwLock<bool>,
    browser: RwLock<Option<Browser>>,
    page: RwLock<Option<Page>>,
    current_url: RwLock<String>,
    a11y: AccessibilityTree,
    _client: reqwest::Client,
}

impl Default for PlaywrightEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaywrightEngine {
    pub fn new() -> Self {
        Self {
            connected: RwLock::new(false),
            headless: RwLock::new(true),
            visible: RwLock::new(false),
            browser: RwLock::new(None),
            page: RwLock::new(None),
            current_url: RwLock::new(String::new()),
            a11y: AccessibilityTree::new(),
            _client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Prime-Browser/1.0")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn connect(&self, endpoint: &str) -> anyhow::Result<()> {
        let is_headless = !endpoint.contains("headed");
        *self.headless.write().await = is_headless;

        let chrome_path = find_chrome()
            .ok_or_else(|| anyhow::anyhow!("No Chrome/Edge installation found"))?;

        let mut config_builder = BrowserConfig::builder()
            .chrome_executable(chrome_path)
            .port(CDP_PORT);

        if !is_headless {
            config_builder = config_builder.with_head();
        }

        let config = config_builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build browser config: {}", e))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to launch browser: {}", e))?;

        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = event {
                    tracing::warn!("Browser handler event error: {:?}", e);
                }
            }
        });

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create page: {}", e))?;

        *self.browser.write().await = Some(browser);
        *self.page.write().await = Some(page);
        *self.connected.write().await = true;
        *self.visible.write().await = !is_headless;

        tracing::info!(
            "Browser engine connected (headless: {}, headed: {})",
            is_headless,
            !is_headless
        );
        Ok(())
    }

    pub async fn disconnect(&self) {
        if let Some(mut browser) = self.browser.write().await.take() {
            let _ = browser.close().await;
        }
        *self.page.write().await = None;
        *self.connected.write().await = false;
        *self.visible.write().await = false;
    }

    pub async fn set_headless(&self, headless: bool) -> anyhow::Result<()> {
        let current = *self.headless.read().await;
        if current == headless {
            return Ok(());
        }
        let endpoint = if headless {
            "ws://localhost"
        } else {
            "ws://localhost?headed"
        };
        self.disconnect().await;
        self.connect(endpoint).await
    }

    pub async fn navigate(&self, url: &str) -> anyhow::Result<PageSnapshot> {
        if !*self.connected.read().await {
            return Err(anyhow::anyhow!("Browser not connected"));
        }

        let page = self.page.read().await;
        let page_ref = page
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active page"))?;

        tracing::info!("Navigating to: {}", url);

        page_ref
            .goto(url)
            .await
            .map_err(|e| anyhow::anyhow!("Navigation failed: {}", e))?;

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        let title = page_ref.get_title().await.unwrap_or_default().unwrap_or_default();
        let html = page_ref.content().await.unwrap_or_default();
        let text = self.extract_visible_text(&html);
        let a11y_tree = Some(serde_json::to_string(&self.a11y.build(&html))?);

        let screenshot = page_ref
            .screenshot(
                chromiumoxide::page::ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .build(),
            )
            .await
            .ok();

        use base64::Engine;
        let screenshot_base64 = screenshot.map(|bytes| {
            format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
        });

        *self.current_url.write().await = url.to_string();

        Ok(PageSnapshot {
            url: url.to_string(),
            title,
            html,
            text,
            a11y_tree,
            screenshot: screenshot_base64,
        })
    }

    pub async fn snapshot(&self) -> anyhow::Result<PageSnapshot> {
        if !*self.connected.read().await {
            return Err(anyhow::anyhow!("Browser not connected"));
        }

        let page = self.page.read().await;
        let page_ref = page
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active page"))?;

        let url = self.current_url.read().await.clone();
        let title = page_ref.get_title().await.unwrap_or_default().unwrap_or_default();
        let html = page_ref.content().await.unwrap_or_default();
        let text = self.extract_visible_text(&html);
        let a11y_tree = Some(serde_json::to_string(&self.a11y.build(&html))?);

        let screenshot = page_ref
            .screenshot(
                chromiumoxide::page::ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .build(),
            )
            .await
            .ok();

        use base64::Engine;
        let screenshot_base64 = screenshot.map(|bytes| {
            format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
        });

        Ok(PageSnapshot {
            url,
            title,
            html,
            text,
            a11y_tree,
            screenshot: screenshot_base64,
        })
    }

    pub async fn execute(&self, action: &BrowserAction) -> anyhow::Result<PageSnapshot> {
        if !*self.connected.read().await {
            return Err(anyhow::anyhow!("Browser not connected"));
        }

        tracing::info!("Executing browser action: {:?}", action);

        if let Some(ms) = action.wait_ms {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }

        let page = self.page.read().await;
        let page_ref = page
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active page"))?;

        match action.action_type.as_str() {
            "navigate" => {
                let _ = page_ref;
                drop(page);
                if let Some(ref url) = action.url {
                    self.navigate(url).await
                } else {
                    Err(anyhow::anyhow!("navigate requires a URL"))
                }
            }
            "click" => {
                if let Some(ref selector) = action.selector {
                    if let Ok(elem) = page_ref.find_element(selector).await {
                        let _ = elem.click().await;
                    } else {
                        let js = format!(
                            "document.querySelector('{}')?.click()",
                            selector.replace('\'', "\\'")
                        );
                        let _ = page_ref.evaluate(js).await;
                    }
                    let _ = page_ref;
                    drop(page);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    self.snapshot().await
                } else {
                    Err(anyhow::anyhow!("click requires a selector"))
                }
            }
            "type" => {
                if let Some(ref selector) = action.selector {
                    if let Some(ref value) = action.value {
                        if let Ok(elem) = page_ref.find_element(selector).await {
                            let _ = elem.click().await;
                            let _ = elem.type_str(value).await;
                        } else {
                            let js = format!(
                                "const el = document.querySelector('{}'); if(el) {{ el.value = '{}'; el.dispatchEvent(new Event('input')); }}",
                                selector.replace('\'', "\\'"),
                                value.replace('\'', "\\'")
                            );
                            let _ = page_ref.evaluate(js).await;
                        }
                    }
                    let _ = page_ref;
                    drop(page);
                    self.snapshot().await
                } else {
                    Err(anyhow::anyhow!("type requires a selector"))
                }
            }
            "wait" => self.snapshot().await,
            "snapshot" => self.snapshot().await,
            other => Err(anyhow::anyhow!("Unknown action: {}", other)),
        }
    }

    pub async fn click_element(&self, selector: &str) -> anyhow::Result<String> {
        let page = self.page.read().await;
        let page_ref = page
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active page"))?;

        if let Ok(elem) = page_ref.find_element(selector).await {
            let _ = elem.click().await;
            Ok(format!("Clicked: {}", selector))
        } else {
            Err(anyhow::anyhow!("Element not found: {}", selector))
        }
    }

    pub async fn get_text(&self) -> anyhow::Result<String> {
        let page = self.page.read().await;
        let page_ref = page
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No active page"))?;

        let html = page_ref.content().await.unwrap_or_default();
        Ok(self.extract_visible_text(&html))
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    pub async fn is_visible(&self) -> bool {
        *self.visible.read().await
    }

    fn extract_visible_text(&self, html: &str) -> String {
        let doc = Html::parse_document(html);
        let mut text = String::new();
        for node in doc.root_element().descendants() {
            let mut is_hidden = false;
            let mut current = node;
            loop {
                if current.parent().is_none() {
                    break;
                }
                current = current.parent().unwrap_or(current);
                if let Some(el) = scraper::ElementRef::wrap(current) {
                    let tag = el.value().name();
                    if matches!(tag, "script" | "style" | "noscript" | "svg" | "head") {
                        is_hidden = true;
                        break;
                    }
                }
            }
            if is_hidden {
                continue;
            }
            if let Some(t) = node.value().as_text() {
                let trimmed = t.text.trim();
                if !trimmed.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
            }
        }
        text
    }
}
