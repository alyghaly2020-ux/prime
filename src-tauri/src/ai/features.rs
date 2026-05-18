use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelFeature {
    ToolCalling,
    Streaming,
    Reasoning,
    FunctionCalling,
    Vision,
    Embeddings,
    Reranking,
    StructuredOutput,
    CodeInterpreter,
}

pub struct FeatureManager {
    model_features: RwLock<HashMap<String, Vec<ModelFeature>>>,
}

impl Default for FeatureManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureManager {
    pub fn new() -> Self {
        let mut map = HashMap::new();

        map.insert(
            "gpt-5".to_string(),
            vec![
                ModelFeature::ToolCalling,
                ModelFeature::Streaming,
                ModelFeature::Reasoning,
                ModelFeature::FunctionCalling,
                ModelFeature::Vision,
                ModelFeature::Embeddings,
                ModelFeature::StructuredOutput,
            ],
        );

        map.insert(
            "claude-4".to_string(),
            vec![
                ModelFeature::ToolCalling,
                ModelFeature::Streaming,
                ModelFeature::Reasoning,
                ModelFeature::Vision,
                ModelFeature::StructuredOutput,
            ],
        );

        map.insert(
            "gemini-2".to_string(),
            vec![
                ModelFeature::Streaming,
                ModelFeature::Vision,
                ModelFeature::Embeddings,
                ModelFeature::FunctionCalling,
            ],
        );

        map.insert(
            "groq-llama".to_string(),
            vec![
                ModelFeature::ToolCalling,
                ModelFeature::Streaming,
                ModelFeature::FunctionCalling,
            ],
        );

        Self {
            model_features: RwLock::new(map),
        }
    }

    pub fn supports(&self, model: &str, feature: &ModelFeature) -> bool {
        self.model_features
            .read()
            .get(model)
            .map(|features| features.contains(feature))
            .unwrap_or(false)
    }

    pub fn register(&self, model: String, features: Vec<ModelFeature>) {
        self.model_features.write().insert(model, features);
    }
}
