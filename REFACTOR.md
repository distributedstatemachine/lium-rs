# Lium Codebase Refactoring Plan

## Overview

This document outlines the comprehensive refactoring strategy for the Lium Rust codebase to improve modularity, testability, and maintainability. The refactoring follows clean architecture principles and Rust best practices.

## Motivation

The original codebase had several issues:
- **Monolithic structure**: All logic mixed together in a single crate
- **Circular dependencies**: Models, errors, and business logic tightly coupled
- **External dependencies in core**: Domain logic contaminated with I/O concerns
- **Poor testability**: Hard to unit test core business logic
- **Maintenance complexity**: Changes to one area affecting unrelated components

## Target Architecture

### Workspace Structure
```
lium-rs/
├── Cargo.toml                 # Workspace root
├── crates/
│   └── lium-core/            # Pure domain logic crate
│       ├── Cargo.toml        # No external dependencies
│       └── src/
│           ├── lib.rs        # Core domain exports
│           ├── models.rs     # Domain entities (ExecutorInfo, PodInfo, etc.)
│           ├── errors.rs     # Domain-specific errors only
│           └── optimization.rs # Business logic (Pareto optimization, etc.)
└── src/                      # Main application crate
    ├── lib.rs               # Re-exports + application modules
    ├── errors.rs            # Application errors (wraps core + I/O)
    ├── api.rs               # HTTP client
    ├── cli.rs               # Command-line interface
    ├── config.rs            # Configuration management
    └── commands/            # CLI command handlers
```

### Dependency Flow
```
┌─────────────────┐
│   CLI Commands  │
├─────────────────┤
│   Application   │ ← HTTP, SSH, Docker, File I/O
│   Layer         │
├─────────────────┤
│   lium-core     │ ← Pure domain logic
│   (No deps)     │   No external dependencies
└─────────────────┘
```

## Refactoring Strategy

### Phase 1: Create Core Domain Crate ✅
1. **Setup workspace structure**
   - Create `Cargo.toml` workspace
   - Create `crates/lium-core` subdirectory
   - Configure workspace dependencies

2. **Extract pure domain models**
   - Move `ExecutorInfo`, `PodInfo`, `TemplateInfo` to `lium-core/src/models.rs`
   - Add utility functions for self-contained operations
   - Ensure zero external dependencies

3. **Extract domain errors**
   - Create pure `LiumError` enum in `lium-core/src/errors.rs`
   - Only domain-specific error variants (no I/O, network, etc.)
   - Use `thiserror` for error definitions

4. **Extract business logic**
   - Move Pareto optimization algorithms to `lium-core/src/optimization.rs`
   - Keep pure mathematical/algorithmic functions
   - No external API calls or I/O operations

### Phase 2: Restructure Application Layer ✅
1. **Update main crate structure**
   - Re-export core types from `src/lib.rs`
   - Create application-level error enum that wraps core errors
   - Add I/O-specific errors (API, SSH, Docker, Config, etc.)

2. **Fix import statements**
   - Update all modules to import from correct locations
   - Use `crate::errors::Result` for application code
   - Use `lium_core::Result` only when working with pure domain logic

3. **Legacy compatibility layer**
   - Keep `helpers.rs` as compatibility shim
   - Re-export functions from specialized modules
   - Mark for gradual deprecation

### Phase 3: Specialized Module Organization ✅
1. **Infrastructure modules**
   - `api.rs` - HTTP client for external API
   - `ssh_utils.rs` - SSH operations
   - `docker_utils.rs` - Docker operations
   - `config.rs` - Configuration management

2. **Feature modules**
   - `formatters.rs` - Display formatting utilities
   - `gpu_utils.rs` - GPU-specific operations
   - `id_generator.rs` - ID generation utilities
   - `parsers.rs` - String parsing utilities
   - `pod_utils.rs` - Pod manipulation utilities
   - `resolvers.rs` - Target resolution logic
   - `storage.rs` - Selection persistence

3. **Command modules**
   - `commands/` - CLI command implementations
   - Each command imports only what it needs
   - Clear separation of concerns

### Phase 4: Cleanup and Optimization (In Progress)
1. **Remove unused imports** ✅
2. **Fix compilation warnings** ✅ 
3. **Update documentation**
4. **Add comprehensive tests**
5. **Performance optimizations**

## Current Status

### ✅ Completed
- [x] Workspace structure setup
- [x] Core domain crate creation (`lium-core`)
- [x] Domain models extraction with utility functions
- [x] Pure domain errors (no external dependencies)
- [x] Business logic extraction (optimization algorithms)
- [x] Application error layer (wraps core + adds I/O errors)
- [x] Import statement fixes across all modules
- [x] Legacy compatibility layer in `helpers.rs`
- [x] Compilation fixes and cleanup

### 🚧 In Progress
- [ ] Final cleanup of unused imports
- [ ] Documentation updates
- [ ] Test coverage improvements

### 📋 TODO
- [ ] Add more comprehensive unit tests for core domain logic
- [ ] Add integration tests for API layer
- [ ] Performance benchmarking and optimization
- [ ] Consider extracting more specialized crates (e.g., `lium-api`)
- [ ] Gradually deprecate `helpers.rs` compatibility layer
- [ ] Add workspace-level documentation

## Benefits Achieved

### 1. **Improved Testability**
- Core domain logic can be unit tested in isolation
- No need to mock HTTP clients or file systems for business logic tests
- Fast test execution for pure functions

### 2. **Better Modularity**
- Clear separation between domain logic and infrastructure
- Specialized modules with single responsibilities
- Reduced coupling between components

### 3. **Enhanced Maintainability**
- Changes to core business logic don't affect I/O code
- Easy to swap out HTTP clients, database drivers, etc.
- Clear dependency boundaries

### 4. **Reusability**
- `lium-core` can be used by other applications
- Domain logic is portable across different interfaces (CLI, web, etc.)
- Business rules centralized and consistent

### 5. **Compilation Performance**
- Smaller compilation units
- Parallel compilation of workspace crates
- Incremental compilation benefits

## Migration Notes

### For Developers
1. **Import Changes**
   ```rust
   // Old
   use crate::models::ExecutorInfo;
   use crate::errors::Result;
   
   // New
   use lium_core::ExecutorInfo;
   use crate::errors::Result; // Application errors
   // or
   use lium_core::Result;    // Core domain errors
   ```

2. **Error Handling**
   ```rust
   // Application code with I/O
   fn api_call() -> crate::errors::Result<Vec<ExecutorInfo>> {
       // Can handle both domain and I/O errors
   }
   
   // Pure domain code
   fn calculate_pareto(executors: &[ExecutorInfo]) -> lium_core::Result<Vec<ExecutorInfo>> {
       // Only domain errors
   }
   ```

3. **Backward Compatibility**
   - Most existing code continues to work through re-exports
   - Gradual migration path available
   - Deprecation warnings guide updates

## Architecture Principles

### 1. **Dependency Inversion**
- Core domain doesn't depend on infrastructure
- Infrastructure depends on and implements core interfaces
- Abstractions owned by core domain

### 2. **Single Responsibility**
- Each module has one clear purpose
- Minimal interfaces between modules
- High cohesion within modules

### 3. **Open/Closed Principle**
- Core domain open for extension, closed for modification
- New features added through new modules
- Existing business logic remains stable

### 4. **Interface Segregation**
- Small, focused traits and interfaces
- Clients depend only on what they use
- No forced dependencies on unused functionality

## Future Considerations

### Potential Further Refactoring
1. **Extract API client to separate crate** (`lium-api`)
2. **Create CLI-specific crate** (`lium-cli`)
3. **Add plugin system** for extensible commands
4. **Consider async traits** for better testability
5. **Add tracing/observability** throughout the stack

### Performance Optimizations
1. **Caching layer** for API responses
2. **Connection pooling** for SSH operations
3. **Parallel operations** where appropriate
4. **Memory optimization** for large data sets

### Testing Strategy
1. **Unit tests** for all core domain logic
2. **Integration tests** for API interactions
3. **End-to-end tests** for CLI workflows
4. **Property-based tests** for optimization algorithms
5. **Performance benchmarks** for critical paths

---

*This refactoring represents a significant improvement in code organization, testability, and maintainability while preserving backward compatibility and providing a clear migration path.* 

## Current Status Analysis

### ✅ Completed (Phase 1)
- [x] Workspace structure setup
- [x] Core domain crate creation (`lium-core`)  
- [x] Domain models extraction with utility functions
- [x] Pure domain errors (no external dependencies)
- [x] Business logic extraction (optimization algorithms)
- [x] Application error layer (wraps core + adds I/O errors)
- [x] Import statement fixes across all modules
- [x] Legacy compatibility layer in `helpers.rs`
- [x] Compilation fixes and cleanup

### 🚨 Current Structure Issues

**File Size Analysis:**
```bash
src/cli.rs         1281 lines  ← **MASSIVE CODE SMELL**
src/display.rs      429 lines  
src/utils.rs        455 lines  ← **SHOULD BE IN lium-utils**
src/api.rs          297 lines  ← **EXTRACT TO lium-api**
src/sdk.rs          289 lines  ← **EXTRACT TO lium-api**
```

**Problems:**
1. **`cli.rs` is massive** (1281 lines) - violates SRP
2. **Mixed concerns** - API client, utilities, domain logic, and CLI all in one crate
3. **Tight coupling** - makes testing and reuse difficult
4. **Large compilation units** - everything rebuilds when anything changes
5. **Utils scattered** - utility functions mixed with application logic

## Remaining Refactoring Work

### Phase 2: Extract Infrastructure Crates

#### **1. Create `lium-utils` Crate (~1.2K lines)**
```
crates/lium-utils/src/
├── lib.rs
├── ssh.rs         (356 lines - from ssh_utils.rs)
├── docker.rs      (313 lines - from docker_utils.rs)  
├── gpu.rs         (80 lines - from gpu_utils.rs)
├── id_generator.rs (114 lines)
├── formatters.rs  (89 lines)
├── parsers.rs     (100 lines)
├── pod.rs         (146 lines - from pod_utils.rs)
└── file_utils.rs  (extracted from utils.rs)
```

**Purpose:** Infrastructure utilities, reusable across projects

**Dependencies:**
```toml
[dependencies]
lium-core = { path = "../lium-core" }
ssh2 = "0.9"
bollard = "0.16" 
regex = "1.10"
tokio = { workspace = true }
# NO CLI dependencies
```

#### **2. Create `lium-api` Crate (~600 lines)**
```
crates/lium-api/src/
├── lib.rs
├── client.rs      (297 lines - from api.rs)
├── sdk.rs         (289 lines - current sdk.rs)
└── models.rs      (re-export from lium-core)
```

**Purpose:** HTTP API client, can be used independently

**Dependencies:**
```toml
[dependencies]
lium-core = { path = "../lium-core" }
lium-utils = { path = "../lium-utils" }
reqwest = { workspace = true }
tokio = { workspace = true }
serde_json = { workspace = true }
# NO CLI dependencies
```

### Phase 3: Refactor Massive CLI Module

#### **Problem: `cli.rs` is 1281 lines!**

**Current Structure:**
```rust
// src/cli.rs - 1281 lines of mixed concerns
pub struct Cli { ... }           // CLI definition
impl Cli { 
    pub async fn run() { ... }   // Main runner logic 
}

// Command handlers mixed together:
// - Executor commands (ls, up)
// - Pod commands (ps, exec, ssh, down)  
// - File commands (scp, rsync)
// - Config commands (init, config, fund)
// - Utility functions
// - Error handling
// - Display logic
```

#### **Solution: Split into Focused Modules**

```
src/cli/
├── mod.rs              (50 lines - main CLI struct & runner)
├── executor_commands.rs (300 lines - ls, up commands)
├── pod_commands.rs     (400 lines - ps, exec, ssh, down)
├── file_commands.rs    (200 lines - scp, rsync) 
├── config_commands.rs  (150 lines - init, fund, config)
├── display_helpers.rs  (100 lines - CLI-specific display)
└── utils.rs           (81 lines - CLI-only utilities)
```

**Benefits:**
- **Single Responsibility** - each module has one concern
- **Easier Testing** - can test command handlers in isolation
- **Team Development** - different people can work on different commands
- **Faster Compilation** - only rebuild changed command modules

### Phase 4: Additional Workspace Crates

#### **Optional: Extract `lium-display` Crate**
```
crates/lium-display/src/
├── lib.rs
├── tables.rs       (table formatting)
├── prompts.rs      (interactive prompts)  
├── themes.rs       (color themes)
└── formatters.rs   (display formatters)
```

**Purpose:** Reusable display utilities for TUI/CLI apps

#### **Optional: Extract `lium-config` Crate**
```
crates/lium-config/src/
├── lib.rs
├── manager.rs      (config management)
├── migration.rs    (config migration)
└── validation.rs   (config validation)
```

**Purpose:** Configuration management, reusable across tools

## Target Architecture

### **Final Workspace Structure**
```
lium-rs/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── lium-core/               # Pure domain logic (NO deps)
│   │   ├── models.rs            # Domain entities  
│   │   ├── errors.rs            # Domain errors
│   │   └── optimization.rs      # Business logic
│   ├── lium-utils/              # Infrastructure utilities
│   │   ├── ssh.rs              # SSH operations
│   │   ├── docker.rs           # Docker operations
│   │   └── parsers.rs          # String parsing
│   ├── lium-api/               # HTTP API client
│   │   ├── client.rs           # HTTP client
│   │   └── sdk.rs              # High-level SDK
│   ├── lium-display/           # Display utilities (optional)
│   └── lium-config/            # Config management (optional)
└── src/                        # Main CLI application
    ├── main.rs                 # Entry point
    ├── cli/                    # **SPLIT UP cli.rs**
    │   ├── mod.rs              # Main CLI runner
    │   ├── executor_commands.rs
    │   ├── pod_commands.rs
    │   └── file_commands.rs
    └── commands/               # Individual command handlers
```

### **Dependency Flow**
```
┌─────────────────┐
│   CLI Commands  │ ← clap, dialoguer, colored
├─────────────────┤
│  lium-display   │ ← CLI display utilities
├─────────────────┤
│   lium-api      │ ← HTTP client, SDK
├─────────────────┤
│   lium-utils    │ ← SSH, Docker, file ops
├─────────────────┤
│   lium-core     │ ← Pure domain (NO external deps)
└─────────────────┘
```

## Implementation Plan

### **Step 1: Extract lium-utils (Week 1)**
```bash
# Create crate
mkdir -p crates/lium-utils/src
# Move files
mv src/ssh_utils.rs crates/lium-utils/src/ssh.rs
mv src/docker_utils.rs crates/lium-utils/src/docker.rs
mv src/gpu_utils.rs crates/lium-utils/src/gpu.rs
# Update imports across codebase
# Update Cargo.toml dependencies
```

### **Step 2: Extract lium-api (Week 2)**  
```bash
# Create crate
mkdir -p crates/lium-api/src
# Move files  
mv src/api.rs crates/lium-api/src/client.rs
mv src/sdk.rs crates/lium-api/src/sdk.rs
# Update imports
# Update Cargo.toml
```

### **Step 3: Refactor cli.rs (Week 3)**
```bash
# Create CLI module structure
mkdir src/cli
# Split cli.rs into focused modules
# Update main.rs to use new structure
# Test all commands work
```

### **Step 4: Optional Extractions (Week 4)**
```bash  
# Extract display utilities if beneficial
# Extract config management if complex enough
# Add comprehensive tests
# Performance optimization
```

## Success Metrics

### **Before Refactoring:**
- `src/cli.rs`: 1281 lines (unmaintainable)
- Single crate with mixed concerns
- Slow compilation (everything rebuilds)
- Hard to test components in isolation
- Cannot reuse API client elsewhere

### **After Refactoring:**
- **Modular CLI**: 6 focused modules <250 lines each
- **Reusable Crates**: API client, utilities can be used independently  
- **Fast Builds**: Only rebuild changed crates
- **Easy Testing**: Can test each crate in isolation
- **Clear Boundaries**: Domain logic separate from infrastructure
- **Team Development**: Multiple people can work on different crates

## Breaking Down cli.rs (Priority #1)

### **Current cli.rs Structure Analysis:**
```rust
// Lines 1-100: Imports and CLI struct definition
// Lines 101-300: Executor commands (ls, up)  
// Lines 301-600: Pod commands (ps, exec, ssh, down)
// Lines 601-800: File commands (scp, rsync)
// Lines 801-1000: Config commands (init, fund, config)
// Lines 1001-1200: Utility functions
// Lines 1201-1281: Error handling and misc
```

### **Split Strategy:**
1. **Keep main CLI runner small** (~50 lines)
2. **Group related commands** (executor, pod, file, config)
3. **Extract utilities** to appropriate crates
4. **Move display logic** to display helpers
5. **Preserve existing API** through re-exports

This refactoring will transform the codebase from a monolithic structure into a well-organized, modular workspace that follows Rust best practices and clean architecture principles. 