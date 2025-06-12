<div align="center">

# 🍄 Lium Utils

**Infrastructure utilities for system operations**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

## Overview

`lium-utils` provides essential infrastructure utilities for the lium ecosystem. This crate handles low-level system operations including:

- **SSH Operations**: Secure remote command execution and file transfers
- **Docker Integration**: Container management and orchestration utilities
- **Process Management**: Local and remote process execution
- **File Operations**: Cross-platform file system utilities
- **Configuration**: System configuration and environment management

## Features

- 🔐 **SSH Client**: Full-featured SSH client with key-based authentication
- 🐳 **Docker Support**: Async Docker API integration
- 🚀 **High Performance**: Async operations for maximum throughput
- 🔧 **Cross-Platform**: Works on Linux, macOS, and Windows
- 🛡️ **Security**: Secure credential handling and encrypted connections
- 📁 **File Utils**: Advanced file operations with proper error handling

## Usage

### SSH Operations

```rust
use lium_utils::ssh::SshClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SshClient::connect("user@hostname").await?;
    
    // Execute remote command
    let output = client.execute("ls -la").await?;
    println!("Remote output: {}", output);
    
    // Transfer file
    client.upload_file("local.txt", "/remote/path/file.txt").await?;
    
    Ok(())
}
```

### Docker Operations

```rust
use lium_utils::docker::DockerClient;

let docker = DockerClient::new()?;
let containers = docker.list_containers().await?;
println!("Found {} containers", containers.len());
```

### Process Management

```rust
use lium_utils::process::execute_command;

let result = execute_command("nvidia-smi", &[]).await?;
println!("GPU info: {}", result.stdout);
```

## Dependencies

- `ssh2` - SSH protocol implementation
- `bollard` - Docker API client
- `tokio` - Async runtime
- `regex` - Pattern matching
- `uuid` - Unique identifiers
- `base64` - Encoding utilities
- `home`/`dirs` - Path utilities

---

<div align="center">

*Part of the 🍄 lium ecosystem*

</div> 