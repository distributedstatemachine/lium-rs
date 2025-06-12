use crate::{
    config::Config,
    display::{print_error, print_info, print_success, print_warning},
    CliError, Result,
};
use dialoguer::{Input, Password};
use lium_api::LiumApiClient;
use lium_utils::build_and_push_image;
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};

/// Handle image subcommands (list, create, delete)
pub async fn handle_subcommand(action: crate::ImageCommands, config: &Config) -> Result<()> {
    use crate::ImageCommands;

    let api_client = LiumApiClient::from_config(config)?;

    match action {
        ImageCommands::List => handle_list(&api_client).await,
        ImageCommands::Create { name, image, tag } => handle_create(name, image, tag, config).await,
        ImageCommands::Delete { id } => handle_delete(&api_client, id).await,
    }
}

/// Handle creating a template from existing Docker image
async fn handle_create(
    name: String,
    image: String,
    tag: Option<String>,
    config: &Config,
) -> Result<()> {
    let tag = tag.unwrap_or_else(|| "latest".to_string());
    let full_image = format!("{}:{}", image, tag);

    print_info(&format!("🐳 Creating template: {}", name));
    print_info(&format!("📦 Docker image: {}", full_image));

    let api_client = LiumApiClient::from_config(config)?;

    // Create template from existing Docker image
    match api_client.post_image(&name, &full_image, &tag).await {
        Ok(_) => {
            print_success("✅ Template created successfully!");
            print_info(&format!("🎯 Template name: {}", name));
            print_info(&format!("📦 Docker image: {}", full_image));

            // Wait for verification
            print_info("🔄 Waiting for image verification...");
            match wait_for_image_verification_by_name(&api_client, &name, &full_image).await {
                Ok(template_id) => {
                    print_success("✅ Image is verified and ready to use!");
                    print_info(&format!("🎯 Template ID: {}", template_id));
                    print_info(&format!(
                        "💡 Use it: lium up <executor> --image {}",
                        template_id
                    ));

                    // Suggest setting as default
                    println!();
                    print_info("💡 To set as default template:");
                    print_info(&format!(
                        "   lium config set template.default_id {}",
                        template_id
                    ));
                }
                Err(e) => {
                    print_warning(&format!("⚠️  Image verification polling failed: {}", e));
                    print_info("💡 Check status with: lium image list");
                }
            }
        }
        Err(e) => {
            print_error(&format!("❌ Failed to create template: {}", e));
            return Err(e.into());
        }
    }

    Ok(())
}

/// Wait for image verification by template name
async fn wait_for_image_verification_by_name(
    api_client: &LiumApiClient,
    template_name: &str,
    docker_image: &str,
) -> Result<String> {
    const MAX_ATTEMPTS: u32 = 30; // 5 minutes with 10-second intervals
    const POLL_INTERVAL: Duration = Duration::from_secs(10);

    let start_time = std::time::Instant::now();

    for attempt in 1..=MAX_ATTEMPTS {
        match api_client.get_templates().await {
            Ok(templates) => {
                for template in templates {
                    // Match by template name or docker image
                    if template.name == template_name || template.docker_image == docker_image {
                        match template.status.as_deref() {
                            Some("VERIFY_SUCCESS") => {
                                return Ok(template.id.clone());
                            }
                            Some("VERIFY_FAILED") => {
                                return Err(CliError::OperationFailed(
                                    "Image verification failed. Please check your image."
                                        .to_string(),
                                ));
                            }
                            Some(status) => {
                                let elapsed = start_time.elapsed().as_secs();
                                print_info(&format!(
                                    "⏳ Status: {}, Elapsed: {}s (attempt {}/{})",
                                    status, elapsed, attempt, MAX_ATTEMPTS
                                ));
                            }
                            None => {
                                let elapsed = start_time.elapsed().as_secs();
                                print_info(&format!(
                                    "⏳ Status: pending, Elapsed: {}s (attempt {}/{})",
                                    elapsed, attempt, MAX_ATTEMPTS
                                ));
                            }
                        }
                        break;
                    }
                }
            }
            Err(e) => {
                print_warning(&format!("⚠️  Failed to check templates: {}", e));
            }
        }

        if attempt < MAX_ATTEMPTS {
            sleep(POLL_INTERVAL).await;
        }
    }

    Err(CliError::OperationFailed(
        "Image verification timed out after 5 minutes. Check status with: lium image list"
            .to_string(),
    ))
}

/// Handle building a new Docker image
pub async fn handle_build(
    image_name: String,
    path: String,
    dockerfile: Option<String>,
    config: &Config,
) -> Result<()> {
    print_info(&format!("🐳 Building Docker image: {}", image_name));
    print_info(&format!("📁 Build path: {}", path));

    // Validate build directory
    let build_path = Path::new(&path);
    if !build_path.exists() {
        return Err(CliError::InvalidInput(format!(
            "Build path not found: {}",
            path
        )));
    }

    // Determine Dockerfile path
    let dockerfile_path = if let Some(df) = dockerfile {
        // Use specified Dockerfile
        let df_path = build_path.join(&df);
        if !df_path.exists() {
            return Err(CliError::InvalidInput(format!(
                "Dockerfile not found: {}",
                df_path.display()
            )));
        }
        df_path
    } else {
        // Default to Dockerfile in build directory
        let default_df = build_path.join("Dockerfile");
        if !default_df.exists() {
            return Err(CliError::InvalidInput(format!(
                "Dockerfile not found in: {}",
                path
            )));
        }
        default_df
    };

    print_info(&format!(
        "📄 Using Dockerfile: {}",
        dockerfile_path.display()
    ));

    // Get or prompt for Docker credentials
    let (docker_user, docker_token) = get_docker_credentials(config).await?;

    print_info("🔨 Building and pushing image...");

    // Build and push the image
    let full_image_name = if image_name.contains(':') {
        image_name.clone()
    } else {
        format!("{}:latest", image_name)
    };

    let image_digest = build_and_push_image(
        &full_image_name,
        &dockerfile_path,
        &docker_user,
        &docker_token,
    )
    .await?;

    print_success("✅ Image built and pushed successfully");
    print_info(&format!("📦 Image: {}", full_image_name));
    print_info(&format!("🔍 Digest: {}", image_digest));

    // Register the image with Lium API
    print_info("📝 Registering image with Lium...");

    let api_client = LiumApiClient::from_config(config)?;

    // Extract name and tag
    let (name_part, tag) = if let Some((name, tag)) = full_image_name.split_once(':') {
        (name, tag)
    } else {
        (full_image_name.as_str(), "latest")
    };

    match api_client.post_image(name_part, &image_digest, tag).await {
        Ok(_) => {
            print_success("✅ Image registered with Lium successfully");

            // Wait for verification
            print_info("🔄 Waiting for image verification...");
            match wait_for_image_verification(&api_client, &image_digest).await {
                Ok(template_id) => {
                    print_success("✅ Image is verified and ready to use!");
                    print_info(&format!("🎯 Template ID: {}", template_id));
                    print_info(&format!(
                        "💡 Use it: lium up <executor> --image {}",
                        template_id
                    ));

                    // Suggest setting as default
                    println!();
                    print_info("💡 To set as default template:");
                    print_info(&format!(
                        "   lium config set template.default_id {}",
                        template_id
                    ));
                }
                Err(e) => {
                    print_warning(&format!("⚠️  Image verification failed: {}", e));
                    print_info("💡 Check status with: lium image list");
                }
            }
        }
        Err(e) => {
            print_error(&format!("❌ Failed to register image with Lium: {}", e));
            print_info("💡 The image was built and pushed, but registration failed.");
            return Err(e.into());
        }
    }

    Ok(())
}

/// Handle listing templates
async fn handle_list(api_client: &LiumApiClient) -> Result<()> {
    print_info("📋 Fetching available templates...");

    match api_client.get_templates().await {
        Ok(templates) => {
            if templates.is_empty() {
                println!("No templates found.");
                return Ok(());
            }

            println!("\n🎯 Available Templates:");
            println!(
                "{:<38} {:<25} {:<20} {:<12}",
                "ID", "Name", "Image", "Status"
            );
            println!("{}", "─".repeat(95));

            for template in templates {
                let image_display = template
                    .docker_image
                    .split('/')
                    .last()
                    .unwrap_or(&template.docker_image);

                let status_display = match template.status.as_deref() {
                    Some("VERIFY_SUCCESS") => "✅ verified",
                    Some("VERIFY_FAILED") => "❌ failed",
                    Some("VERIFYING") => "🔄 verifying",
                    Some(status) => status,
                    None => "pending",
                };

                println!(
                    "{:<38} {:<25} {:<20} {:<12}",
                    template.id, template.name, image_display, status_display
                );
            }

            println!("\n💡 To use a template: lium up <executor> --image <template-id>");
            println!("💡 To set default: lium config set template.default_id <template-id>");
        }
        Err(e) => {
            print_error(&format!("❌ Failed to fetch templates: {}", e));
            return Err(e.into());
        }
    }

    Ok(())
}

/// Handle deleting a template
async fn handle_delete(api_client: &LiumApiClient, id: String) -> Result<()> {
    print_info(&format!("🗑️  Deleting template: {}", id));

    // Check if template exists
    match api_client.get_templates().await {
        Ok(templates) => {
            let template_exists = templates.iter().any(|t| t.id == id);
            if !template_exists {
                print_error(&format!("❌ Template not found: {}", id));
                return Err(CliError::NotFound(format!("Template {} not found", id)));
            }
        }
        Err(e) => {
            print_error(&format!("❌ Failed to check templates: {}", e));
            return Err(e.into());
        }
    }

    // TODO: Implement delete_template in API client when available
    print_warning("⚠️  Template deletion is not yet implemented in the API");
    print_info("💡 Contact support to remove unused templates");

    Err(CliError::OperationFailed(
        "Template deletion not yet implemented".to_string(),
    ))
}

/// Get Docker credentials from config or prompt user
async fn get_docker_credentials(config: &Config) -> Result<(String, String)> {
    if let Some((username, token)) = config.get_docker_credentials()? {
        print_info(&format!(
            "🔑 Using stored Docker credentials for user: {}",
            username
        ));
        return Ok((username, token));
    }

    print_info("🔑 Docker credentials not found. Please provide them:");
    print_info("💡 Get a Docker Hub token from: https://hub.docker.com/settings/security");

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
    config_mut.save()?;

    print_success("💾 Docker credentials saved for future use");

    Ok((username, token))
}

/// Wait for image verification to complete
async fn wait_for_image_verification(
    api_client: &LiumApiClient,
    image_digest: &str,
) -> Result<String> {
    const MAX_ATTEMPTS: u32 = 30; // 5 minutes with 10-second intervals
    const POLL_INTERVAL: Duration = Duration::from_secs(10);

    let start_time = std::time::Instant::now();

    for attempt in 1..=MAX_ATTEMPTS {
        match api_client.get_templates().await {
            Ok(templates) => {
                for template in templates {
                    // Try to match by docker_image containing the digest
                    // or by checking if this is a recently created template
                    // Since we don't have docker_image_digest field, we'll match by image name
                    if template.docker_image.contains(&image_digest)
                        || template.docker_image == image_digest
                    {
                        match template.status.as_deref() {
                            Some("VERIFY_SUCCESS") => {
                                return Ok(template.id.clone());
                            }
                            Some("VERIFY_FAILED") => {
                                return Err(CliError::OperationFailed(
                                    "Image verification failed. Please check your image."
                                        .to_string(),
                                ));
                            }
                            Some(status) => {
                                let elapsed = start_time.elapsed().as_secs();
                                print_info(&format!(
                                    "⏳ Status: {}, Elapsed: {}s (attempt {}/{})",
                                    status, elapsed, attempt, MAX_ATTEMPTS
                                ));
                            }
                            None => {
                                let elapsed = start_time.elapsed().as_secs();
                                print_info(&format!(
                                    "⏳ Status: pending, Elapsed: {}s (attempt {}/{})",
                                    elapsed, attempt, MAX_ATTEMPTS
                                ));
                            }
                        }
                        break;
                    }
                }
            }
            Err(e) => {
                print_warning(&format!("⚠️  Failed to check templates: {}", e));
            }
        }

        if attempt < MAX_ATTEMPTS {
            sleep(POLL_INTERVAL).await;
        }
    }

    Err(CliError::OperationFailed(
        "Image verification timed out after 5 minutes. Check status with: lium image list"
            .to_string(),
    ))
}
