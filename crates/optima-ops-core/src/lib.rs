//! optima-ops-core - Core shared library for Optima Ops CLI and Web Dashboard
//!
//! This crate provides shared functionality for:
//! - Configuration management
//! - Error handling
//! - SSH client for EC2 connections
//! - AWS SDK integration
//! - Health checking
//! - Infrastructure monitoring
//! - GitHub Actions integration

pub mod config;
pub mod error;
pub mod github;
pub mod infra;
pub mod ssh;
pub mod utils;

// Re-exports for convenience
pub use config::{AppConfig, Environment, ServiceConfig, ServiceType};
pub use error::{handle_error, OpsCLIError, Result};
pub use github::{
    default_deployment_services, get_status_class, get_status_text,
    DeploymentService, DeploymentStatus, GitHubClient, WorkflowRun,
};
pub use infra::{InfraClient, InfrastructureStatus, Ec2Status, EcsServiceStatus, EcsClusterStatus, RdsStatus, AlbStatus};
pub use ssh::{parse_container_status, validate_command, ContainerStatus, SSHClient, SSHCommandResult};
pub use utils::expand_tilde;
