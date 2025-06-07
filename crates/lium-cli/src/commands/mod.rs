pub mod config;
pub mod down;
pub mod exec;
pub mod fund;
pub mod image;
pub mod init;
pub mod ls;
pub mod ps;
pub mod rsync;
pub mod ssh;
pub mod theme;
pub mod up;

// Re-export handle functions for convenience
pub mod scp {
    use crate::{config::Config, Result};

    pub async fn handle(
        source: String,
        destination: String,
        coldkey: Option<String>,
        hotkey: Option<String>,
        config: &Config,
    ) -> Result<()> {
        // TODO: Implement actual SCP functionality
        println!("⚠️  SCP functionality not yet implemented");
        println!("Source: {}", source);
        println!("Destination: {}", destination);
        if let Some(coldkey) = coldkey {
            println!("Coldkey: {}", coldkey);
        }
        if let Some(hotkey) = hotkey {
            println!("Hotkey: {}", hotkey);
        }
        println!("💡 Use 'lium ssh <pod>' to connect and manually transfer files");
        Ok(())
    }
}
