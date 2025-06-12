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

#[derive(Args)]
pub struct UpArgs {
    /// Template ID to use (replaces --image)
    #[arg(short, long)]
    pub image: Option<String>,

    /// Filter by GPU type (e.g., RTX4090, H100)
    #[arg(short, long)]
    pub gpu: Option<String>,

    /// Show only available executors
    #[arg(short, long)]
    pub available: bool,

    /// Select executor by index (1-based)
    #[arg(short, long)]
    pub index: Option<String>,

    /// Environment variables (KEY=VALUE,KEY2=VALUE2)
    #[arg(short, long)]
    pub env: Option<String>,

    /// Port mappings (8080:80,9000:9000)
    #[arg(short, long)]
    pub ports: Option<String>,

    /// SSH public key path
    #[arg(long)]
    pub ssh_key: Option<String>,

    /// Pod name (optional)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,
}

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
