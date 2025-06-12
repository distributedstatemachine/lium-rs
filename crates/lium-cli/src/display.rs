use crate::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use lium_core::{ExecutorInfo, PodInfo, TemplateInfo};
use std::collections::HashMap;

/// A utility struct for formatting and displaying tabular data in the terminal.
/// 
/// The Table struct provides functionality to create, populate, and display
/// formatted tables with borders, headers, and rows. It automatically handles
/// column width calculations and alignment.
/// 
/// # Fields
/// * `headers` - Vector of column header strings
/// * `rows` - Vector of row data, where each row is a vector of cell strings
/// * `max_widths` - Vector tracking the maximum width needed for each column
/// 
/// # Examples
/// ```rust
/// let mut table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
/// table.add_row(vec!["John".to_string(), "30".to_string()]);
/// table.print();
/// ```
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    max_widths: Vec<usize>,
}

impl Table {
    /// Creates a new Table instance with the specified headers.
    /// 
    /// Initializes the table with the given headers and calculates initial
    /// column widths based on header lengths.
    /// 
    /// # Arguments
    /// * `headers` - Vector of strings representing column headers
    /// 
    /// # Returns
    /// * `Table` - A new Table instance
    /// 
    /// # Examples
    /// ```rust
    /// let table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// ```
    pub fn new(headers: Vec<String>) -> Self {
        let max_widths = headers.iter().map(|h| h.len()).collect();
        Self {
            headers,
            rows: Vec::new(),
            max_widths,
        }
    }

    /// Adds a new row to the table and updates column widths if necessary.
    /// 
    /// This method adds a row of data to the table and automatically adjusts
    /// column widths if the new row contains cells wider than the current
    /// maximum widths.
    /// 
    /// # Arguments
    /// * `row` - Vector of strings representing the row data
    /// 
    /// # Panics
    /// This method will not panic if the row has fewer columns than headers,
    /// but will ignore any extra columns beyond the number of headers.
    /// 
    /// # Examples
    /// ```rust
    /// let mut table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// table.add_row(vec!["John".to_string(), "30".to_string()]);
    /// ```
    pub fn add_row(&mut self, row: Vec<String>) {
        // Update max widths for each column based on the new row
        for (i, cell) in row.iter().enumerate() {
            if i < self.max_widths.len() {
                self.max_widths[i] = self.max_widths[i].max(cell.len());
            }
        }
        self.rows.push(row);
    }

    /// Prints the complete table to stdout with borders and formatting.
    /// 
    /// This method displays the table with:
    /// - A top border
    /// - Formatted headers
    /// - A separator line
    /// - All data rows
    /// - A bottom border
    /// 
    /// The output is formatted with proper spacing and alignment based on
    /// the calculated column widths.
    /// 
    /// # Examples
    /// ```rust
    /// let mut table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// table.add_row(vec!["John".to_string(), "30".to_string()]);
    /// table.print();
    /// ```
    pub fn print(&self) {
        self.print_top_border();
        self.print_header();
        self.print_middle_border();

        for row in &self.rows {
            self.print_row(row);
        }

        self.print_bottom_border();
    }

    /// Prints the top border of the table using box-drawing characters.
    /// 
    /// This method creates a border that looks like:
    /// ┌────┬────┬────┐
    /// 
    /// The width of each segment is determined by the maximum width of the
    /// corresponding column plus 2 spaces for padding.
    /// 
    /// # Examples
    /// ```rust
    /// let table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// table.print_top_border(); // Prints: ┌────┬────┐
    /// ```
    fn print_top_border(&self) {
        print!("┌");
        for (i, &width) in self.max_widths.iter().enumerate() {
            print!("{}", "─".repeat(width + 2));
            if i < self.max_widths.len() - 1 {
                print!("┬");
            }
        }
        println!("┐");
    }

    /// Prints the middle border of the table using box-drawing characters.
    /// 
    /// This method creates a border that looks like:
    /// ├────┼────┼────┤
    /// 
    /// The width of each segment is determined by the maximum width of the
    /// corresponding column plus 2 spaces for padding.
    /// 
    /// # Examples
    /// ```rust
    /// let table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// table.print_middle_border(); // Prints: ├────┼────┤
    /// ```
    fn print_middle_border(&self) {
        print!("├");
        for (i, &width) in self.max_widths.iter().enumerate() {
            print!("{}", "─".repeat(width + 2));
            if i < self.max_widths.len() - 1 {
                print!("┼");
            }
        }
        println!("┤");
    }

    /// Prints the bottom border of the table using box-drawing characters.
    /// 
    /// This method creates a border that looks like:
    /// └────┴────┴────┘
    /// 
    /// The width of each segment is determined by the maximum width of the
    /// corresponding column plus 2 spaces for padding.
    /// 
    /// # Examples
    /// ```rust
    /// let table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// table.print_bottom_border(); // Prints: └────┴────┘
    /// ```
    fn print_bottom_border(&self) {
        print!("└");
        for (i, &width) in self.max_widths.iter().enumerate() {
            print!("{}", "─".repeat(width + 2));
            if i < self.max_widths.len() - 1 {
                print!("┴");
            }
        }
        println!("┘");
    }

    /// Prints the header row of the table with bold formatting.
    /// 
    /// This method formats the header row with:
    /// - Bold text for each header
    /// - Left-aligned text within each column
    /// - Proper spacing based on column widths
    /// - Vertical borders between columns
    /// 
    /// # Examples
    /// ```rust
    /// let table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// table.print_header(); // Prints: │ Name │ Age │
    /// ```
    fn print_header(&self) {
        print!("│");
        for (i, header) in self.headers.iter().enumerate() {
            print!(" {:<width$} ", header.bold(), width = self.max_widths[i]);
            print!("│");
        }
        println!();
    }

    /// Prints a data row of the table with proper formatting.
    /// 
    /// This method formats each row with:
    /// - Left-aligned text within each column
    /// - Proper spacing based on column widths
    /// - Vertical borders between columns
    /// - Handles rows with fewer columns than headers by using width 0
    /// 
    /// # Arguments
    /// * `row` - A slice of strings representing the row data
    /// 
    /// # Examples
    /// ```rust
    /// let mut table = Table::new(vec!["Name".to_string(), "Age".to_string()]);
    /// table.print_row(&["John".to_string(), "30".to_string()]); // Prints: │ John │ 30 │
    /// ```
    fn print_row(&self, row: &[String]) {
        print!("│");
        for (i, cell) in row.iter().enumerate() {
            let width = if i < self.max_widths.len() {
                self.max_widths[i]
            } else {
                0
            };
            print!(" {:<width$} ", cell, width = width);
            print!("│");
        }
        println!();
    }
}

/// Displays a formatted table of executor information with detailed pricing and availability data.
///
/// This function creates a comprehensive table showing executor details including:
/// - Index number for easy reference
/// - Hardware Unique Identifier (HUID)
/// - GPU specifications and count
/// - Pricing information (per GPU and total)
/// - System resources (RAM)
/// - Geographic location
/// - Current availability status
///
/// The function also provides summary statistics including:
/// - Total number of executors
/// - Number of available executors
/// - Price range across all executors
/// - Pareto optimality indicator (if enabled)
///
/// # Arguments
/// * `executors` - A slice of `ExecutorInfo` structs containing executor details
/// * `show_pareto` - Boolean flag indicating whether to show Pareto optimality message
///
/// # Examples
/// ```rust
/// let executors = vec![ExecutorInfo::default()];
/// display_executors_table(&executors, true);
/// ```
pub fn display_executors_table(executors: &[ExecutorInfo], show_pareto: bool) {
    // Early return with warning if no executors are available
    if executors.is_empty() {
        println!("{}", "No executors found.".yellow());
        return;
    }

    // Initialize table with column headers
    let mut table = Table::new(vec![
        "Index".to_string(),
        "HUID".to_string(),
        "GPU Type".to_string(),
        "Count".to_string(),
        "$/GPU/hr".to_string(),
        "$/hr".to_string(),
        "RAM (GB)".to_string(),
        "Location".to_string(),
        "Status".to_string(),
    ]);

    // Process each executor and add to table
    for (i, executor) in executors.iter().enumerate() {
        let index = (i + 1).to_string();
        let gpu_count = executor.gpu_count.to_string();
        let price_gpu = format!("{:.3}", executor.price_per_gpu_hour);
        let price_total = format!("{:.3}", executor.price_per_hour);

        // Extract RAM information with fallback options
        let ram = executor
            .specs
            .get("memory_gb")
            .or_else(|| executor.specs.get("ram_gb"))
            .or_else(|| executor.specs.get("memory"))
            .and_then(|v| {
                match v {
                    serde_json::Value::Number(n) => n.as_f64().map(|f| format!("{:.0}", f)),
                    serde_json::Value::String(s) => s.parse::<f64>().ok().map(|f| format!("{:.0}", f)),
                    _ => None,
                }
            })
            .unwrap_or_else(|| "N/A".to_string());

        // Extract location information with hierarchical fallback
        let location = executor
            .location
            .get("region")
            .or_else(|| executor.location.get("country"))
            .or_else(|| executor.location.get("state"))
            .or_else(|| executor.location.get("city"))
            .or_else(|| executor.location.get("datacenter"))
            .cloned()
            .unwrap_or_else(|| {
                executor
                    .location
                    .values()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string())
            });

        // Determine availability status
        let status = if executor.available {
            "Available".to_string()
        } else {
            "Rented".to_string()
        };

        // Add formatted row to table
        table.add_row(vec![
            index,
            executor.huid.clone(),
            executor.gpu_type.clone(),
            gpu_count,
            price_gpu,
            price_total,
            ram,
            location,
            status,
        ]);
    }

    // Display the formatted table
    table.print();

    // Calculate and display summary statistics
    let total_executors = executors.len();
    let available_count = executors.iter().filter(|e| e.available).count();
    let price_range = if !executors.is_empty() {
        let prices: Vec<f64> = executors.iter().map(|e| e.price_per_gpu_hour).collect();
        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        format!(
            " • Price range: ${:.3} - ${:.3}/GPU/hr",
            min_price, max_price
        )
    } else {
        String::new()
    };

    // Print summary information
    println!();
    println!(
        "📊 {} total executors • {} available{}",
        total_executors, available_count, price_range
    );

    // Show Pareto optimality message if enabled
    if show_pareto {
        println!(
            "{}",
            "✓ Showing Pareto optimal executors (best price/performance)".green()
        );
    }
}

/// Displays a summary table of GPU types with aggregated statistics.
///
/// This function creates a comprehensive summary table showing:
/// - GPU type distribution
/// - Total and available counts
/// - Price statistics (min, max, average)
/// - Availability percentages
///
/// The table is sorted alphabetically by GPU type for easy reference.
///
/// # Arguments
/// * `gpu_types` - A HashMap mapping GPU type names to vectors of `ExecutorInfo`
///
/// # Examples
/// ```rust
/// let mut gpu_types = HashMap::new();
/// gpu_types.insert("RTX 3090".to_string(), vec![ExecutorInfo::default()]);
/// display_gpu_summary(&gpu_types);
/// ```
pub fn display_gpu_summary(gpu_types: &HashMap<String, Vec<ExecutorInfo>>) {
    // Early return with warning if no GPU types are available
    if gpu_types.is_empty() {
        println!("{}", "No GPU types found.".yellow());
        return;
    }

    // Print header
    println!("{}", "GPU Type Summary".bold().blue());
    println!();

    // Initialize summary table
    let mut table = Table::new(vec![
        "GPU Type".to_string(),
        "Total".to_string(),
        "Available".to_string(),
        "Min $/GPU/hr".to_string(),
        "Max $/GPU/hr".to_string(),
        "Avg $/GPU/hr".to_string(),
    ]);

    // Sort GPU types alphabetically
    let mut gpu_types_vec: Vec<_> = gpu_types.iter().collect();
    gpu_types_vec.sort_by(|a, b| a.0.cmp(b.0));

    // Process each GPU type and calculate statistics
    for (gpu_type, executors) in gpu_types_vec {
        let total_count = executors.len();
        let available_count = executors.iter().filter(|e| e.available).count();

        // Calculate price statistics
        let prices: Vec<f64> = executors.iter().map(|e| e.price_per_gpu_hour).collect();
        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg_price = prices.iter().sum::<f64>() / prices.len() as f64;

        // Add formatted row to table
        table.add_row(vec![
            gpu_type.clone(),
            total_count.to_string(),
            format!(
                "{} ({}%)",
                available_count,
                (available_count * 100) / total_count
            ),
            format!("{:.3}", min_price),
            format!("{:.3}", max_price),
            format!("{:.3}", avg_price),
        ]);
    }

    // Display the formatted table
    table.print();
}

/// Displays a formatted table of pod information in the terminal.
///
/// This function takes a slice of `PodInfo` structs and displays them in a well-formatted
/// table with the following columns:
/// - Index: Sequential number of the pod in the list
/// - Pod HUID: Unique identifier for the pod
/// - Name: User-defined name of the pod
/// - Status: Current state of the pod (running, starting, stopped, etc.)
/// - GPU Type: Model of GPU(s) allocated to the pod
/// - Count: Number of GPUs allocated
/// - Uptime: Time since pod creation in human-readable format
/// - SSH Command: Command to connect to the pod via SSH
///
/// The function handles various edge cases:
/// - Empty pod list: Displays a yellow warning message
/// - Missing GPU information: Falls back to machine name or "Unknown"
/// - Missing timing information: Shows "Unknown" for uptime
/// - Missing SSH command: Shows "N/A"
///
/// # Arguments
/// * `pods` - A slice of `PodInfo` structs containing pod information
///
/// # Examples
/// ```rust
/// let pods = vec![pod1, pod2, pod3];
/// display_pods_table(&pods);
/// ```
///
/// # Notes
/// - GPU type extraction attempts to identify common GPU models (H100, A100, etc.)
/// - Uptime is calculated from either creation timestamp or uptime_in_minutes
/// - Table formatting is handled by the `Table` struct
pub fn display_pods_table(pods: &[PodInfo]) {
    // Handle empty pod list
    if pods.is_empty() {
        println!("{}", "No active pods found.".yellow());
        return;
    }

    // Print table header
    println!("{}", "Active Pods".bold().blue());
    println!();

    // Initialize table with column headers
    let mut table = Table::new(vec![
        "Index".to_string(),
        "Pod HUID".to_string(),
        "Name".to_string(),
        "Status".to_string(),
        "GPU Type".to_string(),
        "Count".to_string(),
        "Uptime".to_string(),
        "SSH Command".to_string(),
    ]);

    // Process each pod and add to table
    for (i, pod) in pods.iter().enumerate() {
        let index = (i + 1).to_string();

        // Extract GPU information with fallbacks
        let (gpu_type, gpu_count) = extract_gpu_info(pod);

        // Calculate uptime with fallbacks
        let uptime = calculate_uptime(pod);

        // Get status and SSH command
        let status = pod.status.clone();
        let ssh_cmd = pod
            .ssh_cmd
            .as_ref()
            .map(|cmd| cmd.clone())
            .unwrap_or_else(|| "N/A".to_string());

        // Add row to table
        table.add_row(vec![
            index,
            pod.huid.clone(),
            pod.name.clone(),
            status,
            gpu_type,
            gpu_count,
            uptime,
            ssh_cmd,
        ]);
    }

    // Display the formatted table
    table.print();
}

/// Extracts GPU type and count information from a pod.
///
/// This helper function attempts to extract GPU information from the pod's executor
/// configuration, with multiple fallback options if the primary source is unavailable.
///
/// # Arguments
/// * `pod` - Reference to a `PodInfo` struct
///
/// # Returns
/// * `(String, String)` - Tuple containing GPU type and count
fn extract_gpu_info(pod: &PodInfo) -> (String, String) {
    if let Some(specs) = pod.executor.get("specs") {
        if let Some(gpu) = specs.get("gpu") {
            // Extract GPU count
            let count = gpu
                .get("count")
                .and_then(|v| v.as_i64())
                .unwrap_or(1)
                .to_string();

            // Extract GPU name with fallback
            let gpu_name = gpu
                .get("details")
                .and_then(|details| details.as_array())
                .and_then(|arr| arr.first())
                .and_then(|detail| detail.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    pod.executor
                        .get("machine_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                });

            (extract_gpu_model(gpu_name).to_string(), count)
        } else {
            ("Unknown".to_string(), "1".to_string())
        }
    } else {
        // Fallback to machine_name
        let gpu_type = pod
            .executor
            .get("machine_name")
            .and_then(|v| v.as_str())
            .map(|name| extract_gpu_model(name).to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        (gpu_type, "1".to_string())
    }
}

/// Calculates the uptime of a pod in a human-readable format.
///
/// This function attempts to calculate uptime from multiple sources:
/// 1. Pod creation timestamp
/// 2. Uptime in minutes from executor
/// 3. Falls back to "Unknown" if no timing information is available
///
/// # Arguments
/// * `pod` - Reference to a `PodInfo` struct
///
/// # Returns
/// * `String` - Formatted uptime string
fn calculate_uptime(pod: &PodInfo) -> String {
    if let Some(created_at) = pod.created_at {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(created_at);

        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        let minutes = duration.num_minutes() % 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    } else if let Some(uptime_minutes) = pod
        .executor
        .get("uptime_in_minutes")
        .and_then(|v| v.as_f64())
    {
        let hours = uptime_minutes / 60.0;
        if hours >= 24.0 {
            let days = (hours / 24.0) as i64;
            let remaining_hours = (hours % 24.0) as i64;
            format!("{}d {}h", days, remaining_hours)
        } else if hours >= 1.0 {
            format!("{:.1}h", hours)
        } else {
            format!("{:.0}m", uptime_minutes)
        }
    } else {
        "Unknown".to_string()
    }
}

/// Extracts a standardized GPU model name from a full GPU name string.
///
/// This function attempts to identify common GPU models from various naming formats.
/// It handles both NVIDIA and other GPU manufacturers' naming conventions.
/// The function uses a hierarchical matching approach, checking for specific models
/// before falling back to a generic pattern matching strategy.
///
/// # Arguments
/// * `gpu_name` - A string slice containing the full GPU name to parse
///
/// # Returns
/// * `&str` - A standardized GPU model name or "GPU" if no match is found
///
/// # Examples
/// ```rust
/// assert_eq!(extract_gpu_model("NVIDIA H100 SXM5"), "H100");
/// assert_eq!(extract_gpu_model("RTX 4090"), "RTX4090");
/// assert_eq!(extract_gpu_model("Unknown GPU 123"), "123");
/// ```
///
/// # Notes
/// - Handles common NVIDIA models: H100, A100, RTX series, V100, A6000, T4, L4, L40
/// - Case-sensitive matching
/// - Falls back to finding any word containing numbers if no specific model is found
/// - Returns "GPU" as a last resort if no model can be identified
fn extract_gpu_model(gpu_name: &str) -> &str {
    // TODO: Consider adding support for more GPU models
    // TODO: Consider case-insensitive matching
    // TODO: Consider adding memory size detection (e.g., "H100 80GB")
    
    if gpu_name.contains("H100") {
        "H100"
    } else if gpu_name.contains("A100") {
        "A100"
    } else if gpu_name.contains("RTX 4090") || gpu_name.contains("RTX4090") {
        "RTX4090"
    } else if gpu_name.contains("RTX 3090") || gpu_name.contains("RTX3090") {
        "RTX3090"
    } else if gpu_name.contains("RTX 3080") || gpu_name.contains("RTX3080") {
        "RTX3080"
    } else if gpu_name.contains("V100") {
        "V100"
    } else if gpu_name.contains("A6000") {
        "A6000"
    } else if gpu_name.contains("T4") {
        "T4"
    } else if gpu_name.contains("L4") {
        "L4"
    } else if gpu_name.contains("L40") {
        "L40"
    } else {
        // Fallback: Try to find any word containing numbers
        gpu_name
            .split_whitespace()
            .find(|word| word.chars().any(|c| c.is_numeric()))
            .unwrap_or("GPU")
    }
}

/// Displays detailed information about a pod in a formatted, human-readable way.
///
/// This function presents pod information in a hierarchical, easy-to-read format
/// with color-coded status indicators and organized sections for different types
/// of information. It handles optional fields gracefully and formats timestamps
/// in a consistent UTC format.
///
/// # Arguments
/// * `pod` - A reference to a `PodInfo` struct containing the pod's details
///
/// # Examples
/// ```rust
/// let pod = PodInfo {
///     name: "my-pod".to_string(),
///     status: "running".to_string(),
///     // ... other fields ...
/// };
/// display_pod_details(&pod);
/// ```
///
/// # Notes
/// - Status colors: green for "running", yellow for "starting", red for "stopped"
/// - Timestamps are displayed in UTC format: YYYY-MM-DD HH:MM:SS UTC
/// - Port mappings are displayed in a nested format if present
/// - SSH command is highlighted in green if available
pub fn display_pod_details(pod: &PodInfo) {
    // TODO: Consider adding more pod details (e.g., resource usage, logs)
    // TODO: Consider adding support for custom color themes
    // TODO: Consider adding support for different timezone displays
    
    println!("{}", format!("Pod Details: {}", pod.name).bold().blue());
    println!("  {}: {}", "HUID".bold(), pod.huid);

    let status_colored = match pod.status.as_str() {
        "running" => pod.status.green(),
        "starting" => pod.status.yellow(),
        "stopped" => pod.status.red(),
        _ => pod.status.normal(),
    };
    println!("  {}: {}", "Status".bold(), status_colored);
    println!("  {}: {}", "ID".bold(), pod.id);

    if let Some(ssh_cmd) = &pod.ssh_cmd {
        println!("  {}: {}", "SSH Command".bold(), ssh_cmd.green());
    }

    if !pod.ports.is_empty() {
        println!("  {}:", "Port Mappings".bold());
        for (service, port) in &pod.ports {
            println!("    {}: {}", service, port);
        }
    }

    if let Some(created_at) = pod.created_at {
        println!(
            "  {}: {}",
            "Created".bold(),
            created_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    if let Some(updated_at) = pod.updated_at {
        println!(
            "  {}: {}",
            "Updated".bold(),
            updated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    println!();
}

/// Displays a formatted table of available templates with their details.
///
/// This function creates a comprehensive table showing template information including:
/// - Index number for easy reference
/// - Template ID
/// - Template name
/// - Docker image with tag (if available)
/// - Current status
/// - Truncated description (if longer than 40 characters)
///
/// The table is formatted with proper spacing and alignment, and includes
/// a header section with a title. If no templates are available, it displays
/// a yellow warning message.
///
/// # Arguments
/// * `templates` - A slice of `TemplateInfo` structs containing template details
///
/// # Examples
/// ```rust
/// let templates = vec![TemplateInfo::default()];
/// display_templates_table(&templates);
/// ```
///
/// # Notes
/// - Descriptions longer than 40 characters are truncated with "..."
/// - Docker image tags are appended with a colon if present
/// - Status defaults to "Unknown" if not specified
/// - Empty template list shows a warning message
pub fn display_templates_table(templates: &[TemplateInfo]) {
    // TODO: Consider adding more template details (e.g., creation date, usage stats)
    // TODO: Consider making the description length configurable
    // TODO: Consider adding sorting options
    
    if templates.is_empty() {
        println!("{}", "No templates found.".yellow());
        return;
    }

    println!("{}", "Available Templates".bold().blue());
    println!();

    let mut table = Table::new(vec![
        "Index".to_string(),
        "ID".to_string(),
        "Name".to_string(),
        "Docker Image".to_string(),
        "Status".to_string(),
        "Description".to_string(),
    ]);

    for (i, template) in templates.iter().enumerate() {
        let index = (i + 1).to_string();
        let image = if let Some(tag) = &template.docker_image_tag {
            format!("{}:{}", template.docker_image, tag)
        } else {
            template.docker_image.clone()
        };

        let status = template
            .status
            .as_ref()
            .unwrap_or(&"Unknown".to_string())
            .clone();

        let description = template
            .description
            .as_ref()
            .map(|d| {
                if d.len() > 40 {
                    format!("{}...", &d[..37])
                } else {
                    d.clone()
                }
            })
            .unwrap_or_else(|| "No description".to_string());

        table.add_row(vec![
            index,
            template.id.clone(),
            template.name.clone(),
            image,
            status,
            description,
        ]);
    }

    table.print();
}

/// Interactive command-line prompts for user input and feedback.
/// 
/// This module provides a set of functions for handling interactive user input
/// and displaying status messages in the terminal. It uses the `dialoguer` crate
/// for interactive prompts and `colored` for styled output.
/// 
/// # Examples
/// ```rust
/// // Confirm an action
/// let confirmed = prompt_confirm("Delete this file?", false)?;
/// 
/// // Select from a list
/// let options = vec!["Option 1", "Option 2", "Option 3"];
/// let selection = prompt_select("Choose an option:", &options)?;
/// 
/// // Get user input
/// let name = prompt_input("Enter your name:", Some("John"))?;
/// 
/// // Display status messages
/// print_success("Operation completed successfully");
/// print_error("An error occurred");
/// print_warning("This action cannot be undone");
/// print_info("Processing your request");
/// ```

/// Prompts the user for a yes/no confirmation with an optional default value.
/// 
/// This function creates an interactive confirmation prompt using the dialoguer crate.
/// It handles errors gracefully and returns a Result containing the user's choice.
/// 
/// # Arguments
/// * `message` - The prompt message to display to the user
/// * `default` - The default value if the user just presses Enter
/// 
/// # Returns
/// * `Result<bool>` - Ok(true) if confirmed, Ok(false) if denied, Err if input fails
/// 
/// # Examples
/// ```rust
/// let confirmed = prompt_confirm("Are you sure?", false)?;
/// if confirmed {
///     // Proceed with action
/// }
/// ```
pub fn prompt_confirm(message: &str, default: bool) -> Result<bool> {
    let result = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(message)
        .default(default)
        .interact()
        .map_err(|e| crate::CliError::OperationFailed(e.to_string()))?;

    Ok(result)
}

/// Prompts the user to select an item from a list of options.
/// 
/// This function creates an interactive selection prompt using the dialoguer crate.
/// It converts all items to strings and allows the user to navigate and select
/// using arrow keys and Enter.
/// 
/// # Arguments
/// * `message` - The prompt message to display to the user
/// * `items` - A slice of items that can be converted to strings
/// 
/// # Returns
/// * `Result<usize>` - Ok(index) of the selected item, Err if input fails
/// 
/// # Examples
/// ```rust
/// let options = vec!["Option 1", "Option 2", "Option 3"];
/// let selection = prompt_select("Choose an option:", &options)?;
/// println!("Selected: {}", options[selection]);
/// ```
pub fn prompt_select<T: ToString>(message: &str, items: &[T]) -> Result<usize> {
    let item_strings: Vec<String> = items.iter().map(|item| item.to_string()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(message)
        .items(&item_strings)
        .default(0)
        .interact()
        .map_err(|e| crate::CliError::OperationFailed(e.to_string()))?;

    Ok(selection)
}

/// Prompts the user for text input with an optional default value.
/// 
/// This function creates an interactive text input prompt using the dialoguer crate.
/// It supports an optional default value that will be used if the user just presses Enter.
/// 
/// # Arguments
/// * `message` - The prompt message to display to the user
/// * `default` - Optional default value to use if the user just presses Enter
/// 
/// # Returns
/// * `Result<String>` - Ok(input) containing the user's input, Err if input fails
/// 
/// # Examples
/// ```rust
/// let name = prompt_input("Enter your name:", Some("John"))?;
/// println!("Hello, {}!", name);
/// ```
pub fn prompt_input(message: &str, default: Option<&str>) -> Result<String> {
    let theme = ColorfulTheme::default();
    let mut input = Input::with_theme(&theme).with_prompt(message);

    if let Some(default_val) = default {
        input = input.default(default_val.to_string());
    }

    let result = input
        .interact_text()
        .map_err(|e| crate::CliError::OperationFailed(e.to_string()))?;

    Ok(result)
}

/// Status message display functions for consistent user feedback.
/// 
/// These functions provide a standardized way to display different types of status
/// messages with appropriate colors and icons. They use the colored crate for
/// terminal styling.

/// Displays a success message with a green checkmark icon.
/// 
/// # Arguments
/// * `message` - The message to display
/// 
/// # Examples
/// ```rust
/// print_success("Operation completed successfully");
/// ```
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Displays an error message with a red X icon.
/// 
/// # Arguments
/// * `message` - The error message to display
/// 
/// # Examples
/// ```rust
/// print_error("Failed to connect to server");
/// ```
pub fn print_error(message: &str) {
    println!("{} {}", "✗".red().bold(), message);
}

/// Displays a warning message with a yellow warning icon.
/// 
/// # Arguments
/// * `message` - The warning message to display
/// 
/// # Examples
/// ```rust
/// print_warning("This action cannot be undone");
/// ```
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

/// Displays an informational message with a blue info icon.
/// 
/// # Arguments
/// * `message` - The informational message to display
/// 
/// # Examples
/// ```rust
/// print_info("Processing your request");
/// ```
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Displays a spinning progress indicator with a message.
/// 
/// This function initiates a visual spinner animation to indicate ongoing operations.
/// The spinner uses a blue "⠋" character that can be animated by subsequent calls.
/// The message is displayed next to the spinner, followed by an ellipsis.
/// 
/// # Arguments
/// * `message` - The message to display alongside the spinner
/// 
/// # Examples
/// ```rust
/// print_spinner_start("Loading data");
/// // ... perform operation ...
/// print_spinner_stop();
/// ```
pub fn print_spinner_start(message: &str) {
    print!("{} {}...", "⠋".blue().bold(), message);
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}

/// Stops the spinner animation and displays a completion message.
/// 
/// This function completes the spinner animation by printing a newline
/// and a green "Done" message. It should be called after the operation
/// that was being monitored is complete.
/// 
/// # Examples
/// ```rust
/// print_spinner_start("Loading data");
/// // ... perform operation ...
/// print_spinner_stop();
/// ```
pub fn print_spinner_stop() {
    println!(" {}", "Done".green());
}

/// Displays a compact overview of executor information.
/// 
/// This function presents a concise, single-line summary for each executor,
/// including:
/// - Availability status (🟢 for available, 🔴 for rented)
/// - Hardware Unique Identifier (HUID)
/// - GPU type and count
/// - Price per GPU per hour
/// - Geographic region
/// 
/// The output is formatted for quick scanning and comparison of multiple executors.
/// 
/// # Arguments
/// * `executors` - A slice of `ExecutorInfo` structs to display
/// 
/// # Examples
/// ```rust
/// let executors = vec![ExecutorInfo::default()];
/// display_executors_compact(&executors);
/// ```
pub fn display_executors_compact(executors: &[ExecutorInfo]) {
    if executors.is_empty() {
        println!("{}", "No executors found.".yellow());
        return;
    }

    println!("{}", "Executors (Compact View)".bold().blue());
    for executor in executors {
        let status_icon = if executor.available { "🟢" } else { "🔴" };
        let price = format!("${:.3}/GPU/hr", executor.price_per_gpu_hour);
        println!(
            "{} {} {} - {} ({}x {}) - {}",
            status_icon,
            executor.huid,
            executor.gpu_type,
            price,
            executor.gpu_count,
            executor.gpu_type,
            executor
                .location
                .get("region")
                .unwrap_or(&"Unknown".to_string())
        );
    }
}

/// Displays detailed information about each executor in a structured format.
/// 
/// This function presents comprehensive information about each executor in a
/// hierarchical, easy-to-read format. For each executor, it shows:
/// - Index number and HUID
/// - GPU configuration (count and type)
/// - Pricing details (total and per-GPU rates)
/// - Availability status (color-coded)
/// - Location information (if available)
/// - Technical specifications (if available)
/// 
/// The output is formatted with proper indentation and color coding for
/// better readability and quick status assessment.
/// 
/// # Arguments
/// * `executors` - A slice of `ExecutorInfo` structs to display
/// 
/// # Examples
/// ```rust
/// let executors = vec![ExecutorInfo::default()];
/// display_executors_detailed(&executors);
/// ```
pub fn display_executors_detailed(executors: &[ExecutorInfo]) {
    if executors.is_empty() {
        println!("{}", "No executors found.".yellow());
        return;
    }

    println!("{}", "Executors (Detailed View)".bold().blue());
    println!();

    for (i, executor) in executors.iter().enumerate() {
        println!("{}. {} ({})", i + 1, executor.huid.bold(), executor.id);
        println!("   GPU: {}x {}", executor.gpu_count, executor.gpu_type);
        println!(
            "   Price: ${:.3}/hr (${:.3}/GPU/hr)",
            executor.price_per_hour, executor.price_per_gpu_hour
        );

        let status_colored = if executor.available {
            "Available".green()
        } else {
            "Rented".red()
        };
        println!("   Status: {}", status_colored);

        if !executor.location.is_empty() {
            println!(
                "   Location: {}",
                executor
                    .location
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if executor.specs != serde_json::Value::Null {
            println!("   Specs: {}", executor.specs);
        }

        println!();
    }
}

// TODO: Add progress bars for long operations
// TODO: Add more sophisticated table formatting options
// TODO: Add theme support for different color schemes
// TODO: Add export options (JSON, CSV)
// TODO: Add pagination for large tables
