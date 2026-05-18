use tokio::process::Command;

pub struct ToolRunner;

impl ToolRunner {
    pub async fn install_pip(package: &str) -> anyhow::Result<String> {
        let output = Command::new("pip")
            .args(["install", package])
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "pip install '{}' failed: {}",
                package,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn install_npm(package: &str) -> anyhow::Result<String> {
        let output = Command::new("npm")
            .args(["install", "-g", package])
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "npm install '{}' failed: {}",
                package,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn run_docker(image: &str, port: u16) -> anyhow::Result<String> {
        let output = Command::new("docker")
            .args(["run", "-d", "--rm", "-p", &format!("{}:{}", port, port), image])
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "docker run '{}' failed: {}",
                image,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn stop_docker(container_id: &str) -> anyhow::Result<String> {
        let output = Command::new("docker")
            .args(["stop", container_id])
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "docker stop '{}' failed: {}",
                container_id,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn check_health(url: &str) -> bool {
        reqwest::get(url)
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn is_installed(cmd: &str) -> bool {
        let check = if cfg!(windows) { "where" } else { "which" };
        Command::new(check)
            .arg(cmd)
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub async fn run_command(cmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let output = Command::new(cmd)
            .args(args)
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Command '{} {:?}' failed: {}",
                cmd,
                args,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn execute_tool(config: &crate::tools::config::ToolConfig) -> anyhow::Result<String> {
        use crate::tools::config::ToolSource;

        let run_cmd = config.run_cmd.as_deref()
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' has no run command", config.id))?;

        match config.source {
            ToolSource::Pip => {
                let parts: Vec<&str> = run_cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(anyhow::anyhow!("Empty run command for tool '{}'", config.id));
                }
                Self::run_command(parts[0], &parts[1..]).await
            }
            ToolSource::Npm | ToolSource::Mcp => {
                let parts: Vec<&str> = run_cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(anyhow::anyhow!("Empty run command for tool '{}'", config.id));
                }
                if parts[0] == "npx" {
                    Self::npx(parts.get(1).unwrap_or(&""), &parts[2..]).await
                } else {
                    Self::run_command(parts[0], &parts[1..]).await
                }
            }
            ToolSource::Docker => {
                let port = config.port.unwrap_or(8080);
                let image = run_cmd.split_whitespace()
                    .last()
                    .unwrap_or(run_cmd);
                Self::run_docker(image, port).await
            }
            ToolSource::Binary | ToolSource::Rust => {
                let parts: Vec<&str> = run_cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(anyhow::anyhow!("Empty run command for tool '{}'", config.id));
                }
                Self::run_command(parts[0], &parts[1..]).await
            }
            ToolSource::BuiltIn => {
                Ok(format!("Built-in tool '{}' is always available", config.id))
            }
        }
    }

    pub async fn install_tool(config: &crate::tools::config::ToolConfig) -> anyhow::Result<String> {
        use crate::tools::config::ToolSource;

        let install_cmd = config.install_cmd.as_deref()
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' has no install command", config.id))?;

        match config.source {
            ToolSource::Pip => Self::install_pip(install_cmd.trim_start_matches("pip install ")).await,
            ToolSource::Npm | ToolSource::Mcp => {
                let pkg = install_cmd.trim_start_matches("npm install -g ")
                    .trim_start_matches("npm install ")
                    .trim_start_matches("npx ");
                Self::install_npm(pkg).await
            }
            ToolSource::Docker => {
                let parts: Vec<&str> = install_cmd.split_whitespace().collect();
                Self::run_command(parts[0], &parts[1..]).await
            }
            ToolSource::Binary | ToolSource::Rust => {
                let parts: Vec<&str> = install_cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(anyhow::anyhow!("Empty install command"));
                }
                Self::run_command(parts[0], &parts[1..]).await
            }
            ToolSource::BuiltIn => Ok("Built-in tool, no installation needed".to_string()),
        }
    }

    pub async fn npx(package: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut all_args = vec![package];
        all_args.extend_from_slice(args);
        let output = Command::new("npx")
            .args(&all_args)
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "npx '{}' failed: {}",
                package,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}
