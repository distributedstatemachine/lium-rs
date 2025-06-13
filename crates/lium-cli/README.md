<div align="center">

# 🍄 Lium CLI

**Command-line interface for lium GPU compute management**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/lium-cli.svg)](https://crates.io/crates/lium-cli)
[![Build Status](https://github.com/distributedstatemachine/lium-rs/workflows/CI/badge.svg)](https://github.com/distributedstatemachine/lium-rs/actions)

🍄 **Lium CLI** - Command-line interface for cloud GPU computing with Lium.

Rent high-performance cloud GPUs, manage containerized workloads, and scale your ML/AI projects with ease. Access RTX 4090s, H100s, A100s, and other powerful GPUs on-demand.

## 🚀 Quick Installation

### One-Line Installer (Recommended)

```bash
curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh
```

### Alternative Installation Methods

<details>
<summary>📦 Package Managers</summary>

#### Homebrew (macOS/Linux)
```bash
brew install distributedstatemachine/tap/lium-cli
```

#### Cargo (Rust)
```bash
cargo install lium-cli
```

#### Arch Linux (AUR)
```bash
yay -S lium-cli
```

</details>

<details>
<summary>📁 Manual Installation</summary>

1. Download the latest binary from [GitHub Releases](https://github.com/distributedstatemachine/lium-rs/releases)
2. Extract and place in your PATH
3. Make executable: `chmod +x lium-cli-*`

**Supported Platforms:**
- Linux (x86_64)

</details>

## 🏃‍♂️ Quick Start

```bash
# 1. Set up your configuration
lium init

# 2. Browse available GPUs
lium ls

# 3. Filter by specific GPU types
lium ls RTX4090 --available

# 4. Start a pod with your preferred setup
lium up --gpu RTX4090 --image pytorch/pytorch:latest

# 5. Connect to your pod
lium ssh 1

# 6. Stop your pod when done
lium down 1
```

## 📋 Commands Overview

| Command | Description | Examples |
|---------|-------------|----------|
| `lium init` | Initial setup and configuration | `lium init` |
| `lium ls` | List and filter available GPUs | `lium ls RTX4090 --available --price "0.5-2.0"` |
| `lium up` | Create and start a new pod | `lium up --gpu H100 --name training-job` |
| `lium ps` | List active pods | `lium ps --all` |
| `lium ssh` | Connect to a pod via SSH | `lium ssh my-pod` |
| `lium exec` | Execute commands on pods | `lium exec 1,2,3 "nvidia-smi"` |
| `lium down` | Stop and terminate pods | `lium down --all` |
| `lium rsync` | Sync files with pods | `lium rsync ./data/ 1:/workspace/` |
| `lium fund` | Manage wallet and billing | `lium fund balance` |
| `lium config` | Manage configuration | `lium config show` |
| `lium theme` | Customize appearance | `lium theme set dark` |

## 🔧 Detailed Usage

### 🔍 Finding the Right GPU

```bash
# List all available GPUs
lium ls

# Filter by GPU type and availability
lium ls --gpu RTX4090 --available

# Show only Pareto optimal options (best price/performance)
lium ls --pareto --format table

# Filter by price range
lium ls --price "0.5-2.0" --sort price

# Show GPU type summary
lium ls --summary

# Export results to CSV for analysis
lium ls --available --export results.csv
```

### 🚀 Creating and Managing Pods

```bash
# Quick start with default template
lium up

# Specify GPU type and custom image
lium up --gpu H100 --image nvidia/pytorch:23.08-py3

# Set environment variables and port mappings
lium up --env "DEBUG=1,API_KEY=secret" --ports "8080:80,8888:8888"

# Create with custom name and skip confirmation
lium up --name training-job --yes

# Use template by ID
lium up --image template-pytorch-base --gpu RTX4090
```

### 💻 Working with Pods

```bash
# List all active pods
lium ps

# Show detailed information for specific pods
lium ps 1,3 all

# Filter pods by status or GPU type
lium ps --status running --gpu RTX4090

# Connect via SSH for interactive work
lium ssh my-pod

# Execute commands across multiple pods
lium exec 1,2,3 "pip install transformers"
lium exec all "nvidia-smi"

# Run a script on pods
lium exec 1 --script setup.sh --env WORKERS=4
```

### 📁 File Management

```bash
# Upload data to pod
lium rsync ./dataset/ 1:/workspace/data/

# Download results from pod
lium rsync 1:/workspace/outputs/ ./results/

# Sync with progress and compression
lium rsync ./models/ 1:/workspace/models/ -av --progress --compress

# Exclude certain files
lium rsync ./project/ 1:/workspace/ --exclude "*.pyc" --exclude ".git"

# Dry run to preview changes
lium rsync ./data/ 1:/workspace/ --dry-run -v
```

## ⚙️ Configuration

### 🔐 Configuration

#### Initial Setup
```bash
lium init
```

The setup wizard will guide you through:
1. **API Key**: Get yours from [Lium Dashboard](https://dashboard.lium.ai/api-keys)
2. **SSH Key**: Configure for secure pod access
3. **Preferences**: Set defaults for common operations

#### Configuration File
Configuration is stored in `~/.lium/config.toml`:

```toml
[api]
api_key = "your-api-key-here"
base_url = "https://api.lium.ai"

[ssh]
key_path = "~/.ssh/id_ed25519.pub"
user = "root"

[template]
default_id = "pytorch-base"
```

#### Environment Variables
- `LIUM_API_KEY`: Override configured API key
- `LIUM_CONFIG_DIR`: Custom configuration directory

## 💰 Billing and Costs

```bash
# Check wallet balance
lium fund balance

# View usage history
lium fund history

# Add funds (requires Bittensor wallet)
lium fund add 10.0
```

**Cost Optimization Tips:**
- Use `--pareto` flag to find price/performance optimal GPUs
- Filter by price range: `--price "0.5-1.5"`
- Stop pods when not in use: `lium down --all`
- Monitor costs with `lium ps` (shows hourly rates)

## 🎨 Customization

### Themes
```bash
# List available themes
lium theme list

# Set a theme
lium theme set dark
lium theme set cyberpunk
```

### Display Formats
```bash
# Table view (default)
lium ls --format table

# Compact view for quick scanning
lium ls --format compact

# Detailed view with full specifications
lium ls --format detailed

# Summary view grouped by GPU type
lium ls --format summary
```

## 🔒 Security

- **SSH Keys**: All pod access uses SSH key authentication
- **API Keys**: Stored securely in local configuration
- **TLS**: All API communication uses HTTPS
- **Isolation**: Each pod runs in its own secure container

## 🆘 Troubleshooting

### Common Issues

**API Connection Failed:**
```bash
# Check API key configuration
lium config show

# Test API connectivity
lium ls --limit 1
```

**SSH Connection Issues:**
```bash
# Verify SSH key configuration
lium config show

# Check SSH key permissions
chmod 600 ~/.ssh/id_ed25519
chmod 644 ~/.ssh/id_ed25519.pub
```

**Pod Creation Failed:**
```bash
# Check available executors
lium ls --available

# Verify template/image exists
lium up --image pytorch/pytorch:latest --gpu RTX4090
```

### Getting Help

```bash
# Command help
lium --help
lium up --help

# Verbose logging
RUST_LOG=debug lium ls

# Support
# Visit https://github.com/distributedstatemachine/lium-rs
# Join our Discord: https://discord.gg/lium
```

## 🔄 Uninstallation

### One-Line Uninstaller
```bash
curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/uninstall.sh | sh
```

### Manual Removal
```bash
# Remove binary (if installed manually)
rm $(which lium)

# Remove configuration (optional)
rm -rf ~/.lium

# Remove from PATH (if added manually)
# Edit your shell profile (~/.bashrc, ~/.zshrc, etc.)
```

## 🛠️ Development

### Building from Source
```bash
git clone https://github.com/distributedstatemachine/lium-rs.git
cd lium-rs/crates/lium-cli
cargo build --release
```

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for detailed guidelines.

## 📚 Resources

- **Documentation**: [GitHub Repository](https://github.com/distributedstatemachine/lium-rs)
- **Issues**: [Report bugs or request features](https://github.com/distributedstatemachine/lium-rs/issues)
- **Releases**: [Latest releases and binaries](https://github.com/distributedstatemachine/lium-rs/releases)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Support

- **GitHub Issues**: [Report bugs or request features](https://github.com/distributedstatemachine/lium-rs/issues)
- **Documentation**: [GitHub Repository](https://github.com/distributedstatemachine/lium-rs)

---

**Made with ❤️ by the Lium team**

</div> 