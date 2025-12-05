//! Environment and service configuration for Optima Ops Dashboard
//!
//! Defines four environments:
//! - EC2 Prod: Docker Compose on EC2
//! - ECS Stage: ECS cluster for staging
//! - ECS Prod: ECS cluster for production
//! - Shared: Shared infrastructure services

use serde::{Deserialize, Serialize};

/// Environment type for the dashboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentType {
    Ec2Prod,
    EcsStage,
    EcsProd,
    Shared,
}

impl EnvironmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnvironmentType::Ec2Prod => "ec2-prod",
            EnvironmentType::EcsStage => "ecs-stage",
            EnvironmentType::EcsProd => "ecs-prod",
            EnvironmentType::Shared => "shared",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            EnvironmentType::Ec2Prod => "EC2 Prod",
            EnvironmentType::EcsStage => "ECS Stage",
            EnvironmentType::EcsProd => "ECS Prod",
            EnvironmentType::Shared => "Shared",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ec2-prod" => Some(EnvironmentType::Ec2Prod),
            "ecs-stage" => Some(EnvironmentType::EcsStage),
            "ecs-prod" => Some(EnvironmentType::EcsProd),
            "shared" => Some(EnvironmentType::Shared),
            _ => None,
        }
    }

    pub fn all() -> &'static [EnvironmentType] {
        &[
            EnvironmentType::Ec2Prod,
            EnvironmentType::EcsStage,
            EnvironmentType::EcsProd,
            EnvironmentType::Shared,
        ]
    }

    pub fn is_ecs(&self) -> bool {
        matches!(self, EnvironmentType::EcsStage | EnvironmentType::EcsProd)
    }

    pub fn is_ec2(&self) -> bool {
        matches!(self, EnvironmentType::Ec2Prod | EnvironmentType::Shared)
    }
}

impl std::fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Service category within an environment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceCategory {
    Core,
    McpTool,
    BiService,
    Migration,
    Scheduled,
    Infrastructure,
}

impl ServiceCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            ServiceCategory::Core => "Core Services",
            ServiceCategory::McpTool => "MCP Tools",
            ServiceCategory::BiService => "BI Services",
            ServiceCategory::Migration => "Migration Tasks",
            ServiceCategory::Scheduled => "Scheduled Tasks",
            ServiceCategory::Infrastructure => "Infrastructure",
        }
    }
}

/// Service definition
#[derive(Debug, Clone)]
pub struct ServiceDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub category: ServiceCategory,
    pub port: Option<u16>,
    pub container_name: Option<&'static str>,
    pub github_repo: Option<&'static str>,
    pub domain: Option<&'static str>,
}

/// Environment configuration
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    pub env_type: EnvironmentType,
    pub ec2_host: Option<&'static str>,
    pub cluster_name: Option<&'static str>,
    pub domain_suffix: &'static str,
    pub services: Vec<ServiceDef>,
}

impl EnvironmentConfig {
    pub fn get_services_by_category(&self, category: ServiceCategory) -> Vec<&ServiceDef> {
        self.services
            .iter()
            .filter(|s| s.category == category)
            .collect()
    }
}

/// Get all environment configurations
pub fn get_all_environments() -> Vec<EnvironmentConfig> {
    vec![
        get_ec2_prod_config(),
        get_ecs_stage_config(),
        get_ecs_prod_config(),
        get_shared_config(),
    ]
}

/// Get a specific environment configuration
pub fn get_environment(env_type: EnvironmentType) -> EnvironmentConfig {
    match env_type {
        EnvironmentType::Ec2Prod => get_ec2_prod_config(),
        EnvironmentType::EcsStage => get_ecs_stage_config(),
        EnvironmentType::EcsProd => get_ecs_prod_config(),
        EnvironmentType::Shared => get_shared_config(),
    }
}

fn get_ec2_prod_config() -> EnvironmentConfig {
    EnvironmentConfig {
        env_type: EnvironmentType::Ec2Prod,
        ec2_host: Some("ec2-prod.optima.shop"),
        cluster_name: None,
        domain_suffix: ".optima.shop",
        services: vec![
            // Core Services
            ServiceDef {
                name: "user-auth",
                display_name: "User Auth",
                category: ServiceCategory::Core,
                port: Some(8292),
                container_name: Some("optima-user-auth-prod"),
                github_repo: Some("Optima-Chat/user-auth"),
                domain: Some("auth.optima.shop"),
            },
            ServiceDef {
                name: "user-auth-admin",
                display_name: "Auth Admin",
                category: ServiceCategory::Core,
                port: Some(8291),
                container_name: Some("optima-user-auth-admin-prod"),
                github_repo: Some("Optima-Chat/user-auth"),
                domain: Some("portal.admin.optima.shop"),
            },
            ServiceDef {
                name: "commerce-backend",
                display_name: "Commerce",
                category: ServiceCategory::Core,
                port: Some(8293),
                container_name: Some("optima-commerce-backend-prod"),
                github_repo: Some("Optima-Chat/commerce-backend"),
                domain: Some("api.optima.shop"),
            },
            ServiceDef {
                name: "mcp-host",
                display_name: "MCP Host",
                category: ServiceCategory::Core,
                port: Some(8294),
                container_name: Some("optima-mcp-host-prod"),
                github_repo: Some("Optima-Chat/mcp-host"),
                domain: Some("mcp.optima.shop"),
            },
            ServiceDef {
                name: "agentic-chat",
                display_name: "Agentic Chat",
                category: ServiceCategory::Core,
                port: Some(8296),
                container_name: Some("optima-agentic-chat-prod"),
                github_repo: Some("Optima-Chat/agentic-chat"),
                domain: Some("ai.optima.shop"),
            },
            // MCP Tools
            ServiceDef {
                name: "comfy-mcp",
                display_name: "Comfy MCP",
                category: ServiceCategory::McpTool,
                port: Some(8261),
                container_name: Some("optima-comfy-mcp-prod"),
                github_repo: Some("Optima-Chat/comfy-mcp"),
                domain: Some("mcp-comfy.optima.shop"),
            },
            ServiceDef {
                name: "fetch-mcp",
                display_name: "Fetch MCP",
                category: ServiceCategory::McpTool,
                port: Some(8250),
                container_name: Some("optima-fetch-mcp-prod"),
                github_repo: Some("Optima-Chat/fetch-mcp"),
                domain: Some("mcp-fetch.optima.shop"),
            },
            ServiceDef {
                name: "research-mcp",
                display_name: "Research MCP",
                category: ServiceCategory::McpTool,
                port: Some(8220),
                container_name: Some("optima-perplexity-mcp-prod"),
                github_repo: Some("Optima-Chat/perplexity-mcp"),
                domain: Some("mcp-research.optima.shop"),
            },
            ServiceDef {
                name: "shopify-mcp",
                display_name: "Shopify MCP",
                category: ServiceCategory::McpTool,
                port: Some(8210),
                container_name: Some("optima-shopify-mcp-prod"),
                github_repo: Some("Optima-Chat/shopify-mcp"),
                domain: Some("mcp-shopify.optima.shop"),
            },
            ServiceDef {
                name: "commerce-mcp",
                display_name: "Commerce MCP",
                category: ServiceCategory::McpTool,
                port: Some(8270),
                container_name: Some("optima-commerce-mcp-prod"),
                github_repo: Some("Optima-Chat/commerce-mcp"),
                domain: Some("mcp-commerce.optima.shop"),
            },
            ServiceDef {
                name: "ads-mcp",
                display_name: "Ads MCP",
                category: ServiceCategory::McpTool,
                port: Some(8240),
                container_name: Some("optima-google-ads-mcp-prod"),
                github_repo: Some("Optima-Chat/google-ads-mcp"),
                domain: Some("mcp-ads.optima.shop"),
            },
            ServiceDef {
                name: "chart-mcp",
                display_name: "Chart MCP",
                category: ServiceCategory::McpTool,
                port: Some(8230),
                container_name: Some("optima-chart-mcp-prod"),
                github_repo: Some("Optima-Chat/chart-mcp"),
                domain: Some("mcp-chart.optima.shop"),
            },
        ],
    }
}

fn get_ecs_stage_config() -> EnvironmentConfig {
    EnvironmentConfig {
        env_type: EnvironmentType::EcsStage,
        ec2_host: None,
        cluster_name: Some("optima-stage-cluster"),
        domain_suffix: ".stage.optima.onl",
        services: vec![
            // Core Services
            ServiceDef {
                name: "user-auth",
                display_name: "User Auth",
                category: ServiceCategory::Core,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/user-auth"),
                domain: Some("auth.stage.optima.onl"),
            },
            ServiceDef {
                name: "user-auth-admin",
                display_name: "Auth Admin",
                category: ServiceCategory::Core,
                port: Some(3000),
                container_name: None,
                github_repo: Some("Optima-Chat/user-auth"),
                domain: Some("portal.admin.stage.optima.onl"),
            },
            ServiceDef {
                name: "commerce-backend",
                display_name: "Commerce",
                category: ServiceCategory::Core,
                port: Some(8200),
                container_name: None,
                github_repo: Some("Optima-Chat/commerce-backend"),
                domain: Some("api.stage.optima.onl"),
            },
            ServiceDef {
                name: "mcp-host",
                display_name: "MCP Host",
                category: ServiceCategory::Core,
                port: Some(8300),
                container_name: None,
                github_repo: Some("Optima-Chat/mcp-host"),
                domain: Some("host.mcp.stage.optima.onl"),
            },
            ServiceDef {
                name: "agentic-chat",
                display_name: "Agentic Chat",
                category: ServiceCategory::Core,
                port: Some(3000),
                container_name: None,
                github_repo: Some("Optima-Chat/agentic-chat"),
                domain: Some("ai.stage.optima.onl"),
            },
            // MCP Tools
            ServiceDef {
                name: "comfy-mcp",
                display_name: "Comfy MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/comfy-mcp"),
                domain: Some("comfy.mcp.stage.optima.onl"),
            },
            ServiceDef {
                name: "fetch-mcp",
                display_name: "Fetch MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/fetch-mcp"),
                domain: Some("fetch.mcp.stage.optima.onl"),
            },
            ServiceDef {
                name: "research-mcp",
                display_name: "Research MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/perplexity-mcp"),
                domain: Some("research.mcp.stage.optima.onl"),
            },
            ServiceDef {
                name: "shopify-mcp",
                display_name: "Shopify MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/shopify-mcp"),
                domain: Some("shopify.mcp.stage.optima.onl"),
            },
            ServiceDef {
                name: "chart-mcp",
                display_name: "Chart MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/chart-mcp"),
                domain: Some("chart.mcp.stage.optima.onl"),
            },
            ServiceDef {
                name: "commerce-mcp",
                display_name: "Commerce MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/commerce-mcp"),
                domain: Some("commerce.mcp.stage.optima.onl"),
            },
            ServiceDef {
                name: "ads-mcp",
                display_name: "Ads MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/google-ads-mcp"),
                domain: Some("ads.mcp.stage.optima.onl"),
            },
            // BI Services
            ServiceDef {
                name: "bi-backend",
                display_name: "BI Backend",
                category: ServiceCategory::BiService,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/optima-bi"),
                domain: Some("bi.stage.optima.onl"),
            },
            ServiceDef {
                name: "bi-dashboard",
                display_name: "BI Dashboard",
                category: ServiceCategory::BiService,
                port: Some(3000),
                container_name: None,
                github_repo: Some("Optima-Chat/optima-bi"),
                domain: Some("dashboard.bi.stage.optima.onl"),
            },
            ServiceDef {
                name: "bi-mcp",
                display_name: "BI MCP",
                category: ServiceCategory::BiService,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/optima-bi"),
                domain: Some("mcp.bi.stage.optima.onl"),
            },
            // Migration Tasks
            ServiceDef {
                name: "user-auth-migration",
                display_name: "User Auth Migration",
                category: ServiceCategory::Migration,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/user-auth"),
                domain: None,
            },
            ServiceDef {
                name: "mcp-host-migration",
                display_name: "MCP Host Migration",
                category: ServiceCategory::Migration,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/mcp-host"),
                domain: None,
            },
            ServiceDef {
                name: "agentic-chat-migration",
                display_name: "Agentic Chat Migration",
                category: ServiceCategory::Migration,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/agentic-chat"),
                domain: None,
            },
            ServiceDef {
                name: "commerce-backend-migration",
                display_name: "Commerce Migration",
                category: ServiceCategory::Migration,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/commerce-backend"),
                domain: None,
            },
            ServiceDef {
                name: "ads-mcp-migration",
                display_name: "Ads MCP Migration",
                category: ServiceCategory::Migration,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/google-ads-mcp"),
                domain: None,
            },
            // Scheduled Tasks
            ServiceDef {
                name: "ads-billing-checker",
                display_name: "Ads Billing Checker",
                category: ServiceCategory::Scheduled,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/google-ads-mcp"),
                domain: None,
            },
        ],
    }
}

fn get_ecs_prod_config() -> EnvironmentConfig {
    EnvironmentConfig {
        env_type: EnvironmentType::EcsProd,
        ec2_host: None,
        cluster_name: Some("optima-prod-cluster"),
        domain_suffix: ".optima.onl",
        services: vec![
            // Core Services
            ServiceDef {
                name: "user-auth",
                display_name: "User Auth",
                category: ServiceCategory::Core,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/user-auth"),
                domain: Some("auth.optima.onl"),
            },
            ServiceDef {
                name: "user-auth-admin",
                display_name: "Auth Admin",
                category: ServiceCategory::Core,
                port: Some(3000),
                container_name: None,
                github_repo: Some("Optima-Chat/user-auth"),
                domain: Some("portal.admin.optima.onl"),
            },
            ServiceDef {
                name: "commerce-backend",
                display_name: "Commerce",
                category: ServiceCategory::Core,
                port: Some(8200),
                container_name: None,
                github_repo: Some("Optima-Chat/commerce-backend"),
                domain: Some("api.optima.onl"),
            },
            ServiceDef {
                name: "mcp-host",
                display_name: "MCP Host",
                category: ServiceCategory::Core,
                port: Some(8300),
                container_name: None,
                github_repo: Some("Optima-Chat/mcp-host"),
                domain: Some("host.mcp.optima.onl"),
            },
            ServiceDef {
                name: "agentic-chat",
                display_name: "Agentic Chat",
                category: ServiceCategory::Core,
                port: Some(3000),
                container_name: None,
                github_repo: Some("Optima-Chat/agentic-chat"),
                domain: Some("ai.optima.onl"),
            },
            // MCP Tools (same as stage but different domain)
            ServiceDef {
                name: "comfy-mcp",
                display_name: "Comfy MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/comfy-mcp"),
                domain: Some("comfy.mcp.optima.onl"),
            },
            ServiceDef {
                name: "fetch-mcp",
                display_name: "Fetch MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/fetch-mcp"),
                domain: Some("fetch.mcp.optima.onl"),
            },
            ServiceDef {
                name: "research-mcp",
                display_name: "Research MCP",
                category: ServiceCategory::McpTool,
                port: Some(8000),
                container_name: None,
                github_repo: Some("Optima-Chat/perplexity-mcp"),
                domain: Some("research.mcp.optima.onl"),
            },
            // Migration Tasks
            ServiceDef {
                name: "user-auth-migration",
                display_name: "User Auth Migration",
                category: ServiceCategory::Migration,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/user-auth"),
                domain: None,
            },
            ServiceDef {
                name: "mcp-host-migration",
                display_name: "MCP Host Migration",
                category: ServiceCategory::Migration,
                port: None,
                container_name: None,
                github_repo: Some("Optima-Chat/mcp-host"),
                domain: None,
            },
        ],
    }
}

fn get_shared_config() -> EnvironmentConfig {
    EnvironmentConfig {
        env_type: EnvironmentType::Shared,
        ec2_host: Some("shared.optima.onl"),
        cluster_name: None,
        domain_suffix: ".optima.onl",
        services: vec![
            ServiceDef {
                name: "infisical",
                display_name: "Infisical",
                category: ServiceCategory::Infrastructure,
                port: Some(5080),
                container_name: Some("infisical"),
                github_repo: None,
                domain: Some("secrets.optima.shop"),
            },
            ServiceDef {
                name: "buildkit",
                display_name: "BuildKit",
                category: ServiceCategory::Infrastructure,
                port: None,
                container_name: Some("buildkitd"),
                github_repo: None,
                domain: None,
            },
            ServiceDef {
                name: "dev-machine",
                display_name: "Dev Machine",
                category: ServiceCategory::Infrastructure,
                port: None,
                container_name: None,
                github_repo: None,
                domain: Some("dev.optima.onl"),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_type_from_str() {
        assert_eq!(EnvironmentType::from_str("ec2-prod"), Some(EnvironmentType::Ec2Prod));
        assert_eq!(EnvironmentType::from_str("ecs-stage"), Some(EnvironmentType::EcsStage));
        assert_eq!(EnvironmentType::from_str("invalid"), None);
    }

    #[test]
    fn test_get_all_environments() {
        let envs = get_all_environments();
        assert_eq!(envs.len(), 4);
    }

    #[test]
    fn test_ec2_prod_has_services() {
        let config = get_ec2_prod_config();
        assert!(!config.services.is_empty());
        assert!(config.ec2_host.is_some());
        assert!(config.cluster_name.is_none());
    }

    #[test]
    fn test_ecs_stage_has_cluster() {
        let config = get_ecs_stage_config();
        assert!(config.cluster_name.is_some());
        assert!(config.ec2_host.is_none());
    }
}
