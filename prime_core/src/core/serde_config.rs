use std::collections::HashMap;

pub struct SerdeRegistry {
    formats: HashMap<String, SerdeFormat>,
}

enum SerdeFormat {
    Json,
    Yaml,
    Toml,
    MessagePack,
}

impl Default for SerdeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SerdeRegistry {
    pub fn new() -> Self {
        let mut formats = HashMap::new();
        formats.insert("json".to_string(), SerdeFormat::Json);
        formats.insert("yaml".to_string(), SerdeFormat::Yaml);
        formats.insert("toml".to_string(), SerdeFormat::Toml);
        formats.insert("msgpack".to_string(), SerdeFormat::MessagePack);
        Self { formats }
    }

    pub fn serialize<T: serde::Serialize>(
        &self,
        value: &T,
        format: &str,
    ) -> anyhow::Result<String> {
        match self
            .formats
            .get(format)
            .ok_or_else(|| anyhow::anyhow!("Unknown format: {}", format))?
        {
            SerdeFormat::Json => serde_json::to_string_pretty(value).map_err(Into::into),
            SerdeFormat::Yaml => serde_yaml::to_string(value).map_err(Into::into),
            SerdeFormat::Toml => toml::to_string_pretty(value).map_err(Into::into),
            SerdeFormat::MessagePack => {
                let bytes = rmp_serde::to_vec(value)?;
                Ok(String::from_utf8_lossy(&bytes).to_string())
            }
        }
    }

    pub fn deserialize<T: serde::de::DeserializeOwned>(
        &self,
        input: &str,
        format: &str,
    ) -> anyhow::Result<T> {
        match self
            .formats
            .get(format)
            .ok_or_else(|| anyhow::anyhow!("Unknown format: {}", format))?
        {
            SerdeFormat::Json => serde_json::from_str(input).map_err(Into::into),
            SerdeFormat::Yaml => serde_yaml::from_str(input).map_err(Into::into),
            SerdeFormat::Toml => toml::from_str(input).map_err(Into::into),
            SerdeFormat::MessagePack => {
                let bytes = input.as_bytes();
                Ok(rmp_serde::from_slice(bytes)?)
            }
        }
    }
}
