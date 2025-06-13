<div align="center">

# 🍄 Lium CLI

**Command-line interface for lium GPU compute management**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/lium-cli.svg)](https://crates.io/crates/lium-cli)
[![Build Status](https://github.com/distributedstatemachine/lium-rs/workflows/CI/badge.svg)](https://github.com/distributedstatemachine/lium-rs/actions)

*Command-line interface for cloud GPU computing with Lium*

Rent high-performance cloud GPUs, manage containerized workloads, and scale your ML/AI projects with ease. Access RTX 4090s, H100s, A100s, and other powerful GPUs on-demand.

</div>

## 🚀 Quick Install

### One-Line Installation

```bash
curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh
```

### Alternative Methods

<details>
<summary>📦 Package Managers (Coming Soon)</summary>

**Cargo (Rust):**
```bash
cargo install lium-cli
```

**AUR (Arch Linux):**
```bash
yay -S lium-cli
```

</details>

<details>
<summary>🔧 Manual Installation</summary>

1. Download the latest binary from [releases](https://github.com/distributedstatemachine/lium-rs/releases)
2. Make it executable: `chmod +x lium-cli-*`
3. Move to PATH: `sudo mv lium-cli-* /usr/local/bin/lium`

**Supported Platform:**
- Linux (x86_64)

</details>

<details>
<summary>🦀 From Source</summary>

```bash
git clone https://github.com/distributedstatemachine/lium-rs.git
cd lium-rs
cargo install --path crates/lium-cli
```

</details>

### Uninstall

```bash
curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/uninstall.sh | sh
```

## 🎯 Quick Start

```bash
# Initialize configuration
lium init

# List available GPU executors
lium ls

# Create a new pod
lium up --image pytorch/pytorch:latest

# SSH into your pod
lium ssh my-pod

# List your active pods
lium ps

# Terminate a pod
lium down my-pod
```

## 📋 Commands Overview

| Command | Description | Example |
|---------|-------------|---------|
| `init` | Set up configuration and API keys | `lium init` |
| `ls` | List available GPU executors | `lium ls --gpu-type "RTX 4090"` |
| `up` | Create and start a new pod | `lium up --image pytorch/pytorch:latest` |
| `down` | Stop and terminate a pod | `lium down my-pod` |
| `ps` | List your active pods | `lium ps` |
| `ssh` | Connect to a pod via SSH | `lium ssh my-pod` |
| `exec` | Execute commands in a pod | `lium exec my-pod "nvidia-smi"` |
| `rsync` | Sync files with a pod | `lium rsync ./data/ my-pod:~/data/` |
| `scp` | Copy files to/from a pod | `lium scp file.txt my-pod:~/` |
| `image` | Manage Docker images | `lium image ls` |
| `config` | Manage configuration | `lium config show` |
| `theme` | Change CLI appearance | `lium theme set dark` |
| `fund` | Manage wallet funding | `lium fund balance` |

## 🔧 Usage Guide

### Finding GPU Executors

```bash
# List all available executors
lium ls

# Filter by GPU type
lium ls --gpu-type "RTX 4090"

# Filter by price range
lium ls --max-price 1.5

# Filter by location
lium ls --region us-east

# Sort by price
lium ls --sort price

# Show detailed view
lium ls --format detailed

# Export to JSON
lium ls --format json > executors.json
```

### Pod Management

```bash
# Basic pod creation
lium up --image pytorch/pytorch:latest

# Specify executor
lium up --executor abc123 --image tensorflow/tensorflow:latest

# Use a template
lium up --template ml-training

# Set resource requirements
lium up --image ubuntu:22.04 --gpu-count 2 --ram 32

# Set environment variables
lium up --image python:3.9 --env "PYTHONPATH=/app" --env "DEBUG=1"

# Forward ports
lium up --image jupyter/tensorflow-notebook --port 8888:8888

# Custom startup script
lium up --image ubuntu:22.04 --script setup.sh
```

### File Operations

```bash
# Copy file to pod
lium scp ./model.py my-pod:~/

# Copy directory from pod
lium scp my-pod:~/results/ ./local-results/

# Sync directories
lium rsync ./code/ my-pod:~/code/ --delete

# Sync with specific options
lium rsync ./data/ my-pod:~/data/ --exclude "*.tmp" --compress
```

### Remote Execution

```bash
# Run a command
lium exec my-pod "nvidia-smi"

# Run with environment variables
lium exec my-pod --env "CUDA_VISIBLE_DEVICES=0" "python train.py"

# Execute a script
lium exec my-pod --script train.sh

# Interactive shell
lium ssh my-pod
```

## ⚙️ Configuration

### Initial Setup

```bash
# Interactive setup wizard
lium init

# Set API key manually
lium config set api_key "your-api-key-here"

# Set default SSH key
lium config set ssh_key_path "~/.ssh/id_rsa"
```

### Configuration File

Configuration is stored in `~/.lium/config.toml`:

```toml
[api]
api_key = "your-api-key-here"
base_url = "https://api.lium.ai"

[ssh]
key_path = "~/.ssh/id_rsa"
user = "root"

[defaults]
image = "pytorch/pytorch:latest"
gpu_count = 1
```

### Configuration Commands

```bash
# View current configuration
lium config show

# Set individual values
lium config set default_image "pytorch/pytorch:latest"
lium config set default_gpu_count 1
lium config set auto_confirm false

# Get specific value
lium config get api_key
```

## 🌐 Connectivity

### SSH Configuration

The CLI automatically manages SSH connectivity:

- **Key Management**: Uses your default SSH key or configured key
- **Auto-Connect**: Establishes secure tunnels automatically
- **Port Forwarding**: Forwards specified ports to your local machine

### Troubleshooting

```bash
# Test connectivity
lium ssh my-pod --test

# Debug connection
lium ssh my-pod --debug

# Use specific SSH key
lium ssh my-pod --ssh-key ~/.ssh/custom_key

# Connect with port forwarding
lium ssh my-pod --port 8888:8888
```

## 💰 Billing

```bash
# Check wallet balance
lium fund balance

# View billing history
lium fund history

# Add funds
lium fund add

# Set spending limits
lium config set max_hourly_spend 10.0
```

## 🎨 Customization

### Themes

```bash
# List available themes
lium theme list

# Set theme
lium theme set dark

# Create custom theme
lium theme create my-theme
```

### Output Formats

```bash
# Table format (default)
lium ls --format table

# Compact format
lium ls --format compact

# Detailed format
lium ls --format detailed

# JSON output
lium ls --format json

# Summary format
lium ls --format summary
```

## 🔍 Advanced Features

### Templates

```bash
# List available templates
lium image templates

# Use a template
lium up --template pytorch-jupyter

# Create custom template
lium image create-template my-template \
  --image pytorch/pytorch:latest \
  --gpu-count 1 \
  --startup-script setup.py
```

### Automation

```bash
# Non-interactive mode
lium up --image ubuntu:22.04 --yes

# Configuration file
lium up --config pod-config.yaml

# Environment file
lium up --env-file .env --image my-app:latest
```

## 🐛 Troubleshooting

### Common Issues

**Pod won't start:**
```bash
# Check pod status
lium ps my-pod --details

# View logs
lium exec my-pod "journalctl -u docker"
```

**SSH connection fails:**
```bash
# Test SSH connectivity
lium ssh my-pod --test

# Check pod status
lium ps my-pod

# Verify SSH key
lium config get ssh_key_path
```

**Command not found:**
```bash
# Check if lium is in PATH
which lium

# Reinstall if needed
curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh
```

### Debug Mode

```bash
# Enable verbose logging
export LIUM_LOG=debug
lium ls

# Or use the debug flag
lium --debug ls
```

### Getting Help

```bash
# General help
lium --help

# Command-specific help
lium up --help

# Version information
lium --version
```

## 🔗 Integration

### CI/CD Pipelines

```yaml
# GitHub Actions example
- name: Setup Lium CLI
  run: |
    curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh
    lium config set api_key ${{ secrets.LIUM_API_KEY }}

- name: Train Model
  run: |
    lium up --image my-training:latest --script train.sh --wait
    lium scp my-pod:~/model.pkl ./artifacts/
    lium down my-pod
```

### Docker Integration

```dockerfile
# Use in Dockerfile
FROM ubuntu:22.04
RUN curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh
```

## 📊 Monitoring

### Resource Usage

```bash
# Monitor pod resources
lium exec my-pod "htop"

# GPU utilization
lium exec my-pod "nvidia-smi -l 1"

# Disk usage
lium exec my-pod "df -h"
```

### Cost Tracking

```bash
# Current costs
lium fund current-costs

# Set spending alerts
lium config set spending_alert_threshold 50.0

# Cost projection
lium fund estimate --hours 24
```

## 🚀 Performance Tips

1. **Image Optimization**: Use smaller, optimized Docker images
2. **Region Selection**: Choose executors close to your location
3. **Resource Matching**: Match executor specs to your workload
4. **Batch Operations**: Use templates for repeated deployments
5. **Data Locality**: Keep data close to compute resources

## 📱 Platform Support

- ✅ **Linux** (x86_64)

## 🆘 Support

- 📖 **Documentation**: [GitHub Repository](https://github.com/distributedstatemachine/lium-rs)
- 🐛 **Bug Reports**: [GitHub Issues](https://github.com/distributedstatemachine/lium-rs/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/distributedstatemachine/lium-rs/discussions)
- 🔐 **Security**: [Security Policy](https://github.com/distributedstatemachine/lium-rs/security/policy)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/distributedstatemachine/lium-rs/blob/main/LICENSE) file for details.

---

<div align="center">

**Built with ❤️ and 🦀 Rust**

*🍄 Lium CLI - Making GPU compute simple and powerful*

</div> 