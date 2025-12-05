//! Error types and error handling for Optima Ops

use thiserror::Error;

/// Result type alias using OpsCLIError
pub type Result<T> = std::result::Result<T, OpsCLIError>;

/// Custom error types for Optima Ops operations
#[derive(Error, Debug)]
pub enum OpsCLIError {
    #[error("SSH connection error: {0}")]
    SSHConnection(String),

    #[error("Command execution error: {0}")]
    CommandExecution(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("AWS error: {0}")]
    AWS(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HTTP(#[from] reqwest::Error),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

/// Handle and display errors with helpful messages
pub fn handle_error(error: &OpsCLIError) {
    eprintln!("✗ 错误: {}", error);

    // If DEBUG environment variable is set, show detailed info
    if std::env::var("DEBUG").is_ok() {
        if let Some(source) = std::error::Error::source(error) {
            eprintln!("\n详细信息:");
            eprintln!("{:?}", source);
        }
    }

    // Provide helpful tips
    match error {
        OpsCLIError::SSHConnection(_) => {
            eprintln!("\n提示:");
            eprintln!("  • 检查 SSH 密钥是否存在: ls -la ~/.ssh/optima-ec2-key");
            eprintln!("  • 检查网络连接");
            eprintln!("  • 验证 EC2 主机地址");
        }
        OpsCLIError::Configuration(_) => {
            eprintln!("\n提示:");
            eprintln!("  • 检查配置文件: ~/.config/optima-ops-cli/config.json");
            eprintln!("  • 验证环境变量设置");
        }
        _ => {}
    }
}
