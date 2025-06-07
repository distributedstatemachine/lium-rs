<div align="center">

# 🍄 Lium API

**HTTP API client for Lium compute services**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

## Overview

`lium-api` provides a comprehensive HTTP client for interacting with Lium compute services. This crate handles all API communication including:

- **Pod Management**: Create, list, update, and delete compute pods
- **Executor Operations**: Manage job executors and their lifecycle
- **Job Submission**: Submit and monitor compute jobs
- **Resource Monitoring**: Query system resources and usage statistics
- **Authentication**: Secure API authentication and session management

## Features

- 🌐 **Async/Await**: Built on `tokio` for high-performance async operations
- 🔐 **Secure**: TLS-enabled HTTP client with certificate validation
- 📡 **Comprehensive**: Full coverage of Lium API endpoints
- 🚀 **Efficient**: Connection pooling and request optimization
- 🛡️ **Robust**: Comprehensive error handling and retry logic

## Usage

```rust
use lium_api::LiumApiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = LiumApiClient::new("https://api.lium.dev")?;
    
    // List all available pods
    let pods = client.get_pods().await?;
    println!("Found {} pods", pods.len());
    
    // Submit a new job
    let job_id = client.submit_job("my-pod", "echo hello").await?;
    println!("Job submitted: {}", job_id);
    
    Ok(())
}
```

## Configuration

The API client supports various configuration options:

```rust
let client = LiumApiClient::builder()
    .base_url("https://api.lium.dev")
    .timeout(Duration::from_secs(30))
    .retry_attempts(3)
    .build()?;
```

## Dependencies

- `reqwest` - HTTP client with TLS support
- `serde`/`serde_json` - JSON serialization
- `tokio` - Async runtime
- `url` - URL parsing and validation
- `chrono` - Date/time handling

---

<div align="center">

*Part of the 🍄 Lium ecosystem*

</div> 