<div align="center">

# 🍄 Lium

**High-Performance GPU Compute Management System**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Build-Passing-green.svg)]()

*Streamlined GPU cluster orchestration and job management*

</div>

## 🚀 Overview

Lium is a modern, high-performance GPU compute management system built in Rust. It provides seamless orchestration of GPU clusters, efficient job scheduling, and intuitive command-line tools for managing distributed compute workloads.

### Key Features

- 🎯 **Pod-Based Architecture**: Organize compute resources into manageable pods
- ⚡ **High-Performance Async**: Built on Tokio for maximum throughput
- 🔧 **Interactive CLI**: Rich, user-friendly command-line interface
- 🌐 **REST API**: Comprehensive HTTP API for programmatic access
- 🐳 **Container Support**: Docker integration for containerized workloads  
- 🔐 **Secure**: SSH-based remote execution with key authentication
- 📊 **Resource Monitoring**: Real-time GPU and system resource tracking

## 🏗️ Architecture

Lium is structured as a modular Rust workspace with four core crates:

```
🍄 lium-rs/
├── 🧠 lium-core     # Domain logic and data structures
├── 🌐 lium-api      # HTTP API client
├── 🔧 lium-utils    # SSH, Docker & system utilities  
└── 🎯 lium-cli      # Command-line interface

```

### Crate Overview

| Crate | Purpose | Key Features |
|-------|---------|--------------|
| **🧠 lium-core** | Domain logic & types | Pod/job models, business logic, validation |
| **🌐 lium-api** | HTTP API client | Async REST client, authentication, error handling |
| **🔧 lium-utils** | System utilities | SSH operations, Docker integration, process management |
| **🎯 lium-cli** | CLI interface | Interactive commands, rich output, flexible targeting |

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/lium-rs.git
cd lium-rs

# Build the project
cargo build --release

# Install the CLI
cargo install --path .
```

### Basic Usage

```bash
# List available pods
lium ls

# Execute a command on a specific pod
lium exec my-gpu-pod "nvidia-smi"

# Submit a training job
lium exec training-pod "python train_model.py --epochs 100"

# Execute on multiple pods
lium exec 1,3,5 "pip install torch torchvision"

# Use interactive selection
lium exec --interactive "python inference.py"
```

### Configuration

```bash
# Set your API endpoint
lium config set api-url https://your-lium-api.com

# Configure authentication
lium config set auth-token your-api-token

# View current settings
lium config show
```

## 🎯 Command Reference

### Core Commands

```bash
lium ls                    # List pods and executors
lium exec <target> <cmd>   # Execute commands on pods
lium cp <src> <dst>        # Copy files to/from pods  
lium logs <job-id>         # View job logs
lium status <job-id>       # Check job status
lium config <action>       # Manage configuration
```

### Flexible Targeting

Lium supports multiple ways to target pods:

```bash
lium exec 1 "command"              # By index
lium exec my-pod "command"         # By name  
lium exec brave-cat-1234 "command" # By HUID
lium exec all "command"            # All pods
lium exec 1,3,5 "command"          # Multiple targets
```

## 🛠️ Development

### Project Structure

```
lium-rs/
├── crates/
│   ├── lium-core/          # Core domain logic
│   ├── lium-api/           # HTTP API client
│   ├── lium-utils/         # System utilities
│   └── lium-cli/           # CLI interface
├── src/
│   └── main.rs             # Binary entry point
├── Cargo.toml              # Workspace configuration
└── README.md               # This file
```

### Building from Source

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Build with optimizations  
cargo build --release

# Run the CLI
./target/release/lium --help
```

### Development Dependencies

- **Rust 1.70+** - Modern Rust toolchain
- **Tokio** - Async runtime
- **Clap** - CLI framework
- **Reqwest** - HTTP client
- **SSH2** - SSH protocol support
- **Bollard** - Docker API client

## 📦 Crate Documentation

Each crate has detailed documentation:

- [🧠 **lium-core**](crates/lium-core/README.md) - Domain models and business logic
- [🌐 **lium-api**](crates/lium-api/README.md) - HTTP API client library  
- [🔧 **lium-utils**](crates/lium-utils/README.md) - SSH, Docker, and system utilities
- [🎯 **lium-cli**](crates/lium-cli/README.md) - Command-line interface

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run `cargo test` and `cargo fmt`
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

<!-- - **Documentation**: [docs.lium.dev](https://docs.lium.dev) -->
<!-- - **API Reference**: [api.lium.dev](https://api.lium.dev) -->
- **Issues**: [GitHub Issues](https://github.com/your-org/lium-rs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/lium-rs/discussions)

---

<div align="center">

**Built with ❤️ and 🦀 Rust**

*🍄 Lium - Making GPU compute simple and powerful*

</div> 