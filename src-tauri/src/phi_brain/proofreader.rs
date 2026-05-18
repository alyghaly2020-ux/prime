/// T2+T3: Proofreader + Hallucination Guard.
///
/// Reviews AI responses before sending to user, checking for:
/// - Spelling/grammar mistakes
/// - Factual accuracy (hallucinations)
/// - Payment/crypto verification
/// - Context contradictions

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::ai::ChatMessage;
use crate::phi_brain::client::OllamaClient;

/// Type of correction found.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionKind {
    Spelling,
    Grammar,
    Factual,
    Payment,
    Contradiction,
}

/// A single correction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub kind: CorrectionKind,
    pub original: String,
    pub fixed: String,
    pub confidence: f32,
}

/// Result of proofreading an AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResult {
    pub corrected_text: String,
    pub corrections: Vec<Correction>,
    pub hallucination_score: f32,
    pub was_modified: bool,
}

/// Proofreader that reviews AI responses using Phi Brain.
pub struct Proofreader {
    client: Arc<OllamaClient>,
    enabled: bool,
}

impl Proofreader {
    pub fn new() -> Self {
        Self {
            client: OllamaClient::new().shared(),
            enabled: std::env::var("PHI_BRAIN_PROOFREAD")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
        }
    }

    /// Create with a shared OllamaClient.
    pub fn with_client(client: Arc<OllamaClient>) -> Self {
        Self {
            client,
            enabled: std::env::var("PHI_BRAIN_PROOFREAD")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
        }
    }

    /// Check if proofreading is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Review an AI response for errors.
    pub async fn review(
        &self,
        response: &str,
        context: &[ChatMessage],
    ) -> ProofResult {
        if !self.enabled {
            return ProofResult {
                corrected_text: response.to_string(),
                corrections: Vec::new(),
                hallucination_score: 0.0,
                was_modified: false,
            };
        }

        // Quick checks first (no Phi needed)
        let quick_result = self.quick_checks(response, context);
        if quick_result.hallucination_score > 0.8 {
            // High confidence hallucination — flag it
            return quick_result;
        }

        // If Ollama is not available, return quick checks only
        if self.client.check_health().await.is_err() {
            return quick_result;
        }

        // Build context summary
        let context_summary: String = context
            .iter()
            .filter(|m| m.role == "user")
            .map(|m| m.content.chars().take(200).collect::<String>())
            .collect::<Vec<_>>()
            .join(" | ");

        let prompt = format!(
            r#"Review this AI response for errors. Check:
1. Spelling/grammar mistakes
2. Factual accuracy (hallucinations) — flag claims that seem wrong
3. If about payments/crypto: verify addresses, amounts, chain names
4. Contradictions with conversation context

Conversation context:
{context}

Response to review:
---
{response}
---

Reply ONLY in JSON:
{{"corrected": "corrected text or null if no changes", "corrections": [{{"kind":"spelling|grammar|factual|payment|contradiction","original":"...","fixed":"...","confidence":0.0-1.0}}], "hallucination_score": 0.0-1.0}}
"#,
            context = context_summary,
            response = response,
        );

        match self.client.generate(&prompt, 0.1, 512).await {
            Ok(raw_response) => {
                // Try to parse JSON from the response
                if let Ok(proof) = serde_json::from_str::<ProofResult>(&raw_response) {
                    if proof.was_modified || !proof.corrections.is_empty() {
                        tracing::info!(
                            "Phi Brain proofread: {} corrections, hallucination_score={}",
                            proof.corrections.len(),
                            proof.hallucination_score
                        );
                    }
                    proof
                } else {
                    // JSON parsing failed — return quick checks
                    quick_result
                }
            }
            Err(e) => {
                tracing::warn!("Phi Brain proofread failed: {}", e);
                quick_result
            }
        }
    }

    /// Quick checks without calling Phi (fast, rule-based).
    fn quick_checks(&self, response: &str, context: &[ChatMessage]) -> ProofResult {
        let mut corrections = Vec::new();
        let corrected = response.to_string();
        let mut hallucination_score: f32 = 0.0;

        // Check for common hallucination patterns
        let hallucination_indicators = [
            "I'm not entirely sure",
            "I don't have access to",
            "As of my knowledge cutoff",
            "I cannot browse",
            "I don't have real-time",
        ];

        for indicator in &hallucination_indicators {
            if response.contains(indicator) {
                hallucination_score += 0.2;
            }
        }

        // Check for contradictions with context
        for msg in context.iter().filter(|m| m.role == "user") {
            // Simple contradiction detection
            if msg.content.contains("don't use") && response.contains("I'll use") {
                corrections.push(Correction {
                    kind: CorrectionKind::Contradiction,
                    original: "I'll use".to_string(),
                    fixed: "I'll avoid using".to_string(),
                    confidence: 0.7,
                });
            }
        }

        // Check for crypto address patterns (basic validation)
        if response.contains("0x") {
            // Extract potential addresses and validate format
            for word in response.split_whitespace() {
                if word.starts_with("0x") && word.len() == 42 {
                    // Basic Ethereum address format check
                    let hex_part = &word[2..];
                    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                        corrections.push(Correction {
                            kind: CorrectionKind::Payment,
                            original: word.to_string(),
                            fixed: "[INVALID ADDRESS]".to_string(),
                            confidence: 0.9,
                        });
                    }
                }
            }
        }

        hallucination_score = hallucination_score.min(1.0);

        let was_modified = !corrections.is_empty();

        ProofResult {
            corrected_text: corrected,
            corrections,
            hallucination_score,
            was_modified,
        }
    }
}
