//! Environment command - show current environment information

use anyhow::Result;
use clap::Args;
use colored::*;
use optima_ops_core::AppConfig;

#[derive(Args)]
pub struct EnvCommand;

impl EnvCommand {
    pub async fn execute(&self, config: &AppConfig) -> Result<()> {
        let env = config.get_environment();
        let env_info = env.get_env_info();
        let ec2_config = config.get_ec2_config(None);
        let aws_config = config.get_aws_config();

        println!("{}", "当前环境配置".bold());
        println!();
        println!("  {} {}", "环境:".cyan(), env.as_str().green().bold());
        println!("  {} {}", "EC2 主机:".cyan(), ec2_config.host);
        println!("  {} {}", "EC2 用户:".cyan(), ec2_config.user);
        println!("  {} {}", "SSH 密钥:".cyan(), ec2_config.key_path);
        println!("  {} {}", "AWS 区域:".cyan(), aws_config.region);
        println!();
        println!("{}", "环境详情".bold());
        println!();
        println!("  {} {}", "RDS 主机:".cyan(), env_info.rds_host);
        println!("  {} {}", "Docker 网络:".cyan(), env_info.docker_network);

        Ok(())
    }
}
