use serde::Serialize;
use std::sync::Arc;

use crate::tools::config::ToolConfig;
use crate::tools::registry::ToolRegistry;

#[derive(Serialize)]
pub struct ToolsListResult {
    pub tools: Vec<ToolConfig>,
    pub total: usize,
    pub by_category: Vec<CategoryCount>,
}

#[derive(Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

fn parse_category(category: &str) -> Result<crate::tools::config::ToolCategory, crate::AppError> {
    use crate::tools::config::ToolCategory;
    match category.to_lowercase().as_str() {
        "tokencompression" => Ok(ToolCategory::TokenCompression),
        "browserstealth" => Ok(ToolCategory::BrowserStealth),
        "apigateway" => Ok(ToolCategory::ApiGateway),
        "promptobfuscation" => Ok(ToolCategory::PromptObfuscation),
        "proxyinfrastructure" => Ok(ToolCategory::ProxyInfrastructure),
        "identitymasking" => Ok(ToolCategory::IdentityMasking),
        "swarmorchestration" => Ok(ToolCategory::SwarmOrchestration),
        "monetization" => Ok(ToolCategory::Monetization),
        "offensivecyber" => Ok(ToolCategory::OffensiveCyber),
        "proxyip" => Ok(ToolCategory::ProxyIp),
        "ipv6blocks" => Ok(ToolCategory::Ipv6Blocks),
        "sshremotedesktop" => Ok(ToolCategory::SshRemoteDesktop),
        "servermanagement" => Ok(ToolCategory::ServerManagement),
        "aiproviderintegration" => Ok(ToolCategory::AiProviderIntegration),
        "communicationplatform" => Ok(ToolCategory::CommunicationPlatform),
        "mcpskills" => Ok(ToolCategory::McpSkills),
        "infrastructure" => Ok(ToolCategory::Infrastructure),
        "searchengine" => Ok(ToolCategory::SearchEngine),
        "contentfetching" => Ok(ToolCategory::ContentFetching),
        "embeddingsvectordb" => Ok(ToolCategory::EmbeddingsVectorDb),
        "memorygraph" => Ok(ToolCategory::MemoryGraph),
        "localmodels" => Ok(ToolCategory::LocalModels),
        "agentorchestration" => Ok(ToolCategory::AgentOrchestration),
        "ragengine" => Ok(ToolCategory::RagEngine),
        "referenceui" => Ok(ToolCategory::ReferenceUi),
        _ => Err(crate::AppError::Workspace(format!("Unknown category: {}", category))),
    }
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn list_all_tools(
    registry: tauri::State<'_, Arc<ToolRegistry>>,
) -> Result<String, crate::AppError> {
    let tools = registry.list_all().await;
    let by_category_raw = registry.count_by_category().await;
    let by_category: Vec<CategoryCount> = by_category_raw
        .into_iter()
        .map(|(category, count)| CategoryCount { category, count })
        .collect();
    let result = ToolsListResult {
        total: tools.len(),
        by_category,
        tools,
    };
    serde_json::to_string(&result).map_err(|e| crate::AppError::Workspace(e.to_string()))
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn get_tool(
    registry: tauri::State<'_, Arc<ToolRegistry>>,
    id: String,
) -> Result<String, crate::AppError> {
    let tool = registry.get(&id).await;
    match tool {
        Some(t) => serde_json::to_string(&t).map_err(|e| crate::AppError::Workspace(e.to_string())),
        None => Err(crate::AppError::Workspace(format!("Tool '{}' not found", id))),
    }
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn search_tools(
    registry: tauri::State<'_, Arc<ToolRegistry>>,
    query: String,
) -> Result<String, crate::AppError> {
    let tools = registry.search(&query).await;
    serde_json::to_string(&tools).map_err(|e| crate::AppError::Workspace(e.to_string()))
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn toggle_tool(
    registry: tauri::State<'_, Arc<ToolRegistry>>,
    id: String,
    enabled: bool,
) -> Result<(), crate::AppError> {
    registry.set_enabled(&id, enabled).await;
    Ok(())
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn enable_tool_category(
    registry: tauri::State<'_, Arc<ToolRegistry>>,
    category: String,
) -> Result<(), crate::AppError> {
    let cat = parse_category(&category)?;
    registry.enable_category(&cat).await;
    Ok(())
}
