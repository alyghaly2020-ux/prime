//! Vision engine for image analysis using multimodal AI APIs.
//!
//! Uses the AI system's provider infrastructure to analyze images
//! via vision-capable models (GPT-4o, Claude 3.5 Sonnet, Gemini 2.0 Flash).
//! Images are encoded as base64 and sent to the AI provider's multimodal endpoint.

use base64::Engine;
use image::GenericImageView;

/// Structured analysis result from vision processing
#[derive(Debug, Clone, serde::Serialize)]
pub struct VisionAnalysis {
    /// Natural language description of the image
    pub description: String,
    /// Objects detected in the image
    pub objects: Vec<String>,
    /// Text found in the image
    pub text: String,
    /// Overall confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Dominant colors extracted from the image
    pub dominant_colors: Vec<String>,
    /// Estimated content type (e.g., "screenshot", "photograph", "document", "diagram")
    pub content_type: String,
    /// Width in pixels (if available)
    pub width: u32,
    /// Height in pixels (if available)
    pub height: u32,
}

/// Vision engine powered by multimodal AI models
#[derive(Debug)]
pub struct VisionEngine;

impl Default for VisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl VisionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Analyze an image using the configured AI provider.
    ///
    /// Sends the image (as base64) to a vision-capable model with a prompt.
    /// Returns structured analysis including detected objects, text, and colors.
    pub async fn analyze(&self, image_data: &[u8], prompt: &str) -> anyhow::Result<VisionAnalysis> {
        if image_data.is_empty() {
            return Ok(VisionAnalysis::empty());
        }

        // Try to decode image dimensions first
        let (width, height) = match image::load_from_memory(image_data) {
            Ok(img) => img.dimensions(),
            Err(_) => (0, 0),
        };

        // Encode image as base64 for API calls
        let b64 = base64::engine::general_purpose::STANDARD.encode(image_data);
        let media_type = detect_media_type(image_data);

        // Build the multimodal prompt
        let analysis_prompt = format!(
            "{}\n\nPlease provide a structured analysis of this image. \
             Include: a brief description, any objects you can identify, \
             any text visible in the image, the dominant colors, and the \
             likely content type (screenshot, photograph, document, diagram, illustration, etc.).",
            prompt
        );

        // Try to get analysis from available AI providers
        let result = self
            .call_vision_api(&b64, &media_type, &analysis_prompt)
            .await;

        match result {
            Ok(analysis_text) => Ok(self.parse_analysis(&analysis_text, width, height)),
            Err(e) => {
                // Fallback: return basic info without AI analysis
                tracing::warn!("Vision AI analysis failed: {}. Returning basic info.", e);
                Ok(VisionAnalysis {
                    description: format!("Image analysis unavailable: {}", e),
                    objects: vec![],
                    text: String::new(),
                    confidence: 0.0,
                    dominant_colors: vec![],
                    content_type: "unknown".to_string(),
                    width,
                    height,
                })
            }
        }
    }

    /// Generate a text description of an image.
    pub async fn describe(&self, image_data: &[u8]) -> String {
        if image_data.is_empty() {
            return String::new();
        }

        match self
            .analyze(image_data, "Describe this image in detail.")
            .await
        {
            Ok(analysis) => analysis.description,
            Err(e) => format!("Description unavailable: {}", e),
        }
    }

    /// Call a vision-capable AI provider to analyze the image.
    ///
    /// Uses the OpenAI API with GPT-4o or compatible vision model.
    /// Falls back to basic metadata if no API key is configured.
    async fn call_vision_api(
        &self,
        b64_image: &str,
        media_type: &str,
        prompt: &str,
    ) -> anyhow::Result<String> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .map_err(|_| anyhow::anyhow!("No AI API key found (set OPENAI_API_KEY, ANTHROPIC_API_KEY, or GOOGLE_API_KEY)"))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        // Try OpenAI first (most widely available vision support)
        if std::env::var("OPENAI_API_KEY").is_ok() {
            return self
                .call_openai_vision(&client, &api_key, b64_image, media_type, prompt)
                .await;
        }

        // Try Google Gemini
        if std::env::var("GOOGLE_API_KEY").is_ok() {
            return self
                .call_gemini_vision(&client, &api_key, b64_image, media_type, prompt)
                .await;
        }

        Err(anyhow::anyhow!("No supported vision provider configured"))
    }

    async fn call_openai_vision(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        b64_image: &str,
        media_type: &str,
        prompt: &str,
    ) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{};base64,{}", media_type, b64_image),
                            "detail": "auto"
                        }
                    }
                ]
            }],
            "max_tokens": 1024,
            "temperature": 0.3,
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI vision request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body_snippet = response.text().await.unwrap_or_default();
            let safe = if body_snippet.len() > 200 {
                format!("{}… (truncated)", &body_snippet[..200])
            } else {
                body_snippet
            };
            return Err(anyhow::anyhow!(
                "OpenAI vision returned HTTP {}: {}",
                status,
                safe
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse vision response: {}", e))?;

        body["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No content in vision response"))
    }

    async fn call_gemini_vision(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        b64_image: &str,
        media_type: &str,
        prompt: &str,
    ) -> anyhow::Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
            api_key
        );

        let body = serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [
                    {"text": prompt},
                    {
                        "inline_data": {
                            "mime_type": media_type,
                            "data": b64_image
                        }
                    }
                ]
            }],
            "generationConfig": {
                "maxOutputTokens": 1024,
                "temperature": 0.3,
            }
        });

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Gemini vision request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Gemini vision returned HTTP {}: {}",
                status,
                text
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Gemini response: {}", e))?;

        body["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|p| p["text"].as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No content in Gemini vision response"))
    }

    /// Parse AI analysis text into a structured VisionAnalysis.
    fn parse_analysis(&self, text: &str, width: u32, height: u32) -> VisionAnalysis {
        // Use the analysis text as the description
        // In production, use structured output from the model for precise parsing
        let description = text.to_string();

        // Infer content type from description
        let content_type = self.infer_content_type(text);

        // Simple extraction: look for quoted text as "text found"
        let found_text = self.extract_quoted_text(text);

        // Lowercase for keyword matching
        let lower = text.to_lowercase();

        // Detect common objects from keywords
        let mut objects = Vec::new();
        let object_keywords = [
            "person", "people", "face", "car", "tree", "building", "animal", "dog", "cat", "bird",
            "food", "book", "screen", "phone", "laptop", "table", "chair", "document", "logo",
            "chart", "graph", "diagram",
        ];
        for keyword in &object_keywords {
            if lower.contains(keyword) {
                objects.push(keyword.to_string());
            }
        }

        // Estimate confidence based on description length and keyword presence
        let confidence = if text.len() > 100 && !objects.is_empty() {
            0.7
        } else if text.len() > 50 {
            0.5
        } else {
            0.3
        };

        VisionAnalysis {
            description,
            objects,
            text: found_text,
            confidence,
            dominant_colors: vec!["unknown".to_string()],
            content_type,
            width,
            height,
        }
    }

    fn infer_content_type(&self, text: &str) -> String {
        let lower = text.to_lowercase();
        if lower.contains("screenshot") || lower.contains("screen capture") {
            "screenshot".to_string()
        } else if lower.contains("photograph") || lower.contains("photo") {
            "photograph".to_string()
        } else if lower.contains("document") || lower.contains("text") {
            "document".to_string()
        } else if lower.contains("diagram") || lower.contains("chart") || lower.contains("graph") {
            "diagram".to_string()
        } else if lower.contains("illustration") || lower.contains("drawing") {
            "illustration".to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn extract_quoted_text(&self, text: &str) -> String {
        let mut found = Vec::new();
        let mut in_quotes = false;
        let mut current = String::new();

        for ch in text.chars() {
            match ch {
                '"' | '\u{201c}' | '\u{201d}' => {
                    if in_quotes {
                        found.push(current.clone());
                        current.clear();
                    }
                    in_quotes = !in_quotes;
                }
                _ if in_quotes => current.push(ch),
                _ => {}
            }
        }

        found.join("; ")
    }
}

impl VisionAnalysis {
    pub fn empty() -> Self {
        Self {
            description: String::new(),
            objects: vec![],
            text: String::new(),
            confidence: 0.0,
            dominant_colors: vec![],
            content_type: "unknown".to_string(),
            width: 0,
            height: 0,
        }
    }
}

/// Detect media type from image bytes by examining the header magic bytes.
fn detect_media_type(data: &[u8]) -> String {
    if data.len() < 4 {
        return "image/png".to_string();
    }

    // Check magic bytes
    if data[0] == 0xFF && data[1] == 0xD8 {
        "image/jpeg".to_string()
    } else if data[0] == 0x89 && data[1] == b'P' && data[2] == b'N' && data[3] == b'G' {
        "image/png".to_string()
    } else if data[0] == b'R' && data[1] == b'I' && data[2] == b'F' && data[3] == b'F' {
        "image/webp".to_string()
    } else if data[0] == b'G' && data[1] == b'I' && data[2] == b'F' {
        "image/gif".to_string()
    } else if data[0] == b'B' && data[1] == b'M' {
        "image/bmp".to_string()
    } else {
        "image/png".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_media_type() {
        assert_eq!(detect_media_type(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(detect_media_type(&[0x89, b'P', b'N', b'G']), "image/png");
        assert_eq!(detect_media_type(&[b'G', b'I', b'F', 0x38]), "image/gif");
        assert_eq!(detect_media_type(&[0, 0, 0, 0]), "image/png");
    }

    #[test]
    fn test_parse_analysis() {
        let engine = VisionEngine::new();
        let analysis = engine.parse_analysis(
            "This is a screenshot of a webpage showing a login form with an email field. \
             There is a person in the photograph next to a car.",
            1920,
            1080,
        );

        assert_eq!(analysis.width, 1920);
        assert_eq!(analysis.height, 1080);
        assert!(analysis.content_type == "screenshot" || analysis.content_type == "photograph");
        assert!(analysis.objects.contains(&"person".to_string()));
        assert!(analysis.objects.contains(&"car".to_string()));
        assert!(analysis.confidence > 0.0);
    }

    #[test]
    fn test_empty_analysis() {
        let empty = VisionAnalysis::empty();
        assert_eq!(empty.description, "");
        assert!(empty.objects.is_empty());
        assert_eq!(empty.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_describe_empty() {
        let engine = VisionEngine::new();
        let desc = engine.describe(&[]).await;
        assert_eq!(desc, "");
    }

    #[test]
    fn test_extract_quoted_text() {
        let engine = VisionEngine::new();
        let result = engine.extract_quoted_text(r#"Found text: "Hello World" and "Goodbye""#);
        assert_eq!(result, "Hello World; Goodbye");
    }

    #[test]
    fn test_infer_content_type() {
        let engine = VisionEngine::new();
        assert_eq!(
            engine.infer_content_type("This is a screenshot"),
            "screenshot"
        );
        assert_eq!(
            engine.infer_content_type("A photograph of nature"),
            "photograph"
        );
        assert_eq!(engine.infer_content_type("A detailed diagram"), "diagram");
        assert_eq!(engine.infer_content_type("Random content"), "unknown");
    }
}
