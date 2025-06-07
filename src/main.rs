//! # Lium CLI Application Entry Point
//!
//! This is now a thin wrapper around the lium-cli crate.
//! All CLI functionality has been moved to the lium-cli crate for better modularity.

use std::process;

#[tokio::main]
async fn main() {
    // Set up panic handler for better error messages
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Error: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("Location: {}:{}", location.file(), location.line());
        }
        process::exit(1);
    }));

    // Run the CLI - all functionality is now in lium-cli crate
    if let Err(e) = lium_cli::run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
