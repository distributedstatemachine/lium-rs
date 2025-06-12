use crate::{config::Config, CliError, Result};
use dialoguer::{Input, Password};
use lium_api::LiumApiClient;
use lium_utils::build_and_push_image;
use std::path::Path;

/// Handle the image command for Docker image management
pub async fn handle(action: crate::ImageCommands, config: &Config) -> Result<()> {
    use crate::ImageCommands;
    use lium_api::LiumApiClient;

    let api_client = LiumApiClient::from_config(config)?;

    match action {
        ImageCommands::List => handle_list(&api_client).await,
        ImageCommands::Create { name, image, tag } => handle_create(name, image, tag, config).await,
        ImageCommands::Delete { id } => handle_delete(&api_client, id).await,
    }
}

/// Handle listing templates
async fn handle_list(api_client: &LiumApiClient) -> Result<()> {
    println!("📋 Fetching available templates...");

    match api_client.get_templates().await {
        Ok(templates) => {
            if templates.is_empty() {
                println!("No templates found.");
                return Ok(());
            }

            println!("\n🎯 Available Templates:");
            println!(
                "{:<20} {:<30} {:<15} {:<12}",
                "ID", "Name", "Image", "Status"
            );
            println!("{}", "─".repeat(80));

            for template in templates {
                println!(
                    "{:<20} {:<30} {:<15} {:<12}",
                    template.id,
                    template.name,
                    template.docker_image,
                    template.status.as_deref().unwrap_or("unknown")
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch templates: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Handle creating a new template
async fn handle_create(
    name: String,
    image: String,
    tag: Option<String>,
    config: &Config,
) -> Result<()> {
    let tag = tag.unwrap_or_else(|| "latest".to_string());
    let full_image = format!("{}:{}", image, tag);

    println!("🐳 Creating template: {}", name);
    println!("📦 Docker image: {}", full_image);

    let api_client = LiumApiClient::from_config(config)?;

    // Create template from existing Docker image
    match api_client.post_image(&name, &full_image, &tag).await {
        Ok(_) => {
            println!("✅ Template created successfully!");
            println!("🎯 Template name: {}", name);
            println!("📦 Docker image: {}", full_image);

            // Wait for verification
            println!("🔄 Waiting for image verification...");
            if let Err(e) = wait_for_image_verification(&api_client, &name).await {
                println!("⚠️  Image verification polling failed: {}", e);
                println!("💡 You can check the status manually with 'lium image list'");
            }
        }
        Err(e) => {
            println!("❌ Failed to create template: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Handle deleting a template
async fn handle_delete(_api_client: &LiumApiClient, id: String) -> Result<()> {
    println!("🗑️  Deleting template: {}", id);

    // TODO: Implement delete_template in API client
    Err(CliError::OperationFailed(
        "Template deletion not yet implemented in API client".to_string(),
    ))
}

/// Legacy function - for backwards compatibility if needed
pub async fn handle_legacy(name: String, dockerfile_dir: String, config: &Config) -> Result<()> {
    println!("🐳 Building and pushing Docker image: {}", name);

    // Validate dockerfile directory
    let dockerfile_path = Path::new(&dockerfile_dir);
    if !dockerfile_path.exists() {
        return Err(CliError::InvalidInput(format!(
            "Directory not found: {}",
            dockerfile_dir
        )));
    }

    let dockerfile = dockerfile_path.join("Dockerfile");
    if !dockerfile.exists() {
        return Err(CliError::InvalidInput(format!(
            "Dockerfile not found in: {}",
            dockerfile_dir
        )));
    }

    // Get or prompt for Docker credentials
    let (docker_user, docker_token) = get_docker_credentials(config).await?;

    println!("🔨 Building image...");

    // Build and push the image
    let image_digest =
        build_and_push_image(&name, &dockerfile, &docker_user, &docker_token).await?;

    println!("✅ Image built and pushed successfully");
    println!("📦 Image: {}", name);
    println!("🔍 Digest: {}", image_digest);

    // Register the image with Lium API
    println!("📝 Registering image with Lium...");

    let api_client = lium_api::LiumApiClient::from_config(config)?;

    // Extract tag from image name (after the colon)
    let (image_name_part, tag) = if let Some((name_part, tag_part)) = name.split_once(':') {
        (name_part, tag_part)
    } else {
        (name.as_str(), "latest")
    };

    match api_client
        .post_image(image_name_part, &image_digest, tag)
        .await
    {
        Ok(_) => {
            println!("✅ Image registered with Lium successfully");

            // Poll for verification status
            println!("🔄 Waiting for image verification...");
            if let Err(e) = wait_for_image_verification(&api_client, image_name_part).await {
                println!("⚠️  Image verification polling failed: {}", e);
                println!("💡 You can check the status manually with 'lium image list'");
            }
        }
        Err(e) => {
            println!("⚠️  Failed to register image with Lium: {}", e);
            println!(
                "💡 The image was built and pushed successfully, but Lium registration failed."
            );
        }
    }

    Ok(())
}

/// Get Docker credentials from config or prompt user
async fn get_docker_credentials(config: &Config) -> Result<(String, String)> {
    if let Some((username, token)) = config.get_docker_credentials()? {
        println!("🔑 Using stored Docker credentials for user: {}", username);
        return Ok((username, token));
    }

    println!("🔑 Docker credentials not found. Please provide them:");
    println!(
        "💡 You can get a Docker Hub access token from: https://hub.docker.com/settings/security"
    );

    let username: String = Input::new()
        .with_prompt("Docker Hub username")
        .interact()
        .map_err(|e| CliError::InvalidInput(format!("Input error: {}", e)))?;

    let token: String = Password::new()
        .with_prompt("Docker Hub access token")
        .interact()
        .map_err(|e| CliError::InvalidInput(format!("Input error: {}", e)))?;

    if username.trim().is_empty() || token.trim().is_empty() {
        return Err(CliError::InvalidInput(
            "Username and token cannot be empty".to_string(),
        ));
    }

    // Store credentials for future use
    let mut config_mut = Config::new()?;
    config_mut.set_docker_credentials(&username, &token)?;

    // Save with timeout to prevent hanging
    use tokio::time::{timeout, Duration};
    match timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || config_mut.save()),
    )
    .await
    {
        Ok(result) => match result {
            Ok(save_result) => save_result?,
            Err(_) => {
                return Err(CliError::InvalidInput(
                    "Config save task failed".to_string(),
                ))
            }
        },
        Err(_) => {
            return Err(CliError::InvalidInput(
                "Config save timed out after 5 seconds".to_string(),
            ))
        }
    }

    println!("💾 Docker credentials saved for future use");

    Ok((username, token))
}

/// Wait for image verification to complete
async fn wait_for_image_verification(
    api_client: &lium_api::LiumApiClient,
    image_name: &str,
) -> Result<()> {
    use tokio::time::{sleep, Duration};

    const MAX_ATTEMPTS: u32 = 30; // 5 minutes with 10-second intervals
    const POLL_INTERVAL: Duration = Duration::from_secs(10);

    for attempt in 1..=MAX_ATTEMPTS {
        println!(
            "🔍 Checking verification status... (attempt {}/{})",
            attempt, MAX_ATTEMPTS
        );

        match api_client.get_templates().await {
            Ok(templates) => {
                for template in templates {
                    if template.docker_image.contains(image_name) {
                        match template.status.as_deref() {
                            Some("VERIFY_SUCCESS") => {
                                println!("✅ Image verification completed successfully!");
                                println!("🎯 Template ID: {}", template.id);
                                println!("📛 Template name: {}", template.name);
                                return Ok(());
                            }
                            Some("VERIFY_FAILED") => {
                                return Err(CliError::OperationFailed(
                                    "Image verification failed. Please check your image and try again.".to_string()
                                ));
                            }
                            Some(status) => {
                                println!("⏳ Status: {}", status);
                            }
                            None => {
                                println!("⏳ Status: pending");
                            }
                        }
                        break;
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to check templates: {}", e);
            }
        }

        if attempt < MAX_ATTEMPTS {
            sleep(POLL_INTERVAL).await;
        }
    }

    Err(CliError::OperationFailed(
        "Image verification polling timed out. Check status manually.".to_string(),
    ))
}
