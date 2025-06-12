<div align="center">

# 🍄 Lium Core

**Core domain logic for lium GPU compute management**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

## Overview

`lium-core` provides the foundational domain types and business logic for the lium GPU compute management system. This crate contains:

- **Domain Models**: Core data structures for pods, executors, jobs, and compute resources
- **Business Logic**: Validation, state management, and core algorithms
- **Type Definitions**: Shared types used across the entire Lium ecosystem
- **Error Types**: Comprehensive error handling for domain operations

## Features

- 🚀 **High Performance**: Optimized data structures for GPU compute workloads
- 🔒 **Type Safety**: Leverages Rust's type system for robust domain modeling
- 📊 **Resource Management**: Efficient tracking of compute resources and scheduling
- 🌐 **Serialization**: Full JSON serialization support for API compatibility

## Usage

```rust
use lium_core::{PodInfo, ExecutorInfo, JobStatus};

// Create and work with domain objects
let pod = PodInfo::new("my-pod", "gpu-node-1");
let status = JobStatus::Running;
```

## Dependencies

- `serde` - Serialization/deserialization
- `chrono` - Date and time handling  
- `uuid` - Unique identifier generation
- `thiserror` - Error handling

---

<div align="center">

*Part of the 🍄 lium ecosystem*

</div> 