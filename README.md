# Optima Ops

Optima Ops 是一个 Rust 实现的运维工具集，包含 CLI 和 Web Dashboard。

## 项目结构

```
optima-ops/
├── Cargo.toml                    # Workspace 配置
├── crates/
│   ├── optima-ops-core/          # 核心共享库
│   ├── optima-ops-cli/           # CLI 工具
│   └── optima-ops-web/           # Web Dashboard
└── README.md
```

## 快速开始

### 开发构建

```bash
# 检查编译
cargo check

# 开发构建
cargo build

# 运行 CLI
cargo run -p optima-ops-cli -- --help

# 运行 Web Dashboard
cargo run -p optima-ops-web
```

### 发布构建

```bash
cargo build --release
```

## Crates

### optima-ops-core

核心共享库，提供：
- 配置管理 (config.rs)
- 错误处理 (error.rs)
- SSH 客户端 (ssh.rs)
- 工具函数 (utils.rs)

### optima-ops-cli

命令行工具，提供：
- `oor env` - 显示环境信息
- `oor services health` - 检查服务健康状态
- `oor services list` - 列出所有服务
- `oor version` - 显示版本

### optima-ops-web

Web Dashboard，提供：
- 服务健康状态面板
- 基础设施概览
- 环境切换

技术栈：
- Axum 0.7 (Web 框架)
- Askama (模板引擎)
- HTMX 2.0 (前端交互)
- Tailwind CSS (样式)

## 配置

配置文件位置：`~/.config/optima-ops-cli/config.json`

```json
{
  "environment": "production",
  "ec2": {
    "production": {
      "host": "ec2-prod.optima.shop",
      "user": "ec2-user",
      "keyPath": "~/.ssh/optima-ec2-key"
    }
  },
  "aws": {
    "region": "ap-southeast-1"
  }
}
```

## 环境变量

- `OPTIMA_OPS_ENV` - 覆盖默认环境
- `OPTIMA_SSH_KEY` - 覆盖 SSH 密钥路径
- `RUST_LOG` - 日志级别 (debug, info, warn, error)
- `LISTEN_ADDR` - Web 服务监听地址 (默认 0.0.0.0:8080)

## License

MIT
