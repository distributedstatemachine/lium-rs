<div align="center">

# 🍄 Lium CLI

**Command-line interface for GPU compute management**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

## Overview

`lium-cli` is the primary command-line interface for the Lium GPU compute management system. It provides a comprehensive set of commands for:

- **Pod Management**: List, create, and manage compute pods
- **Job Execution**: Submit and monitor compute jobs across clusters  
- **Resource Monitoring**: View system resources and usage statistics
- **Configuration**: Manage API endpoints and authentication
- **Interactive Operations**: User-friendly prompts and selections

## Features

- 🎯 **Intuitive Commands**: Simple, memorable command structure
- 🎨 **Rich Output**: Colored, formatted output with progress indicators
- 🔧 **Interactive Mode**: Smart prompts and multi-selection interfaces
- ⚡ **High Performance**: Async operations for fast execution
- 🛡️ **Error Handling**: Comprehensive error messages and recovery
- 📋 **Flexible Targeting**: Support for indices, names, HUIDs, and "all"

## Installation

```bash
# Build from source
cargo install --path crates/lium-cli

# Or use the main binary
cargo build --release
./target/release/lium --help
```

## Usage

### Basic Commands

```bash
# List all pods
lium ls

# Submit a job to a specific pod  
lium exec my-pod "python train.py"

# Execute on multiple pods
lium exec 1,3,5 "nvidia-smi"

# Interactive pod selection
lium exec --interactive "python -c 'print(\"Hello GPU!\")'"

# Monitor job status
lium status job-123
```

### Advanced Usage

```bash
# Execute on all pods
lium exec all "pip install torch"

# Use HUID targeting
lium exec brave-cat-1234 "python model.py"

# Copy files to pods
lium cp model.py my-pod:/workspace/

# Real-time logs
lium logs --follow job-456
```

### Configuration

```bash
# Set API endpoint
lium config set api-url https://api.lium.dev

# View current configuration
lium config show
```

## Command Structure

```
lium <COMMAND> [OPTIONS] [ARGS]

Commands:
  ls        List pods and executors
  exec      Execute commands on pods
  cp        Copy files to/from pods
  logs      View job logs
  status    Check job status
  config    Manage configuration
  help      Show help information
```

## Dependencies

- `clap` - Command-line argument parsing
- `dialoguer` - Interactive prompts
- `colored` - Terminal color output
- `tokio` - Async runtime
- `serde`/`serde_json` - Configuration handling

---

<div align="center">

*Part of the 🍄 Lium ecosystem*

</div> 