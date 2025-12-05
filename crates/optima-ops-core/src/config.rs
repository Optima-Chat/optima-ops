//! Configuration management for Optima Ops
//!
//! Handles loading and parsing of configuration files for:
//! - Environment settings (prod, stage, shared, dev)
//! - EC2 connection details
//! - AWS region and profile
//! - Service definitions

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::utils::expand_tilde;

/// Environment type for deployment targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Production,
    Stage,
    Shared,
    Development,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Production => "production",
            Environment::Stage => "stage",
            Environment::Shared => "shared",
            Environment::Development => "development",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Some(Environment::Production),
            "stage" | "staging" => Some(Environment::Stage),
            "shared" => Some(Environment::Shared),
            "development" | "dev" => Some(Environment::Development),
            _ => None,
        }
    }

    pub fn get_env_info(&self) -> EnvInfo {
        match self {
            Environment::Production => EnvInfo {
                ec2_host: "ec2-prod.optima.shop",
                rds_host: "optima-prod-postgres.ctg866o0ehac.ap-southeast-1.rds.amazonaws.com",
                docker_network: "optima-prod",
            },
            Environment::Stage => EnvInfo {
                ec2_host: "ec2-stage.optima.shop",
                rds_host: "optima-stage-postgres.ctg866o0ehac.ap-southeast-1.rds.amazonaws.com",
                docker_network: "optima-stage",
            },
            Environment::Shared => EnvInfo {
                ec2_host: "shared.optima.onl",
                rds_host: "",
                docker_network: "optima-shared",
            },
            Environment::Development => EnvInfo {
                ec2_host: "ec2-dev.optima.shop",
                rds_host: "optima-dev-postgres.ctg866o0ehac.ap-southeast-1.rds.amazonaws.com",
                docker_network: "optima-dev",
            },
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Environment-specific information
pub struct EnvInfo {
    pub ec2_host: &'static str,
    pub rds_host: &'static str,
    pub docker_network: &'static str,
}

/// EC2 connection configuration
#[derive(Debug, Clone, Deserialize)]
pub struct EC2Config {
    pub host: String,
    pub user: String,
    #[serde(rename = "keyPath")]
    pub key_path: String,
}

/// AWS configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AWSConfig {
    pub region: String,
    pub profile: Option<String>,
}

/// Main configuration file structure
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub environment: Environment,
    pub ec2: EC2ConfigMap,
    pub aws: AWSConfig,
}

/// EC2 configurations for all environments
#[derive(Debug, Clone, Deserialize)]
pub struct EC2ConfigMap {
    pub production: EC2Config,
    pub stage: EC2Config,
    pub shared: EC2Config,
    pub development: EC2Config,
}

/// Service type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Core,
    MCP,
}

/// Individual service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub repo: String,
    pub container: String,
    #[serde(rename = "healthEndpoint")]
    pub health_endpoint: String,
    #[serde(rename = "type")]
    pub service_type: ServiceType,
    pub port: Option<u16>,
    #[serde(rename = "hasDatabase")]
    pub has_database: bool,
    #[serde(rename = "hasRedis")]
    pub has_redis: bool,
}

/// Services configuration file structure
#[derive(Debug, Clone, Deserialize)]
pub struct ServicesConfigFile {
    pub services: ServicesMap,
}

/// Services grouped by type
#[derive(Debug, Clone, Deserialize)]
pub struct ServicesMap {
    pub core: Vec<ServiceConfig>,
    pub mcp: Vec<ServiceConfig>,
}

/// Application configuration manager
#[derive(Clone)]
pub struct AppConfig {
    config: ConfigFile,
    services: ServicesConfigFile,
    current_env: Environment,
}

impl AppConfig {
    /// Load configuration from files
    pub fn load() -> Result<Self> {
        // Load main config
        let config_path = Self::get_config_path()?;
        let config: ConfigFile = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            serde_json::from_str(&content)
                .context("Failed to parse config file")?
        } else {
            Self::default_config()
        };

        // Load services config
        let services_path = Self::get_services_config_path()?;
        let services: ServicesConfigFile = if services_path.exists() {
            let content = fs::read_to_string(&services_path)
                .context("Failed to read services config file")?;
            serde_json::from_str(&content)
                .context("Failed to parse services config file")?
        } else {
            Self::default_services_config()
        };

        // Determine current environment
        let current_env = std::env::var("OPTIMA_OPS_ENV")
            .ok()
            .and_then(|s| Environment::from_str(&s))
            .unwrap_or(config.environment);

        Ok(Self {
            config,
            services,
            current_env,
        })
    }

    pub fn get_environment(&self) -> Environment {
        self.current_env
    }

    pub fn get_ec2_config(&self, env: Option<Environment>) -> &EC2Config {
        let env = env.unwrap_or(self.current_env);
        match env {
            Environment::Production => &self.config.ec2.production,
            Environment::Stage => &self.config.ec2.stage,
            Environment::Shared => &self.config.ec2.shared,
            Environment::Development => &self.config.ec2.development,
        }
    }

    pub fn get_aws_config(&self) -> &AWSConfig {
        &self.config.aws
    }

    pub fn get_all_services(&self) -> Vec<&ServiceConfig> {
        self.services.services.core.iter()
            .chain(self.services.services.mcp.iter())
            .collect()
    }

    pub fn get_services_by_type(&self, service_type: ServiceType) -> Vec<&ServiceConfig> {
        match service_type {
            ServiceType::Core => self.services.services.core.iter().collect(),
            ServiceType::MCP => self.services.services.mcp.iter().collect(),
        }
    }

    pub fn get_service(&self, name: &str) -> Option<&ServiceConfig> {
        self.get_all_services().into_iter().find(|s| s.name == name)
    }

    pub fn get_ssh_key_path(&self, env: Option<Environment>) -> PathBuf {
        // Prefer environment variable
        if let Ok(key_path) = std::env::var("OPTIMA_SSH_KEY") {
            return PathBuf::from(expand_tilde(&key_path));
        }

        // Expand ~ to home directory
        let key_path = &self.get_ec2_config(env).key_path;
        PathBuf::from(expand_tilde(key_path))
    }

    pub fn get_ssh_private_key(&self, env: Option<Environment>) -> Result<String> {
        let key_path = self.get_ssh_key_path(env);
        fs::read_to_string(&key_path)
            .with_context(|| format!("Failed to read SSH key from {}", key_path.display()))
    }

    fn get_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Failed to get home directory")?;
        Ok(home.join(".config/optima-ops-cli/config.json"))
    }

    fn get_services_config_path() -> Result<PathBuf> {
        // Look in project root directory
        let current_exe = std::env::current_exe()?;
        let exe_dir = current_exe.parent()
            .context("Failed to get executable directory")?;

        // Dev mode: ../services-config.json
        let dev_path = exe_dir.join("../services-config.json");
        if dev_path.exists() {
            return Ok(dev_path);
        }

        // Release mode: same directory as executable
        Ok(exe_dir.join("services-config.json"))
    }

    fn default_config() -> ConfigFile {
        ConfigFile {
            environment: Environment::Production,
            ec2: EC2ConfigMap {
                production: EC2Config {
                    host: "ec2-prod.optima.shop".to_string(),
                    user: "ec2-user".to_string(),
                    key_path: "~/.ssh/optima-ec2-key".to_string(),
                },
                stage: EC2Config {
                    host: "ec2-stage.optima.shop".to_string(),
                    user: "ec2-user".to_string(),
                    key_path: "~/.ssh/optima-ec2-key".to_string(),
                },
                shared: EC2Config {
                    host: "shared.optima.onl".to_string(),
                    user: "ec2-user".to_string(),
                    key_path: "~/.ssh/optima-ec2-key".to_string(),
                },
                development: EC2Config {
                    host: "ec2-dev.optima.shop".to_string(),
                    user: "ec2-user".to_string(),
                    key_path: "~/.ssh/optima-ec2-key".to_string(),
                },
            },
            aws: AWSConfig {
                region: "ap-southeast-1".to_string(),
                profile: None,
            },
        }
    }

    fn default_services_config() -> ServicesConfigFile {
        ServicesConfigFile {
            services: ServicesMap {
                core: vec![
                    ServiceConfig {
                        name: "user-auth".to_string(),
                        repo: "Optima-Chat/user-auth".to_string(),
                        container: "optima-user-auth-prod".to_string(),
                        health_endpoint: "https://auth.optima.shop/health".to_string(),
                        service_type: ServiceType::Core,
                        port: Some(8100),
                        has_database: true,
                        has_redis: true,
                    },
                    ServiceConfig {
                        name: "mcp-host".to_string(),
                        repo: "Optima-Chat/mcp-host".to_string(),
                        container: "optima-mcp-host-prod".to_string(),
                        health_endpoint: "https://mcp.optima.shop/health".to_string(),
                        service_type: ServiceType::Core,
                        port: Some(8300),
                        has_database: true,
                        has_redis: false,
                    },
                    ServiceConfig {
                        name: "commerce-backend".to_string(),
                        repo: "Optima-Chat/commerce-backend".to_string(),
                        container: "optima-commerce-backend-prod".to_string(),
                        health_endpoint: "https://api.optima.shop/health".to_string(),
                        service_type: ServiceType::Core,
                        port: Some(8200),
                        has_database: true,
                        has_redis: true,
                    },
                    ServiceConfig {
                        name: "agentic-chat".to_string(),
                        repo: "Optima-Chat/agentic-chat".to_string(),
                        container: "optima-agentic-chat-prod".to_string(),
                        health_endpoint: "https://ai.optima.shop/health".to_string(),
                        service_type: ServiceType::Core,
                        port: Some(8250),
                        has_database: true,
                        has_redis: false,
                    },
                ],
                mcp: vec![
                    ServiceConfig {
                        name: "comfy-mcp".to_string(),
                        repo: "Optima-Chat/comfy-mcp".to_string(),
                        container: "optima-comfy-mcp-prod".to_string(),
                        health_endpoint: "https://mcp-comfy.optima.shop".to_string(),
                        service_type: ServiceType::MCP,
                        port: Some(8261),
                        has_database: false,
                        has_redis: false,
                    },
                ],
            },
        }
    }
}
