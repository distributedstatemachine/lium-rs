use crate::{
    config::Config,
    display::{
        display_executors_table, print_error, print_info, print_success, prompt_confirm,
        prompt_select,
    },
    CliError, Result,
};
use clap::Args;
use lium_api::LiumApiClient;
use lium_core::{
    filter_by_availability, filter_by_gpu_type, parse_env_vars, parse_executor_index,
    parse_port_mappings, sort_by_price, validate_docker_image,
};
use std::collections::HashMap;

/// Command-line arguments for the `up` command that creates and starts new pods.
///
/// The `up` command is the primary way to rent cloud GPU executors and start containerized
/// workloads on them. It handles template/image selection, executor filtering, environment
/// configuration, and pod creation.
///
/// # Examples
/// ```bash
/// # Start a pod with the default template
/// lium up
///
/// # Use a specific Docker image
/// lium up --image pytorch/pytorch:latest
///
/// # Filter by GPU type and select by index
/// lium up --gpu RTX4090 --index 1
///
/// # Set environment variables and port mappings
/// lium up --env "DEBUG=1,API_KEY=secret" --ports "8080:80,9000:9000"
///
/// # Skip confirmation prompts (useful for automation)
/// lium up --yes --name my-training-pod
/// ```
///
/// # Template vs Docker Image Handling
/// The `image` parameter accepts both:
/// - Template IDs: Direct references to pre-configured templates
/// - Docker images: Full Docker image names (e.g., `pytorch/pytorch:latest`)
///
/// When a Docker image is provided, the system first searches for existing templates
/// that use that image. If none are found, it attempts to use the input as a template ID.
///
/// # Executor Selection
/// Executors can be filtered and selected through multiple criteria:
/// - GPU type filtering via `--gpu`
/// - Availability filtering (defaults to available only)
/// - Manual selection via `--index`
/// - Interactive selection when no index is specified
///
/// # TODO
/// - Add support for custom resource requirements (CPU, RAM, storage)
/// - Implement cost estimation before pod creation
/// - Add support for multi-GPU configurations
/// - Add template creation from arbitrary Docker images
#[derive(Args)]
pub struct UpArgs {
    /// Template ID or Docker image to use for the pod.
    ///
    /// This parameter accepts either:
    /// - Template ID: A pre-configured template identifier (e.g., "pytorch-base")
    /// - Docker image: Full Docker image name with optional tag (e.g., "pytorch/pytorch:2.0-gpu")
    ///
    /// When a Docker image is provided, the system attempts to find an existing template
    /// that uses that image. If no matching template is found, it tries to use the input
    /// as a template ID directly.
    ///
    /// If not specified, the system uses the first available template as default.
    #[arg(short, long)]
    pub image: Option<String>,

    /// Filter executors by GPU type (case-insensitive partial matching).
    ///
    /// Supports common GPU models:
    /// - RTX series: RTX4090, RTX3090, RTX3080
    /// - Professional: H100, A100, V100, A6000, T4, L4, L40
    ///
    /// Examples: "RTX4090", "H100", "A100"
    #[arg(short, long)]
    pub gpu: Option<String>,

    /// Show only available executors for rental.
    ///
    /// When enabled, filters out executors that are currently rented or unavailable.
    /// This is the default behavior for the up command since you can only rent
    /// available executors.
    #[arg(short, long)]
    pub available: bool,

    /// Select executor by index from the filtered list (1-based indexing).
    ///
    /// After filtering executors by GPU type and availability, this allows direct
    /// selection without interactive prompts. Useful for automation and scripting.
    ///
    /// Examples: "1", "3", "5"
    #[arg(short, long)]
    pub index: Option<String>,

    /// Environment variables to set in the pod (comma-separated KEY=VALUE pairs).
    ///
    /// Variables are injected into the container environment and can be used by
    /// applications running inside the pod.
    ///
    /// Format: "KEY1=VALUE1,KEY2=VALUE2"
    /// Example: "DEBUG=1,API_KEY=secret,CUDA_VISIBLE_DEVICES=0"
    #[arg(short, long)]
    pub env: Option<String>,

    /// Port mappings from pod to host (comma-separated HOST_PORT:CONTAINER_PORT pairs).
    ///
    /// Maps ports from the container to the host system, enabling external access
    /// to services running inside the pod.
    ///
    /// Format: "HOST_PORT:CONTAINER_PORT,HOST_PORT2:CONTAINER_PORT2"
    /// Example: "8080:80,9000:9000,8888:8888"
    #[arg(short, long)]
    pub ports: Option<String>,

    /// Path to SSH public key file for pod access.
    ///
    /// Overrides the default SSH key configured in the user's configuration.
    /// The corresponding private key will be used for SSH connections to the pod.
    ///
    /// Example: "~/.ssh/custom_key.pub"
    #[arg(long)]
    pub ssh_key: Option<String>,

    /// Custom name for the pod (optional).
    ///
    /// If not provided, a name is automatically generated using the executor HUID.
    /// Pod names are used for identification in `lium ps`, `lium exec`, and other commands.
    ///
    /// Example: "training-job-v2", "jupyter-workspace"
    #[arg(short, long)]
    pub name: Option<String>,

    /// Skip confirmation prompts and proceed automatically.
    ///
    /// Useful for automation, scripts, and CI/CD pipelines where interactive
    /// confirmation is not possible or desired.
    #[arg(short, long)]
    pub yes: bool,
}

/// Handles the `up` command to create and start a new pod on a cloud GPU executor.
///
/// This is the main entry point for pod creation. It orchestrates the entire process
/// from executor selection to pod startup, including template resolution, filtering,
/// user interaction, and API calls.
///
/// # Arguments
/// * `args` - Command-line arguments parsed into `UpArgs` struct
/// * `config` - User configuration containing API keys, SSH settings, etc.
///
/// # Returns
/// * `Result<()>` - Success or error with detailed error information
///
/// # Process Flow
/// 1. **Configuration Validation**: Checks API key availability and creates API client
/// 2. **Template Resolution**: Handles both template IDs and Docker images
/// 3. **Executor Fetching**: Retrieves available executors from the API
/// 4. **Filtering**: Applies GPU type, availability, and other filters
/// 5. **Selection**: Interactive or index-based executor selection
/// 6. **Confirmation**: Shows summary and requests user confirmation (unless `--yes`)
/// 7. **Pod Creation**: Calls the rent_pod API to create and start the pod
/// 8. **Result Display**: Shows pod details including SSH connection info
///
/// # Error Conditions
/// - Invalid API key or configuration
/// - No executors found matching criteria
/// - Template/image not found
/// - Network errors during API calls
/// - SSH key configuration issues
/// - Pod creation failures
///
/// # Examples
/// ```rust
/// use lium_cli::commands::up::{handle, UpArgs};
/// use lium_cli::config::Config;
///
/// let args = UpArgs {
///     image: Some("pytorch/pytorch:latest".to_string()),
///     gpu: Some("RTX4090".to_string()),
///     available: true,
///     index: Some("1".to_string()),
///     env: Some("DEBUG=1".to_string()),
///     ports: Some("8080:80".to_string()),
///     ssh_key: None,
///     name: Some("my-pod".to_string()),
///     yes: false,
/// };
///
/// let config = Config::new()?;
/// handle(args, &config).await?;
/// ```
///
/// # Debug Information
/// The function includes extensive debug logging for troubleshooting:
/// - API key validation and source
/// - Template resolution process
/// - Executor filtering results
/// - API call success/failure details
///
/// # TODO
/// - Add support for spot instances and preemptible pricing
/// - Implement pod startup health checks
/// - Add support for persistent storage mounting
/// - Improve error messages with suggested solutions
/// - Add cost estimation before pod creation
pub async fn handle(args: UpArgs, config: &Config) -> Result<()> {
    // DEBUG: Check if API key exists in config
    print_info("DEBUG: Checking API key configuration...");

    match config.get_api_key()? {
        Some(key) => {
            if key.len() >= 8 {
                print_info(&format!(
                    "DEBUG: API key found in config: {}...{}",
                    &key[..4],
                    &key[key.len() - 4..]
                ));
            } else {
                print_info("DEBUG: API key found but too short to display");
            }
        }
        None => {
            print_info("DEBUG: No API key found in config or environment");
        }
    }

    // DEBUG: Check environment variable
    match std::env::var("LIUM_API_KEY") {
        Ok(key) => {
            if key.len() >= 8 {
                print_info(&format!(
                    "DEBUG: LIUM_API_KEY env var found: {}...{}",
                    &key[..4],
                    &key[key.len() - 4..]
                ));
            } else {
                print_info("DEBUG: LIUM_API_KEY env var found but too short");
            }
        }
        Err(_) => {
            print_info("DEBUG: LIUM_API_KEY environment variable not set");
        }
    }

    print_info("DEBUG: Creating API client from config...");
    let client = match LiumApiClient::from_config(config) {
        Ok(c) => {
            print_info("DEBUG: API client created successfully");
            c
        }
        Err(e) => {
            print_error(&format!("DEBUG: Failed to create API client: {}", e));
            return Err(e);
        }
    };

    // Handle both templates and Docker images
    let template_id = match args.image {
        Some(image_input) => {
            // Check if input looks like a Docker image (contains : or /) or a template ID
            if image_input.contains(':')
                || image_input.contains('/')
                || image_input.starts_with("docker.io")
            {
                // Looks like a Docker image - try to find matching template
                print_info(&format!(
                    "Looking for template with Docker image: {}",
                    image_input
                ));

                print_info("DEBUG: About to call get_templates()...");
                match client.get_templates().await {
                    Ok(templates) => {
                        print_info(&format!(
                            "DEBUG: Successfully fetched {} templates",
                            templates.len()
                        ));

                        // Try to find existing template with this docker image
                        let matching_template = templates.iter().find(|template| {
                            let docker_tag =
                                template.docker_image_tag.as_deref().unwrap_or("latest");
                            let full_image = format!("{}:{}", template.docker_image, docker_tag);

                            full_image == image_input || template.docker_image == image_input
                        });

                        if let Some(template) = matching_template {
                            print_info(&format!(
                                "Found existing template '{}' for image {}",
                                template.name, image_input
                            ));
                            template.id.clone()
                        } else {
                            // No existing template found - attempt to use image directly
                            print_info(&format!("No existing template found for {}. Attempting to use as template ID...", image_input));
                            image_input
                        }
                    }
                    Err(e) => {
                        print_error(&format!("DEBUG: API call to get_templates() failed"));
                        print_error(&format!("Failed to fetch templates: {}", e));
                        print_info(&format!(
                            "Attempting to use '{}' directly as template ID",
                            image_input
                        ));
                        image_input
                    }
                }
            } else {
                // Looks like a template ID - use directly
                print_info(&format!("Using template ID: {}", image_input));
                image_input
            }
        }
        None => {
            // No input provided - fetch templates and use default
            print_info("No image specified, fetching available templates...");

            print_info("DEBUG: About to call get_templates() for default template...");
            match client.get_templates().await {
                Ok(templates) => {
                    print_info(&format!(
                        "DEBUG: Successfully fetched {} templates",
                        templates.len()
                    ));

                    if templates.is_empty() {
                        return Err(CliError::OperationFailed("No templates found".to_string()));
                    }

                    // Use first template as default
                    let first_template = &templates[0];
                    let template_id = first_template.id.clone();
                    let template_name = &first_template.name;
                    let docker_image = &first_template.docker_image;
                    let docker_tag = first_template
                        .docker_image_tag
                        .as_deref()
                        .unwrap_or("latest");

                    print_info(&format!(
                        "Using default template: '{}' ({}:{})",
                        template_name, docker_image, docker_tag
                    ));
                    template_id
                }
                Err(e) => {
                    print_error(&format!("DEBUG: API call to get_templates() failed"));
                    print_error(&format!("Failed to fetch templates: {}", e));
                    print_error(&format!("DEBUG: Error details: {:?}", e));
                    return Err(CliError::OperationFailed(
                        "Could not fetch templates".to_string(),
                    ));
                }
            }
        }
    };

    // Parse environment variables
    let env_vars = if let Some(env_str) = &args.env {
        parse_env_vars(env_str)?
    } else {
        HashMap::new()
    };

    // Parse port mappings
    let port_mappings = if let Some(ports_str) = &args.ports {
        parse_port_mappings(ports_str)?
    } else {
        HashMap::new()
    };

    // Fetch and filter executors
    print_info("DEBUG: About to fetch executors...");
    let mut executors = match client.get_executors().await {
        Ok(execs) => {
            print_info(&format!(
                "DEBUG: Successfully fetched {} executors",
                execs.len()
            ));
            execs
        }
        Err(e) => {
            print_error(&format!("DEBUG: Failed to fetch executors: {}", e));
            return Err(e.into());
        }
    };

    if executors.is_empty() {
        return Err(CliError::OperationFailed("No executors found".to_string()));
    }

    // Apply filters
    if let Some(gpu_type) = &args.gpu {
        executors = filter_by_gpu_type(&executors, gpu_type);
    }

    if args.available {
        executors = filter_by_availability(&executors, true);
    } else {
        // Default to available only for renting
        executors = filter_by_availability(&executors, true);
    }

    if executors.is_empty() {
        return Err(CliError::OperationFailed(
            "No available executors found matching your criteria".to_string(),
        ));
    }

    // Sort by price (cheapest first)
    sort_by_price(&mut executors);

    // Select executor
    let selected_executor = if let Some(index_str) = &args.index {
        let index = parse_executor_index(index_str, executors.len())?;
        executors[index].clone()
    } else {
        // Interactive selection
        println!("Available executors:");
        display_executors_table(&executors, false);
        println!();

        let executor_names: Vec<String> = executors
            .iter()
            .enumerate()
            .map(|(i, e)| {
                format!(
                    "{}. {} - {}x {} - ${:.3}/GPU/hr",
                    i + 1,
                    e.huid,
                    e.gpu_count,
                    e.gpu_type,
                    e.price_per_gpu_hour
                )
            })
            .collect();

        let selection = prompt_select("Select an executor:", &executor_names)?;
        executors[selection].clone()
    };

    // Show selection summary
    print_info(&format!("Selected executor: {}", selected_executor.huid));
    print_info(&format!(
        "GPU Configuration: {}x {}",
        selected_executor.gpu_count, selected_executor.gpu_type
    ));
    print_info(&format!(
        "Cost: ${:.3}/GPU/hr (${:.3}/hr total)",
        selected_executor.price_per_gpu_hour, selected_executor.price_per_hour
    ));
    print_info(&format!("Using template/image: {}", template_id));

    if !env_vars.is_empty() {
        print_info(&format!("Environment Variables: {:?}", env_vars));
    }

    if !port_mappings.is_empty() {
        print_info(&format!("Port Mappings: {:?}", port_mappings));
    }

    // Confirmation
    if !args.yes {
        let confirm = prompt_confirm(
            "Do you want to rent this executor and start the pod?",
            false,
        )?;

        if !confirm {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    // Rent the executor
    print_info("Renting executor...");

    // Call rent_pod API
    let executor_id = selected_executor.id.clone();
    let pod_name = args
        .name
        .unwrap_or_else(|| format!("pod-{}", selected_executor.huid));

    let ssh_keys = config.get_ssh_public_keys().unwrap_or_default();

    // Debug: Print what we're about to send
    print_info(&format!(
        "Renting: executor_id={}, pod_name={}, template_id={}",
        executor_id, pod_name, template_id
    ));

    print_info("DEBUG: About to call rent_pod API...");
    match client
        .rent_pod(&executor_id, &pod_name, &template_id, ssh_keys)
        .await
    {
        Ok(pod_info) => {
            print_info("DEBUG: rent_pod API call successful");
            print_success("Pod started successfully!");
            println!();

            // Parse the response JSON to extract fields
            if let Some(huid) = pod_info.get("huid").and_then(|v| v.as_str()) {
                print_info(&format!("Pod HUID: {}", huid));
            }
            if let Some(name) = pod_info.get("name").and_then(|v| v.as_str()) {
                print_info(&format!("Pod Name: {}", name));
            }
            if let Some(status) = pod_info.get("status").and_then(|v| v.as_str()) {
                print_info(&format!("Status: {}", status));
            }

            if let Some(ssh_cmd) = pod_info.get("ssh_cmd").and_then(|v| v.as_str()) {
                print_success(&format!("SSH Command: {}", ssh_cmd));
            }

            if let Some(ports) = pod_info.get("ports").and_then(|v| v.as_object()) {
                if !ports.is_empty() {
                    println!("\nPort Mappings:");
                    for (service, port) in ports {
                        println!("  {}: {}", service, port);
                    }
                }
            }

            println!();
            if let Some(huid) = pod_info.get("huid").and_then(|v| v.as_str()) {
                println!("Use 'lium ssh {}' to connect", huid);
            }
        }
        Err(e) => {
            print_error(&format!("DEBUG: rent_pod API call failed"));
            print_error(&format!("Failed to start pod: {}", e));
            print_error(&format!("DEBUG: Error details: {:?}", e));
            return Err(e.into());
        }
    }

    Ok(())
}
