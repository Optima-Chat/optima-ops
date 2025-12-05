//! Version command - show version information

use anyhow::Result;
use clap::Args;
use colored::*;

#[derive(Args)]
pub struct VersionCommand;

impl VersionCommand {
    pub fn execute(&self) -> Result<()> {
        println!("{} {}", "Optima Ops CLI".bold(), env!("CARGO_PKG_VERSION").green());
        println!();
        println!("  {} {}", "构建时间:".cyan(), env!("CARGO_PKG_VERSION"));
        println!("  {} {}", "Rust 版本:".cyan(), "2021 Edition");
        println!("  {} {}", "目标平台:".cyan(), std::env::consts::OS);

        Ok(())
    }
}
