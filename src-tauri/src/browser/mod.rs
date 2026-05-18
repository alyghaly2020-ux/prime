//! Browser automation engine. Controls Playwright for navigation, clicking, typing and screenshots. Extracts DOM content, accessibility trees, performs OCR and vision-based analysis.

pub mod a11y;
pub mod dom;
pub mod ocr;
pub mod playwright;
pub mod vision;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    pub action_type: String,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub url: Option<String>,
    pub wait_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub html: String,
    pub text: String,
    pub a11y_tree: Option<String>,
    pub screenshot: Option<String>,
}

pub struct System {
    pub playwright: Arc<playwright::PlaywrightEngine>,
    pub dom: Arc<dom::DomParser>,
    pub a11y: Arc<a11y::AccessibilityTree>,
    pub ocr: Arc<ocr::OcrEngine>,
    pub vision: Arc<vision::VisionEngine>,
}

impl std::fmt::Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System").finish_non_exhaustive()
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    pub fn new() -> Self {
        Self {
            playwright: Arc::new(playwright::PlaywrightEngine::new()),
            dom: Arc::new(dom::DomParser::new()),
            a11y: Arc::new(a11y::AccessibilityTree::new()),
            ocr: Arc::new(ocr::OcrEngine::new()),
            vision: Arc::new(vision::VisionEngine::new()),
        }
    }

    pub async fn navigate(&self, url: &str) -> anyhow::Result<PageSnapshot> {
        self.playwright.navigate(url).await
    }

    pub async fn snapshot(&self) -> anyhow::Result<PageSnapshot> {
        self.playwright.snapshot().await
    }

    pub async fn execute(&self, action: &BrowserAction) -> anyhow::Result<PageSnapshot> {
        self.playwright.execute(action).await
    }

    pub async fn extract_text(&self, html: &str) -> String {
        self.dom.extract_text(html)
    }

    pub async fn read_screen_text(&self, image: &[u8]) -> String {
        self.ocr.recognize(image).await
    }
}
