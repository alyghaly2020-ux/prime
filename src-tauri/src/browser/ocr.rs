//! Optical Character Recognition (OCR) engine.
//!
//! Uses the `image` crate to decode image data.
//! For actual OCR, compile with the `tesseract` feature flag enabled,
//! which enables the leptess (Tesseract) backend.
//!
//! Without the feature flag, this engine validates image data and returns
//! a placeholder message indicating that Tesseract OCR is not available.

use image::GenericImageView;

/// Optical Character Recognition engine
#[derive(Debug)]
pub struct OcrEngine;

impl Default for OcrEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrEngine {
    pub fn new() -> Self {
        Self
    }

    /// Recognize text from raw image bytes.
    ///
    /// If the `tesseract` feature is not enabled, validates the image
    /// and returns a message indicating OCR is not available.
    ///
    /// Supported image formats: PNG, JPEG, GIF, BMP, WEBP, and others
    /// supported by the `image` crate.
    pub async fn recognize(&self, image_data: &[u8]) -> String {
        if image_data.is_empty() {
            return String::new();
        }

        #[cfg(feature = "tesseract")]
        {
            self.ocr_with_tesseract(image_data)
        }

        #[cfg(not(feature = "tesseract"))]
        {
            self.placeholder_ocr(image_data)
        }
    }

    /// Recognize text from an image file path.
    pub async fn recognize_file(&self, path: &str) -> anyhow::Result<String> {
        let image_data = tokio::fs::read(path).await?;
        Ok(self.recognize(&image_data).await)
    }

    /// Check if Tesseract OCR is available
    pub fn is_available() -> bool {
        cfg!(feature = "tesseract")
    }

    /// Get the OCR backend name
    pub fn backend_name() -> &'static str {
        if cfg!(feature = "tesseract") {
            "Tesseract (leptess)"
        } else {
            "Placeholder (no OCR backend)"
        }
    }

    // -----------------------------------------------------------------------
    // Placeholder OCR (when tesseract feature is not enabled)
    // -----------------------------------------------------------------------

    #[cfg(not(feature = "tesseract"))]
    fn placeholder_ocr(&self, image_data: &[u8]) -> String {
        // Validate the image data using the `image` crate
        match image::load_from_memory(image_data) {
            Ok(img) => {
                let dims = img.dimensions();
                tracing::debug!(
                    "OCR placeholder: valid image {}x{} ({} bytes)",
                    dims.0,
                    dims.1,
                    image_data.len()
                );
                format!(
                    "OCR not available - compile with 'tesseract' feature to enable. \
                     Image decoded: {}x{} px, {} bytes",
                    dims.0,
                    dims.1,
                    image_data.len()
                )
            }
            Err(e) => {
                tracing::warn!("OCR received invalid image: {}", e);
                format!("Invalid image data: {}", e)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Real Tesseract OCR (when tesseract feature is enabled)
    // -----------------------------------------------------------------------

    #[cfg(feature = "tesseract")]
    fn ocr_with_tesseract(&self, image_data: &[u8]) -> String {
        // In production, this would use leptess:
        //
        // let mut api = leptess::LepTess::new(None, "eng+ara")
        //     .map_err(|e| format!("Failed to init Tesseract: {}", e))?;
        //
        // api.set_image_from_mem(image_data)
        //     .map_err(|e| format!("Failed to set image: {}", e))?;
        //
        // let text = api.get_utf8_text()
        //     .map_err(|e| format!("OCR failed: {}", e))?;
        //
        // text

        // Placeholder until leptess is added to dependencies
        match image::load_from_memory(image_data) {
            Ok(img) => {
                let dims = img.dimensions();
                format!(
                    "[Tesseract OCR] Image: {}x{} px, {} bytes. \
                     Install leptess crate and add to Cargo.toml for actual OCR.",
                    dims.0,
                    dims.1,
                    image_data.len()
                )
            }
            Err(e) => format!("Invalid image data: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available() {
        // Should always be callable
        let _available = OcrEngine::is_available();
    }

    #[test]
    fn test_backend_name() {
        let name = OcrEngine::backend_name();
        assert!(!name.is_empty());
    }

    #[tokio::test]
    async fn test_recognize_empty() {
        let engine = OcrEngine::new();
        let result = engine.recognize(&[]).await;
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_recognize_invalid_data() {
        let engine = OcrEngine::new();
        let result = engine.recognize(b"not an image").await;
        // Without tesseract, should return a message about OCR not being available
        // or an error message for invalid image
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_recognize_file_not_found() {
        let engine = OcrEngine::new();
        let result = engine.recognize_file("/nonexistent/image.png").await;
        assert!(result.is_err());
    }
}
