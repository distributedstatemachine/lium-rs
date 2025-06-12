//! # Lium CLI Application Entry Point
//!
//! This is the main entry point for the Lium CLI application. It serves as a thin wrapper around
//! the lium-cli crate, which contains all the core CLI functionality. This separation allows for
//! better modularity and easier testing of the CLI components.
//!
//! ## Key Features
//! - Custom panic handler for improved error reporting
//! - Async runtime setup using tokio
//! - Error handling and graceful exit on failures
//!
//! ## Usage
//! The application is typically run from the command line. All CLI commands and functionality
//! are handled by the lium-cli crate.
//!
//! ## Error Handling
//! - Panics are caught and displayed with file and line information
//! - CLI errors are displayed and result in a non-zero exit code
//!
//! ## Dependencies
//! - tokio: Async runtime
//! - lium-cli: Core CLI functionality

use std::process;

#[tokio::main]
async fn main() {
    // Set up panic handler for better error messages
    // This provides detailed error information including file and line numbers
    // when a panic occurs, making debugging easier
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Error: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("Location: {}:{}", location.file(), location.line());
        }
        process::exit(1);
    }));

    // Run the CLI - all functionality is now in lium-cli crate
    // Any errors from the CLI execution are caught and displayed
    if let Err(e) = lium_cli::run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
