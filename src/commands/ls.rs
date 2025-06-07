use crate::api::LiumApiClient;
use crate::config::Config;
use crate::display::{display_executors_table, display_gpu_summary};
use crate::errors::Result;
use crate::utils::{
    filter_by_availability, filter_by_gpu_type, filter_by_price_range, find_pareto_optimal,
    group_by_gpu_type, parse_price_range, sort_by_gpu_count, sort_by_price,
};
use clap::Args;

#[derive(Args)]
pub struct LsArgs {
    /// Filter by GPU type (e.g., RTX4090, H100)
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
}

pub async fn handle_ls(args: LsArgs, _config: &Config) -> Result<()> {
    let client = LiumApiClient::from_config()?;

    // Fetch executors from API
    let mut executors = client.get_executors().await?;

    if executors.is_empty() {
        println!("No executors found.");
        return Ok(());
    }

    // Apply filters
    if let Some(gpu_type) = &args.gpu {
        executors = filter_by_gpu_type(&executors, gpu_type);
    }

    if let Some(price_range) = &args.price {
        let (min_price, max_price) = parse_price_range(price_range)?;
        executors = filter_by_price_range(&executors, min_price, max_price);
    }

    if args.available {
        executors = filter_by_availability(&executors, true);
    }

    // Find Pareto optimal if requested
    if args.pareto {
        executors = find_pareto_optimal(&executors);
    }

    // Apply sorting
    if args.sort_price {
        sort_by_price(&mut executors);
    } else if args.sort_gpu {
        sort_by_gpu_count(&mut executors);
    } else {
        // Default sort by price
        sort_by_price(&mut executors);
    }

    // Apply limit
    if let Some(limit) = args.limit {
        executors.truncate(limit);
    }

    // Display results
    if args.summary {
        let gpu_groups = group_by_gpu_type(&executors);
        display_gpu_summary(&gpu_groups);
    } else {
        display_executors_table(&executors, args.pareto);
    }

    // Show filter summary
    if args.gpu.is_some() || args.price.is_some() || args.available || args.pareto {
        println!();
        let mut filters = Vec::new();

        if let Some(gpu) = &args.gpu {
            filters.push(format!("GPU: {}", gpu));
        }
        if let Some(price) = &args.price {
            filters.push(format!("Price: ${}/GPU/hr", price));
        }
        if args.available {
            filters.push("Available only".to_string());
        }
        if args.pareto {
            filters.push("Pareto optimal".to_string());
        }

        println!("Filters applied: {}", filters.join(", "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn create_test_executor(
        huid: &str,
        gpu_type: &str,
        price: f64,
        available: bool,
    ) -> ExecutorInfo {
        ExecutorInfo {
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
}

// TODO: Add more sophisticated filtering options (location, specs)
// TODO: Add caching for executor data
// TODO: Add export options (JSON, CSV)
// TODO: Add real-time updates for availability
// TODO: Add cost estimation features
