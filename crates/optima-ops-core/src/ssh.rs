//! SSH client for EC2 connections
//!
//! Provides secure SSH connectivity with command validation and whitelisting.

use crate::config::{AppConfig, Environment};
use crate::error::{OpsCLIError, Result};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::time::{Duration, Instant};

// ============== SSH Command Whitelist ==============

const READONLY_COMMANDS: &[&str] = &[
    "docker ps",
    "docker logs",
    "docker inspect",
    "docker stats",
    "docker network",
    "docker images",
    "docker exec",
    "ip ",
    "ip-",
    "df -h",
    "df -BG",
    "free -h",
    "free -m",
    "systemctl status",
    "systemctl show",
    "systemctl list-units",
    "journalctl",
    "cat",
    "grep",
    "ls",
    "find",
    "tail",
    "head",
    "echo",
    "pwd",
    "whoami",
    "uptime",
    "date",
    "wc",
    "curl",
    "ec2-metadata",
    "cut",
];

const LOWRISK_COMMANDS: &[&str] = &[
    "docker-compose restart",
    "docker restart",
    "systemctl restart",
];

const DANGEROUS_COMMANDS: &[&str] = &[
    "rm ",
    "docker rm",
    "docker system prune",
    "docker volume rm",
    "kill ",
    "shutdown",
    "reboot",
    "poweroff",
    " > ",
    " >> ",
    ";",
    "&&",
    "||",
];

/// Result of command validation
pub struct CommandValidation {
    pub safe: bool,
    pub reason: Option<String>,
}

/// Validate a command against the whitelist
pub fn validate_command(command: &str) -> CommandValidation {
    let cmd_lower = command.trim().to_lowercase();

    // Check dangerous commands
    for dangerous in DANGEROUS_COMMANDS {
        if cmd_lower.contains(dangerous) {
            return CommandValidation {
                safe: false,
                reason: Some(format!("命令包含危险操作: {}", dangerous)),
            };
        }
    }

    // Check pipe (allow inside quotes)
    let outside_quotes = command
        .replace(r#""[^"]*""#, "")
        .replace(r"'[^']*'", "");
    if outside_quotes.contains('|') {
        return CommandValidation {
            safe: false,
            reason: Some("命令包含危险操作: |".to_string()),
        };
    }

    // Check readonly commands
    for readonly in READONLY_COMMANDS {
        if cmd_lower.starts_with(readonly) {
            return CommandValidation {
                safe: true,
                reason: None,
            };
        }
    }

    // Check low-risk commands
    for lowrisk in LOWRISK_COMMANDS {
        if cmd_lower.starts_with(lowrisk) {
            return CommandValidation {
                safe: true,
                reason: None,
            };
        }
    }

    CommandValidation {
        safe: false,
        reason: Some("命令未在白名单中".to_string()),
    }
}

// ============== SSH Client ==============

/// Result of SSH command execution
pub struct SSHCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub command: String,
    pub execution_time: Duration,
}

/// SSH client for connecting to EC2 instances
pub struct SSHClient {
    session: Option<Session>,
    env: Environment,
    config: AppConfig,
}

impl SSHClient {
    pub fn new(config: &AppConfig, env: Option<Environment>) -> Self {
        Self {
            session: None,
            env: env.unwrap_or_else(|| config.get_environment()),
            config: config.clone(),
        }
    }

    /// Connect to the EC2 instance
    pub async fn connect(&mut self) -> Result<()> {
        if self.session.is_some() {
            return Ok(());
        }

        let ec2_config = self.config.get_ec2_config(Some(self.env));
        let private_key = self.config.get_ssh_private_key(Some(self.env))?;

        // Establish TCP connection
        let tcp = TcpStream::connect(format!("{}:22", ec2_config.host))
            .map_err(|e| OpsCLIError::SSHConnection(format!("无法连接到 {}: {}", ec2_config.host, e)))?;

        tcp.set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| OpsCLIError::SSHConnection(format!("设置超时失败: {}", e)))?;

        // Create SSH session
        let mut sess = Session::new()
            .map_err(|e| OpsCLIError::SSHConnection(format!("创建 SSH session 失败: {}", e)))?;

        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| OpsCLIError::SSHConnection(format!("SSH 握手失败: {}", e)))?;

        // Authenticate
        sess.userauth_pubkey_memory(&ec2_config.user, None, &private_key, None)
            .map_err(|e| OpsCLIError::SSHConnection(format!("SSH 认证失败: {}", e)))?;

        if !sess.authenticated() {
            return Err(OpsCLIError::SSHConnection("SSH 认证失败".to_string()));
        }

        self.session = Some(sess);
        Ok(())
    }

    /// Disconnect from the EC2 instance
    pub fn disconnect(&mut self) {
        if let Some(sess) = &mut self.session {
            let _ = sess.disconnect(None, "Closing connection", None);
        }
        self.session = None;
    }

    /// Execute a command on the EC2 instance
    pub async fn execute_command(
        &mut self,
        command: &str,
        validate_safety: bool,
        _timeout: Option<Duration>,
    ) -> Result<SSHCommandResult> {
        let start_time = Instant::now();

        // Safety check
        if validate_safety {
            let validation = validate_command(command);
            if !validation.safe {
                return Err(OpsCLIError::CommandExecution(format!(
                    "命令被安全策略阻止: {}",
                    validation.reason.unwrap_or_default()
                )));
            }
        }

        // Ensure connected
        if self.session.is_none() {
            self.connect().await?;
        }

        let session = self.session.as_ref().unwrap();

        // Execute command
        let mut channel = session
            .channel_session()
            .map_err(|e| OpsCLIError::CommandExecution(format!("创建 channel 失败: {}", e)))?;

        channel
            .exec(command)
            .map_err(|e| OpsCLIError::CommandExecution(format!("执行命令失败: {}", e)))?;

        // Read output
        let mut stdout = String::new();
        let mut stderr = String::new();

        channel
            .read_to_string(&mut stdout)
            .map_err(|e| OpsCLIError::CommandExecution(format!("读取 stdout 失败: {}", e)))?;

        channel
            .stderr()
            .read_to_string(&mut stderr)
            .map_err(|e| OpsCLIError::CommandExecution(format!("读取 stderr 失败: {}", e)))?;

        channel
            .wait_close()
            .map_err(|e| OpsCLIError::CommandExecution(format!("等待关闭失败: {}", e)))?;

        let exit_code = channel.exit_status()
            .map_err(|e| OpsCLIError::CommandExecution(format!("获取退出码失败: {}", e)))?;
        let execution_time = start_time.elapsed();

        Ok(SSHCommandResult {
            stdout,
            stderr,
            exit_code,
            command: command.to_string(),
            execution_time,
        })
    }

    /// Execute a docker command
    pub async fn docker_command(&mut self, command: &str) -> Result<SSHCommandResult> {
        self.execute_command(&format!("docker {}", command), true, None)
            .await
    }

    /// Get container status
    pub async fn get_container_status(&mut self, container_name: Option<&str>) -> Result<SSHCommandResult> {
        let filter = container_name
            .map(|n| format!(" --filter \"name={}\"", n))
            .unwrap_or_default();

        self.docker_command(&format!(
            "ps -a{} --format \"{{{{.ID}}}}\\t{{{{.Names}}}}\\t{{{{.Status}}}}\\t{{{{.Ports}}}}\"",
            filter
        ))
        .await
    }

    /// Get container logs
    pub async fn get_container_logs(
        &mut self,
        container_name: &str,
        tail: Option<u32>,
        follow: bool,
    ) -> Result<SSHCommandResult> {
        let tail_arg = tail.map(|n| format!("--tail {}", n)).unwrap_or_default();
        let follow_arg = if follow { "-f" } else { "" };

        self.docker_command(&format!("logs {} {} {}", tail_arg, follow_arg, container_name))
            .await
    }
}

impl Drop for SSHClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

// ============== Container Status Parsing ==============

/// Container status information
#[derive(Debug, Clone)]
pub struct ContainerStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub ports: String,
}

/// Parse docker ps output into ContainerStatus structs
pub fn parse_container_status(output: &str) -> Vec<ContainerStatus> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                Some(ContainerStatus {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    status: parts[2].to_string(),
                    ports: parts.get(3).unwrap_or(&"").to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_command_safe() {
        let result = validate_command("docker ps");
        assert!(result.safe);
    }

    #[test]
    fn test_validate_command_dangerous() {
        let result = validate_command("rm -rf /");
        assert!(!result.safe);
    }

    #[test]
    fn test_validate_command_not_whitelisted() {
        let result = validate_command("some-unknown-command");
        assert!(!result.safe);
    }

    #[test]
    fn test_parse_container_status() {
        let output = "abc123\tmy-container\tUp 5 hours\t80/tcp";
        let containers = parse_container_status(output);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "my-container");
    }
}
