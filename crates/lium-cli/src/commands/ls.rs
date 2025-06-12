use crate::{
    config::Config,
    display::{
        display_executors_compact, display_executors_detailed, display_executors_table,
        display_gpu_summary,
    },
    CliError, Result,
};
use clap::Args;
use lium_api::LiumApiClient;
use lium_core::{
    filter_by_availability, filter_by_gpu_type, filter_by_price_range, find_pareto_optimal,
    group_by_gpu_type, parse_price_range, sort_by_gpu_count, sort_by_price,
};
use log::debug;

#[derive(Args)]
pub struct LsArgs {
    /// GPU type to filter by (e.g., RTX4090, H100) - can also use --gpu flag
    #[arg(value_name = "GPU_TYPE")]
    pub gpu_type: Option<String>,

    /// Filter by GPU type (e.g., RTX4090, H100) - alternative to positional argument
    #[arg(short, long)]
    pub gpu: Option<String>,

    /// Filter by price range per GPU per hour (e.g., "0.5-2.0")
    #[arg(short, long)]
    pub price: Option<String>,

    /// Show only available executors
    #[arg(short, long)]
    pub available: bool,

    /// Sort by price (ascending)
    #[arg(long)]
    pub sort_price: bool,

    /// Sort by GPU count (descending)
    #[arg(long)]
    pub sort_gpu: bool,

    /// Show GPU type summary instead of detailed list
    #[arg(long)]
    pub summary: bool,

    /// Show only Pareto optimal executors (best price/performance)
    #[arg(long)]
    pub pareto: bool,

    /// Limit number of results
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Display format: table (default), compact, or detailed
    #[arg(long, value_enum, default_value = "table")]
    pub format: DisplayFormat,

    /// Sort by different criteria
    #[arg(long, value_enum)]
    pub sort: Option<SortBy>,

    /// Show only executors from specific locations
    #[arg(long)]
    pub location: Option<String>,

    /// Minimum RAM in GB
    #[arg(long)]
    pub min_ram: Option<f64>,

    /// Export results to file (json, csv)
    #[arg(long)]
    pub export: Option<String>,

    /// Reverse sort order
    #[arg(long)]
    pub reverse: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DisplayFormat {
    Table,
    Compact,
    Detailed,
    Summary,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum SortBy {
    Price,
    GpuCount,
    Ram,
    Location,
    Status,
}

pub async fn handle(args: LsArgs, config: &Config) -> Result<()> {
    let client = LiumApiClient::from_config(config)?;

    // Fetch executors from API with better error handling
    debug!("Fetching executors from API");
    let mut executors = match client.get_executors().await {
        Ok(execs) => execs,
        Err(e) => {
            return Err(CliError::Api(e));
        }
    };

    if executors.is_empty() {
        println!("No executors found.");
        return Ok(());
    }

    debug!("Successfully fetched {} executors", executors.len());

    // Apply filters
    // Determine GPU filter - positional argument takes precedence over --gpu flag
    let gpu_filter = args.gpu_type.as_ref().or(args.gpu.as_ref());

    if let Some(gpu_type) = gpu_filter {
        debug!("Filtering by GPU type: {}", gpu_type);
        executors = filter_by_gpu_type(&executors, gpu_type);
        debug!("After GPU filter: {} executors", executors.len());
    }

    if let Some(price_range) = &args.price {
        debug!("Parsing price range: {}", price_range);
        let (min_price, max_price) = parse_price_range(price_range).map_err(|e| {
            CliError::InvalidInput(format!(
                "Invalid price range '{}'. Use format like '0.5-2.0': {}",
                price_range, e
            ))
        })?;
        debug!(
            "Filtering by price range: ${:.2} - ${:.2}",
            min_price, max_price
        );
        executors = filter_by_price_range(&executors, min_price, max_price);
        debug!("After price filter: {} executors", executors.len());
    }

    if args.available {
        debug!("Filtering for available executors only");
        executors = filter_by_availability(&executors, true);
        debug!("After availability filter: {} executors", executors.len());
    }

    if let Some(location) = &args.location {
        debug!("Filtering by location: {}", location);
        executors = filter_by_location(&executors, location);
        debug!("After location filter: {} executors", executors.len());
    }

    if let Some(min_ram) = args.min_ram {
        debug!("Filtering by minimum RAM: {} GB", min_ram);
        executors = filter_by_min_ram(&executors, min_ram);
        debug!("After RAM filter: {} executors", executors.len());
    }

    // Find Pareto optimal if requested
    if args.pareto {
        debug!("Finding Pareto optimal executors");
        executors = find_pareto_optimal(&executors);
        debug!("After Pareto filter: {} executors", executors.len());
    }

    // Apply sorting
    apply_sorting(&mut executors, &args);

    // Apply limit
    if let Some(limit) = args.limit {
        debug!("Limiting results to {} executors", limit);
        executors.truncate(limit);
    }

    // Export if requested
    if let Some(export_path) = &args.export {
        export_results(&executors, export_path)?;
        println!("Results exported to: {}", export_path);
    }

    // Display results based on format
    match args.format {
        DisplayFormat::Table => {
            display_executors_table(&executors, args.pareto);
        }
        DisplayFormat::Compact => {
            display_executors_compact(&executors);
        }
        DisplayFormat::Detailed => {
            display_executors_detailed(&executors);
        }
        DisplayFormat::Summary => {
            let gpu_groups = group_by_gpu_type(&executors);
            display_gpu_summary(&gpu_groups);
        }
    }

    // Show filter summary
    show_filter_summary(&args);

    Ok(())
}

fn apply_sorting(executors: &mut Vec<lium_core::ExecutorInfo>, args: &LsArgs) {
    // Determine sort criteria
    let sort_by = if let Some(sort_by) = &args.sort {
        sort_by
    } else if args.sort_price {
        &SortBy::Price
    } else if args.sort_gpu {
        &SortBy::GpuCount
    } else {
        &SortBy::Price // Default
    };

    debug!("Sorting by: {:?}", sort_by);

    match sort_by {
        SortBy::Price => {
            sort_by_price(executors);
        }
        SortBy::GpuCount => {
            sort_by_gpu_count(executors);
        }
        SortBy::Ram => {
            executors.sort_by(|a, b| {
                let ram_a = extract_ram_from_specs(&a.specs);
                let ram_b = extract_ram_from_specs(&b.specs);
                ram_a
                    .partial_cmp(&ram_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::Location => {
            executors.sort_by(|a, b| {
                let loc_a = extract_location_for_sort(&a.location);
                let loc_b = extract_location_for_sort(&b.location);
                loc_a.cmp(&loc_b)
            });
        }
        SortBy::Status => {
            executors.sort_by(|a, b| {
                // Available first, then by status
                match (a.available, b.available) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.status.cmp(&b.status),
                }
            });
        }
    }

    if args.reverse {
        debug!("Reversing sort order");
        executors.reverse();
    }
}

fn filter_by_location(
    executors: &[lium_core::ExecutorInfo],
    location: &str,
) -> Vec<lium_core::ExecutorInfo> {
    let location_lower = location.to_lowercase();
    executors
        .iter()
        .filter(|executor| {
            executor
                .location
                .values()
                .any(|loc| loc.to_lowercase().contains(&location_lower))
        })
        .cloned()
        .collect()
}

fn filter_by_min_ram(
    executors: &[lium_core::ExecutorInfo],
    min_ram: f64,
) -> Vec<lium_core::ExecutorInfo> {
    executors
        .iter()
        .filter(|executor| {
            let ram = extract_ram_from_specs(&executor.specs);
            ram >= min_ram
        })
        .cloned()
        .collect()
}

fn extract_ram_from_specs(specs: &serde_json::Value) -> f64 {
    specs
        .get("memory_gb")
        .or_else(|| specs.get("ram_gb"))
        .or_else(|| specs.get("memory"))
        .and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or(0.0)
}

fn extract_location_for_sort(location: &std::collections::HashMap<String, String>) -> String {
    location
        .get("region")
        .or_else(|| location.get("country"))
        .or_else(|| location.get("state"))
        .or_else(|| location.values().next())
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string())
}

fn export_results(executors: &[lium_core::ExecutorInfo], export_path: &str) -> Result<()> {
    use std::fs;

    if export_path.ends_with(".json") {
        let json = serde_json::to_string_pretty(executors)
            .map_err(|e| CliError::OperationFailed(format!("JSON serialization failed: {}", e)))?;
        fs::write(export_path, json)
            .map_err(|e| CliError::OperationFailed(format!("Failed to write file: {}", e)))?;
    } else if export_path.ends_with(".csv") {
        let mut csv_content = String::new();
        csv_content.push_str("HUID,GPU Type,GPU Count,Price per GPU per Hour,Price per Hour,RAM GB,Location,Status,Available\n");

        for executor in executors {
            let ram = extract_ram_from_specs(&executor.specs);
            let location = extract_location_for_sort(&executor.location);
            csv_content.push_str(&format!(
                "{},{},{},{:.3},{:.3},{:.0},{},{},{}\n",
                executor.huid,
                executor.gpu_type,
                executor.gpu_count,
                executor.price_per_gpu_hour,
                executor.price_per_hour,
                ram,
                location,
                executor.status,
                executor.available
            ));
        }

        fs::write(export_path, csv_content)
            .map_err(|e| CliError::OperationFailed(format!("Failed to write CSV file: {}", e)))?;
    } else {
        return Err(CliError::InvalidInput(
            "Export format must be .json or .csv".to_string(),
        ));
    }

    Ok(())
}

fn show_filter_summary(args: &LsArgs) {
    let mut filters = Vec::new();

    // Show GPU filter (from either positional or flag)
    let gpu_filter = args.gpu_type.as_ref().or(args.gpu.as_ref());
    if let Some(gpu) = gpu_filter {
        filters.push(format!("GPU: {}", gpu));
    }

    if let Some(price) = &args.price {
        filters.push(format!("Price: ${}/GPU/hr", price));
    }
    if args.available {
        filters.push("Available only".to_string());
    }
    if let Some(location) = &args.location {
        filters.push(format!("Location: {}", location));
    }
    if let Some(min_ram) = args.min_ram {
        filters.push(format!("Min RAM: {} GB", min_ram));
    }
    if args.pareto {
        filters.push("Pareto optimal".to_string());
    }
    if let Some(limit) = args.limit {
        filters.push(format!("Limit: {}", limit));
    }

    if !filters.is_empty() {
        println!();
        println!("📋 Filters applied: {}", filters.join(", "));
    }

    // Show sort info
    let sort_info = if let Some(sort_by) = &args.sort {
        format!(
            "Sort: {:?}{}",
            sort_by,
            if args.reverse { " (reversed)" } else { "" }
        )
    } else if args.sort_price {
        format!(
            "Sort: Price{}",
            if args.reverse { " (reversed)" } else { "" }
        )
    } else if args.sort_gpu {
        format!(
            "Sort: GPU Count{}",
            if args.reverse { " (reversed)" } else { "" }
        )
    } else {
        "Sort: Price (default)".to_string()
    };

    if args.sort.is_some() || args.sort_price || args.sort_gpu || args.reverse {
        println!("🔀 {}", sort_info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_executor(
        huid: &str,
        gpu_type: &str,
        price: f64,
        available: bool,
    ) -> lium_core::ExecutorInfo {
        lium_core::ExecutorInfo {
            id: format!("exec_{}", huid),
            huid: huid.to_string(),
            machine_name: format!("machine-{}-{}", gpu_type.to_lowercase(), huid),
            gpu_type: gpu_type.to_string(),
            gpu_count: 1,
            price_per_gpu_hour: price,
            price_per_hour: price,
            available,
            status: if available {
                "available".to_string()
            } else {
                "rented".to_string()
            },
            location: HashMap::new(),
            specs: serde_json::json!({
                "cpu_cores": 8,
                "memory_gb": 32,
                "storage_gb": 500
            }),
        }
    }

    #[test]
    fn test_filter_by_gpu_type() {
        let executors = vec![
            create_test_executor("1", "RTX4090", 1.0, true),
            create_test_executor("2", "H100", 2.0, true),
            create_test_executor("3", "RTX4090", 1.5, false),
        ];

        let filtered = filter_by_gpu_type(&executors, "RTX4090");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.gpu_type.contains("RTX4090")));
    }

    #[test]
    fn test_filter_by_availability() {
        let executors = vec![
            create_test_executor("1", "RTX4090", 1.0, true),
            create_test_executor("2", "H100", 2.0, false),
            create_test_executor("3", "RTX4090", 1.5, true),
        ];

        let filtered = filter_by_availability(&executors, true);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.available));
    }

    #[test]
    fn test_filter_by_min_ram() {
        let executors = vec![
            create_test_executor("1", "RTX4090", 1.0, true),
            create_test_executor("2", "H100", 2.0, true),
            create_test_executor("3", "RTX4090", 1.5, false),
        ];

        let filtered = filter_by_min_ram(&executors, 30.0);
        assert_eq!(filtered.len(), 3); // All have 32GB RAM

        let filtered = filter_by_min_ram(&executors, 40.0);
        assert_eq!(filtered.len(), 0); // None have 40GB+ RAM
    }

    #[test]
    fn test_export_csv() {
        let executors = vec![create_test_executor("1", "RTX4090", 1.0, true)];
        let temp_file = "/tmp/test_export.csv";

        export_results(&executors, temp_file).expect("Export should succeed");

        let content = std::fs::read_to_string(temp_file).expect("File should exist");
        assert!(content.contains("HUID,GPU Type"));
        assert!(content.contains("exec-1,RTX4090"));

        // Cleanup
        std::fs::remove_file(temp_file).ok();
    }
}

// TODO: Add real-time updates for availability
// TODO: Add cost estimation features
// TODO: Add favorites/bookmarking system
// TODO: Add notification system for price changes
// TODO: Add integration with external monitoring tools
