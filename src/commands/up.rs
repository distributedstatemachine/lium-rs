use crate::api::LiumApiClient;
use crate::config::Config;
use crate::display::{
    display_executors_table, print_error, print_info, print_success, prompt_confirm, prompt_select,
};
use crate::errors::{LiumError, Result};
use crate::utils::{
    filter_by_availability, filter_by_gpu_type, parse_env_vars, parse_executor_index,
    parse_port_mappings, sort_by_price, validate_docker_image,
};
use clap::Args;
use std::collections::HashMap;

#[derive(Args)]
pub struct UpArgs {
    /// Docker image to run
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

pub async fn handle_up(args: UpArgs, config: &Config) -> Result<()> {
    let client = LiumApiClient::from_config()?;

    // Get Docker image
    let docker_image = match args.image {
        Some(image) => {
            validate_docker_image(&image)?;
            image
        }
        None => {
            return Err(LiumError::InvalidInput(
                "Docker image is required. Use --image or -i flag.".to_string(),
            ));
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
    let mut executors = client.get_executors().await?;

    if executors.is_empty() {
        return Err(LiumError::OperationFailed("No executors found".to_string()));
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
        return Err(LiumError::OperationFailed(
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
    print_info(&format!("Docker Image: {}", docker_image));

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
    let template_id = docker_image;
    let ssh_keys = config.get_ssh_public_keys().unwrap_or_default();

    match client
        .rent_pod(&executor_id, &pod_name, &template_id, ssh_keys)
        .await
    {
        Ok(pod_info) => {
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
            print_error(&format!("Failed to start pod: {}", e));
            return Err(e);
        }
    }

    Ok(())
}
