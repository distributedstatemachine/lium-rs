use crate::config::Config;
use crate::errors::Result;
use dialoguer::Select;

/// Available themes
const AVAILABLE_THEMES: &[(&str, &str)] = &[
    ("default", "Default theme with standard colors"),
    ("dark", "Dark theme with muted colors"),
    ("light", "Light theme with bright colors"),
    ("minimal", "Minimal theme with reduced styling"),
    ("cyberpunk", "Cyberpunk theme with neon colors"),
];

/// Handle theme list command
pub async fn handle_theme_list() -> Result<()> {
    println!("🎨 Available themes:");
    for (name, description) in AVAILABLE_THEMES {
        println!("  {} - {}", name, description);
    }
    Ok(())
}

/// Handle theme set command
pub async fn handle_theme_set(name: Option<String>) -> Result<()> {
    let mut config = Config::new()?;

    let theme_name = if let Some(name) = name {
        // Validate the provided theme name
        if !AVAILABLE_THEMES.iter().any(|(theme, _)| *theme == name) {
            return Err(crate::errors::LiumError::InvalidInput(format!(
                "Unknown theme: {}. Use 'lium theme list' to see available themes.",
                name
            )));
        }
        name
    } else {
        // Interactive theme selection
        let theme_options: Vec<String> = AVAILABLE_THEMES
            .iter()
            .map(|(name, desc)| format!("{} - {}", name, desc))
            .collect();

        let selection = Select::new()
            .with_prompt("Select a theme")
            .items(&theme_options)
            .default(0)
            .interact()
            .map_err(|e| crate::errors::LiumError::InvalidInput(format!("Input error: {}", e)))?;

        AVAILABLE_THEMES[selection].0.to_string()
    };

    // Store theme in config
    config.set_value("display", "theme", &theme_name)?;
    config.save()?;

    println!("✅ Theme set to: {}", theme_name);
    println!("💡 The theme will be applied to future command outputs");

    Ok(())
}

/// Get the current theme from config
pub fn get_current_theme(config: &Config) -> String {
    config
        .get_value("display", "theme")
        .unwrap_or(None)
        .unwrap_or_else(|| "default".to_string())
}

/// Apply theme colors to a string (for display utilities)
pub fn apply_theme_color(text: &str, color_type: &str, theme: &str) -> String {
    match theme {
        "dark" => apply_dark_theme(text, color_type),
        "light" => apply_light_theme(text, color_type),
        "minimal" => text.to_string(), // No colors
        "cyberpunk" => apply_cyberpunk_theme(text, color_type),
        _ => apply_default_theme(text, color_type), // default theme
    }
}

fn apply_default_theme(text: &str, color_type: &str) -> String {
    match color_type {
        "header" => format!("\x1b[1;36m{}\x1b[0m", text), // Bold cyan
        "success" => format!("\x1b[32m{}\x1b[0m", text),  // Green
        "error" => format!("\x1b[31m{}\x1b[0m", text),    // Red
        "warning" => format!("\x1b[33m{}\x1b[0m", text),  // Yellow
        "info" => format!("\x1b[34m{}\x1b[0m", text),     // Blue
        "accent" => format!("\x1b[35m{}\x1b[0m", text),   // Magenta
        _ => text.to_string(),
    }
}

fn apply_dark_theme(text: &str, color_type: &str) -> String {
    match color_type {
        "header" => format!("\x1b[1;37m{}\x1b[0m", text), // Bold white
        "success" => format!("\x1b[32m{}\x1b[0m", text),  // Green
        "error" => format!("\x1b[91m{}\x1b[0m", text),    // Bright red
        "warning" => format!("\x1b[93m{}\x1b[0m", text),  // Bright yellow
        "info" => format!("\x1b[94m{}\x1b[0m", text),     // Bright blue
        "accent" => format!("\x1b[95m{}\x1b[0m", text),   // Bright magenta
        _ => text.to_string(),
    }
}

fn apply_light_theme(text: &str, color_type: &str) -> String {
    match color_type {
        "header" => format!("\x1b[1;30m{}\x1b[0m", text), // Bold black
        "success" => format!("\x1b[32m{}\x1b[0m", text),  // Green
        "error" => format!("\x1b[31m{}\x1b[0m", text),    // Red
        "warning" => format!("\x1b[33m{}\x1b[0m", text),  // Yellow
        "info" => format!("\x1b[34m{}\x1b[0m", text),     // Blue
        "accent" => format!("\x1b[35m{}\x1b[0m", text),   // Magenta
        _ => text.to_string(),
    }
}

fn apply_cyberpunk_theme(text: &str, color_type: &str) -> String {
    match color_type {
        "header" => format!("\x1b[1;96m{}\x1b[0m", text), // Bold bright cyan
        "success" => format!("\x1b[92m{}\x1b[0m", text),  // Bright green
        "error" => format!("\x1b[91m{}\x1b[0m", text),    // Bright red
        "warning" => format!("\x1b[93m{}\x1b[0m", text),  // Bright yellow
        "info" => format!("\x1b[96m{}\x1b[0m", text),     // Bright cyan
        "accent" => format!("\x1b[95m{}\x1b[0m", text),   // Bright magenta
        _ => text.to_string(),
    }
}
