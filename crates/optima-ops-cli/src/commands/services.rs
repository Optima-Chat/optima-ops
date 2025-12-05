//! Services command - health checks and status

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use comfy_table::{presets::UTF8_FULL, Table, Cell, Color};
use optima_ops_core::{AppConfig, ServiceType};
use reqwest::Client;
use std::time::Duration;

#[derive(Args)]
pub struct ServicesCommand {
    #[command(subcommand)]
    command: ServicesSubcommand,
}

#[derive(Subcommand)]
enum ServicesSubcommand {
    /// Check health of all services
    Health(HealthCommand),

    /// List all configured services
    List(ListCommand),
}

#[derive(Args)]
struct HealthCommand {
    /// Filter by service name
    #[arg(short, long)]
    service: Option<String>,

    /// Filter by service type (core, mcp)
    #[arg(short = 't', long, default_value = "all")]
    r#type: String,
}

#[derive(Args)]
struct ListCommand {
    /// Filter by service type (core, mcp)
    #[arg(short = 't', long, default_value = "all")]
    r#type: String,
}

impl ServicesCommand {
    pub async fn execute(&self, config: &AppConfig, json: bool) -> Result<()> {
        match &self.command {
            ServicesSubcommand::Health(cmd) => cmd.execute(config, json).await,
            ServicesSubcommand::List(cmd) => cmd.execute(config, json).await,
        }
    }
}

impl HealthCommand {
    async fn execute(&self, config: &AppConfig, json: bool) -> Result<()> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        let services = match self.r#type.as_str() {
            "core" => config.get_services_by_type(ServiceType::Core),
            "mcp" => config.get_services_by_type(ServiceType::MCP),
            _ => config.get_all_services(),
        };

        // Filter by name if specified
        let services: Vec<_> = if let Some(ref name) = self.service {
            services.into_iter().filter(|s| s.name.contains(name)).collect()
        } else {
            services
        };

        if services.is_empty() {
            println!("{}", "没有找到匹配的服务".yellow());
            return Ok(());
        }

        println!("{} 正在检查 {} 个服务的健康状态...\n", "⏳".cyan(), services.len());

        let mut results = Vec::new();

        for service in &services {
            let start = std::time::Instant::now();
            let result = client.get(&service.health_endpoint).send().await;
            let elapsed = start.elapsed().as_millis();

            let (status, status_text) = match result {
                Ok(resp) if resp.status().is_success() => ("healthy", "✓ 健康".green()),
                Ok(resp) => ("unhealthy", format!("✗ HTTP {}", resp.status()).red()),
                Err(_) => ("unhealthy", "✗ 无响应".red()),
            };

            results.push((service.name.clone(), status, status_text.to_string(), elapsed));
        }

        if json {
            let json_results: Vec<_> = results.iter()
                .map(|(name, status, _, time)| {
                    serde_json::json!({
                        "name": name,
                        "status": status,
                        "response_time_ms": time
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_results)?);
        } else {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["服务", "状态", "响应时间"]);

            for (name, status, status_text, time) in &results {
                let color = if *status == "healthy" { Color::Green } else { Color::Red };
                table.add_row(vec![
                    Cell::new(name),
                    Cell::new(status_text).fg(color),
                    Cell::new(format!("{}ms", time)),
                ]);
            }

            println!("{table}");

            // Summary
            let healthy_count = results.iter().filter(|(_, s, _, _)| *s == "healthy").count();
            let total = results.len();

            println!();
            if healthy_count == total {
                println!("{} 所有服务运行正常", "✓".green().bold());
            } else {
                println!("{} {}/{} 服务健康", "⚠".yellow().bold(), healthy_count, total);
            }
        }

        Ok(())
    }
}

impl ListCommand {
    async fn execute(&self, config: &AppConfig, json: bool) -> Result<()> {
        let services = match self.r#type.as_str() {
            "core" => config.get_services_by_type(ServiceType::Core),
            "mcp" => config.get_services_by_type(ServiceType::MCP),
            _ => config.get_all_services(),
        };

        if json {
            let json_services: Vec<_> = services.iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "type": format!("{:?}", s.service_type),
                        "container": s.container,
                        "health_endpoint": s.health_endpoint,
                        "port": s.port
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_services)?);
        } else {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["服务", "类型", "容器", "端口"]);

            for service in &services {
                let type_str = match service.service_type {
                    ServiceType::Core => "Core",
                    ServiceType::MCP => "MCP",
                };
                table.add_row(vec![
                    &service.name,
                    type_str,
                    &service.container,
                    &service.port.map(|p| p.to_string()).unwrap_or_default(),
                ]);
            }

            println!("{table}");
            println!();
            println!("共 {} 个服务", services.len());
        }

        Ok(())
    }
}
